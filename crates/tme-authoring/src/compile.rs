//! Semantic validation of one authored member.
//!
//! There is exactly one implementation of member semantics in this crate.
//! [`crate::promotion::load`] reaches it through the promotion gate and
//! [`crate::candidate::validate_candidate`] reaches it without one; neither
//! gets a looser copy. A second, gentler validator for candidates is precisely
//! the false-green failure the authoring standard exists to kill.
//!
//! **One member type, whatever the member is.** What a member carries —
//! routes, structures, landmarks, transitions — is declared by its contract,
//! not by which Rust type it compiles into. A land of one member and a land of
//! three run the same code.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde_json::Value;

use crate::Result;
use crate::contract::{self, MemberContract, TileRole};
use crate::tiled::{self, Point};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Structure {
    pub id: String,
    pub purpose: String,
    pub scope: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub access: Point,
    pub facade_door: Point,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Landmark {
    pub id: String,
    pub role: String,
    pub at: Point,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Transition {
    pub id: String,
    pub member: String,
    pub target_member: String,
    pub paired_transition: String,
    pub direction: String,
    pub marker: Point,
    pub access: Point,
}

/// Compiled cells and the passability they imply. Cells are
/// `[row][column][layer]`, bottom layer first, which is the runtime world
/// template's own cell shape.
#[derive(Debug, Clone)]
pub(crate) struct Grid {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) cells: Vec<Vec<Vec<Option<String>>>>,
    pub(crate) passable: BTreeSet<Point>,
}

/// One compiled authored member. Only [`crate::promotion::load`] produces one
/// inside a land, and nothing in this crate converts a candidate report into
/// one.
#[derive(Debug, Clone)]
pub struct Member {
    pub(crate) contract: &'static MemberContract,
    pub(crate) grid: Grid,
    pub(crate) route_cells: BTreeSet<Point>,
    pub(crate) structures: Vec<Structure>,
    pub(crate) landmarks: BTreeMap<String, Landmark>,
    pub(crate) transitions: BTreeMap<String, Transition>,
    pub(crate) arrival: Option<Point>,
    pub(crate) report: MemberReport,
}

/// The compiled member's read-only view.
///
/// This is the logical frame the Workbench renders: the same cells, routes,
/// structures and landmarks the projection was built from, so a logical
/// address is by construction an address in the authoritative frame. Every
/// accessor borrows; nothing here hands out a way to build a `Member`.
impl Member {
    pub fn id(&self) -> &'static str {
        self.contract.id
    }

    pub fn width(&self) -> usize {
        self.grid.width
    }

    pub fn height(&self) -> usize {
        self.grid.height
    }

    pub fn cells(&self) -> &[Vec<Vec<Option<String>>>] {
        &self.grid.cells
    }

    pub fn is_passable(&self, at: Point) -> bool {
        self.grid.passable.contains(&at)
    }

    pub fn route_cells(&self) -> &BTreeSet<Point> {
        &self.route_cells
    }

    pub fn structures(&self) -> &[Structure] {
        &self.structures
    }

    pub fn landmarks(&self) -> &BTreeMap<String, Landmark> {
        &self.landmarks
    }

    pub fn transitions(&self) -> &BTreeMap<String, Transition> {
        &self.transitions
    }

    pub fn arrival(&self) -> Option<Point> {
        self.arrival
    }

    pub fn report(&self) -> &MemberReport {
        &self.report
    }
}

/// Everything a compiled member reports about itself. A member that carries no
/// routes, structures, or landmarks reports zero of them rather than reporting
/// through a second, narrower shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MemberReport {
    pub member: String,
    pub width: usize,
    pub height: usize,
    pub passable_cells: usize,
    pub route_cells: usize,
    pub structure_footprint_cells: usize,
    pub structures: Vec<String>,
    pub landmarks: Vec<String>,
    pub transitions: Vec<String>,
    pub arrival: Option<Point>,
}

/// A member's tile layers, already vocabulary-checked, keyed by layer name.
struct TileLayers {
    data: BTreeMap<&'static str, Vec<u32>>,
    classes: &'static [contract::TileClass],
    width: usize,
    height: usize,
}

impl TileLayers {
    fn layer(&self, name: &str) -> &[u32] {
        self.data
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[] as &[u32])
    }

    fn gid(&self, name: &str, index: usize) -> u32 {
        self.layer(name).get(index).copied().unwrap_or(0)
    }

    fn class(&self, gid: u32) -> &'static contract::TileClass {
        &self.classes[(gid - 1) as usize]
    }
}

pub fn compile_member(contract: &'static MemberContract, document: &Value) -> Result<Member> {
    let layers = read_member(document, contract)?;
    let structures = read_structures(document, contract, &layers)?;
    let footprints = footprint_cells(&structures);
    let authored_footprints = marked_cells(&layers, "structure_footprints");
    if authored_footprints != footprints {
        return Err(
            "structure objects and the structure_footprints layer describe different cells".into(),
        );
    }
    let route_cells = marked_cells(&layers, "routes");
    let grid = build_grid(&layers)?;

    let landmarks = read_landmarks(document, contract, &layers, &grid)?;
    let transitions = read_transitions(document, contract, &layers, &grid)?;

    let arrival = landmarks
        .values()
        .find(|landmark| landmark.role == contract::ARRIVAL_ROLE)
        .map(|landmark| landmark.at);
    if let Some(arrival) = arrival
        && !route_cells.contains(&arrival)
    {
        return Err("the arrival landmark must stand on an authored route cell".into());
    }

    // Every walkable cell must be reachable from the member's own way in: its
    // arrival if it has one, otherwise the first transition that reaches it. A
    // member with neither is a member nobody can enter.
    let root = match arrival.or_else(|| transitions.values().next().map(|entry| entry.access)) {
        Some(root) => root,
        None => {
            return Err(format!(
                "member {} declares neither an arrival landmark nor a transition to reach it by",
                contract.id
            ));
        }
    };
    require_full_reachability(&grid, root, contract.id)?;

    for structure in &structures {
        if !grid.passable.contains(&structure.access) {
            return Err(format!("structure {} access cell is blocked", structure.id));
        }
        let nearest = route_cells
            .iter()
            .map(|cell| cell.x.abs_diff(structure.access.x) + cell.y.abs_diff(structure.access.y))
            .min()
            .unwrap_or(usize::MAX);
        if nearest > 1 {
            return Err(format!(
                "structure {} access cell is detached from every authored route",
                structure.id
            ));
        }
        let clustered_ground = contract.clustered_ground_class.ok_or_else(|| {
            format!(
                "member {} authors structures but declares no clustered ground class",
                contract.id
            )
        })?;
        let clustered = structure.scope == "clustered";
        for y in structure.y..structure.y + structure.height {
            for x in structure.x..structure.x + structure.width {
                let on_clustered_ground = grid.cells[y][x]
                    .iter()
                    .flatten()
                    .any(|terrain| terrain == clustered_ground);
                if on_clustered_ground != clustered {
                    return Err(format!(
                        "structure {} is scoped {:?} but its footprint at {},{} disagrees",
                        structure.id, structure.scope, x, y
                    ));
                }
            }
        }
    }

    let report = MemberReport {
        member: contract.id.into(),
        width: grid.width,
        height: grid.height,
        passable_cells: grid.passable.len(),
        route_cells: route_cells.len(),
        structure_footprint_cells: footprints.len(),
        structures: structures.iter().map(|row| row.id.clone()).collect(),
        landmarks: landmarks.keys().cloned().collect(),
        transitions: transitions.keys().cloned().collect(),
        arrival,
    };
    Ok(Member {
        contract,
        grid,
        route_cells,
        structures,
        landmarks,
        transitions,
        arrival,
        report,
    })
}

/// Envelope, map properties, tileset vocabulary, layer set, and per-layer tile
/// vocabulary — the assertions that must hold before any coordinate means
/// anything.
fn read_member(document: &Value, contract: &'static MemberContract) -> Result<TileLayers> {
    let (width, height) = contract.envelope();
    let root = tiled::object(document, "authored map")?;
    for (name, expected) in [
        ("width", width as i64),
        ("height", height as i64),
        ("tilewidth", tiled::TILE),
        ("tileheight", tiled::TILE),
    ] {
        if tiled::integer(root.get(name), &format!("authored map.{name}"))? != expected {
            return Err(format!("authored map.{name} must be {expected}"));
        }
    }
    if root.get("orientation").and_then(Value::as_str) != Some("orthogonal")
        || root.get("infinite").and_then(Value::as_bool) != Some(false)
    {
        return Err("an authored member must be finite and orthogonal".into());
    }

    let declared = tiled::properties(document, "authored map")?;
    let required = contract.declared_properties();
    if declared.len() != required.len() {
        return Err(format!(
            "authored map declares {} properties; the accepted contract declares {}",
            declared.len(),
            required.len()
        ));
    }
    for (name, expected) in required {
        if declared.get(name) != Some(&expected) {
            return Err(format!("accepted map property {name} differs"));
        }
    }

    let classes = contract.classes;
    let authored_classes = tiled::tileset_classes(document)?;
    if authored_classes.len() != classes.len()
        || authored_classes
            .iter()
            .zip(classes)
            .any(|(authored, accepted)| authored != accepted.name)
    {
        return Err("the embedded tile vocabulary differs from the accepted class set".into());
    }

    let layers = tiled::layers_by_name(document)?;
    let expected_layers = contract
        .tile_layers
        .iter()
        .chain(contract.object_layers)
        .copied()
        .collect::<BTreeSet<_>>();
    if layers.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_layers {
        return Err("the authored layer set differs from the accepted contract".into());
    }

    let mut data = BTreeMap::new();
    for name in contract.tile_layers {
        let values = tiled::tile_data(layers[*name], name, width, height)?;
        let empties_allowed = !matches!(*name, "base_terrain" | "passability");
        for (index, gid) in values.iter().enumerate() {
            if *gid == 0 {
                if empties_allowed {
                    continue;
                }
                return Err(format!("layer {name} leaves cell {index} unauthored"));
            }
            let class = classes.get((*gid - 1) as usize).ok_or_else(|| {
                format!("layer {name} uses tile id {gid}, which is outside the class set")
            })?;
            if !layer_admits(name, class.role) {
                return Err(format!(
                    "layer {name} uses class {:?}, which belongs to another layer",
                    class.name
                ));
            }
        }
        data.insert(*name, values);
    }
    Ok(TileLayers {
        data,
        classes,
        width,
        height,
    })
}

fn layer_admits(layer: &str, role: TileRole) -> bool {
    contract::layer_of(role) == layer
}

fn marked_cells(layers: &TileLayers, name: &str) -> BTreeSet<Point> {
    layers
        .layer(name)
        .iter()
        .enumerate()
        .filter_map(|(index, gid)| {
            (*gid != 0).then_some(Point {
                x: index % layers.width,
                y: index / layers.width,
            })
        })
        .collect()
}

/// Project the tile layers into runtime cells and prove the authored
/// passability annotation says exactly what the terrain implies. A stale
/// annotation is the classic declared-versus-painted divergence, so it is a
/// rejection rather than a preference for one side.
fn build_grid(layers: &TileLayers) -> Result<Grid> {
    let mut cells = vec![vec![Vec::new(); layers.width]; layers.height];
    let mut passable = BTreeSet::new();
    for index in 0..layers.width * layers.height {
        let point = Point {
            x: index % layers.width,
            y: index / layers.width,
        };
        let base = layers.class(layers.gid("base_terrain", index));
        let blocked_base = matches!(base.role, TileRole::Base { blocked: true });
        let route = layers.gid("routes", index);
        let footprint = layers.gid("structure_footprints", index);
        let mark = layers.gid("landmark_marks", index);

        // Where a route replaces blocked ground (see `cell_is_passable`), the base
        // terrain does not survive into the stack: a bridge is what the cell
        // is now. Everywhere else the base stays and the route sits on top.
        let layer_stack = &mut cells[point.y][point.x];
        if !blocked_base || route == 0 {
            layer_stack.push(Some(base.name.to_owned()));
        }
        for gid in [route, footprint, mark] {
            if gid != 0 {
                layer_stack.push(Some(layers.class(gid).name.to_owned()));
            }
        }

        let computed = cell_is_passable(blocked_base, route, footprint);
        let annotation = layers.class(layers.gid("passability", index));
        let TileRole::Passability { walkable } = annotation.role else {
            return Err(format!(
                "the passability layer carries class {:?} at {},{}",
                annotation.name, point.x, point.y
            ));
        };
        if computed != walkable {
            return Err(format!(
                "the passability annotation is stale at {},{}",
                point.x, point.y
            ));
        }
        if computed {
            passable.insert(point);
        }
    }
    Ok(Grid {
        width: layers.width,
        height: layers.height,
        cells,
        passable,
    })
}

/// Whether a cell is walkable, from the three tile layers that decide it.
///
/// The one statement of the rule. [`build_grid`] checks the authored
/// annotation against it and [`crate::replay`] refreshes that annotation from
/// it, so an edited document and a compiled one cannot disagree about what
/// "walkable" means — there is nothing to disagree with.
///
/// A route laid across impassable ground REPLACES it: a bridge is what the
/// cell is now, not something floating on deep water. A footprint always wins.
pub(crate) fn cell_is_passable(base_blocked: bool, route: u32, footprint: u32) -> bool {
    (!base_blocked || route != 0) && footprint == 0
}

fn require_full_reachability(grid: &Grid, root: Point, member: &str) -> Result<()> {
    if !grid.passable.contains(&root) {
        return Err(format!("the {member} reachability root is blocked"));
    }
    let mut seen = BTreeSet::from([root]);
    let mut queue = VecDeque::from([root]);
    while let Some(current) = queue.pop_front() {
        for (dx, dy) in [(1_isize, 0_isize), (-1, 0), (0, 1), (0, -1)] {
            let (Some(x), Some(y)) = (
                current.x.checked_add_signed(dx),
                current.y.checked_add_signed(dy),
            ) else {
                continue;
            };
            let next = Point { x, y };
            if grid.passable.contains(&next) && seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    if seen != grid.passable {
        return Err(format!(
            "the {member} has {} walkable cells no one can reach",
            grid.passable.len() - seen.len()
        ));
    }
    Ok(())
}

fn footprint_cells(structures: &[Structure]) -> BTreeSet<Point> {
    structures
        .iter()
        .flat_map(|structure| {
            (structure.y..structure.y + structure.height).flat_map(move |y| {
                (structure.x..structure.x + structure.width).map(move |x| Point { x, y })
            })
        })
        .collect()
}

fn read_structures(
    document: &Value,
    contract: &'static MemberContract,
    layers: &TileLayers,
) -> Result<Vec<Structure>> {
    let program = contract
        .structures
        .iter()
        .map(|entry| (entry.id, entry.scope))
        .collect::<BTreeMap<_, _>>();
    let objects = match tiled::layers_by_name(document)?.get("structures") {
        Some(layer) => tiled::named_objects(layer, "structures")?,
        None => BTreeMap::new(),
    };
    if objects.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != program.keys().copied().collect()
    {
        return Err("the structure program differs from the accepted contract".into());
    }

    let mut structures = Vec::new();
    let mut claimed: BTreeSet<Point> = BTreeSet::new();
    for (id, value) in &objects {
        let label = format!("structure {id}");
        let row = tiled::object(value, &label)?;
        if row.get("class").and_then(Value::as_str) != Some(contract::STRUCTURE_CLASS) {
            return Err(format!("{label} must be a {}", contract::STRUCTURE_CLASS));
        }
        let origin = tiled::point(value, layers.width, layers.height, &label)?;
        let width_px = tiled::integer(row.get("width"), &format!("{label}.width"))?;
        let height_px = tiled::integer(row.get("height"), &format!("{label}.height"))?;
        if width_px <= 0
            || height_px <= 0
            || width_px % tiled::TILE != 0
            || height_px % tiled::TILE != 0
        {
            return Err(format!(
                "{label} dimensions must be positive {}px cell multiples",
                tiled::TILE
            ));
        }
        let width = (width_px / tiled::TILE) as usize;
        let height = (height_px / tiled::TILE) as usize;
        if origin.x + width > layers.width || origin.y + height > layers.height {
            return Err(format!("{label} footprint leaves the envelope"));
        }
        let props = tiled::properties(value, &label)?;
        if !tiled::property_bool(&props, "occupied", &label)? {
            return Err(format!("{label} must be occupied"));
        }
        let scope = tiled::property_string(&props, "scope", &label)?;
        if program.get(id.as_str()) != Some(&scope.as_str()) {
            return Err(format!("{label} scope differs from the accepted contract"));
        }
        let access = Point {
            x: tiled::property_int(&props, "access_cell_x", &label)?,
            y: tiled::property_int(&props, "access_cell_y", &label)?,
        };
        if access.x >= layers.width || access.y >= layers.height {
            return Err(format!("{label} access cell leaves the envelope"));
        }
        for y in origin.y..origin.y + height {
            for x in origin.x..origin.x + width {
                if !claimed.insert(Point { x, y }) {
                    return Err(format!("{label} overlaps another footprint"));
                }
            }
        }
        structures.push(Structure {
            id: id.clone(),
            purpose: tiled::property_string(&props, "purpose", &label)?,
            scope,
            x: origin.x,
            y: origin.y,
            width,
            height,
            access,
            facade_door: facade_door(origin, width, height, access, &label)?,
        });
    }
    structures.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(structures)
}

/// The one footprint cell an access cell opens into. Zero means the access
/// cell is not against the building; more than one means the door is
/// ambiguous, and an ambiguous door is a door that drifts.
fn facade_door(
    origin: Point,
    width: usize,
    height: usize,
    access: Point,
    label: &str,
) -> Result<Point> {
    let matches = [
        (access.x.checked_sub(1), Some(access.y)),
        (access.x.checked_add(1), Some(access.y)),
        (Some(access.x), access.y.checked_sub(1)),
        (Some(access.x), access.y.checked_add(1)),
    ]
    .into_iter()
    .filter_map(|(x, y)| Some(Point { x: x?, y: y? }))
    .filter(|point| {
        point.x >= origin.x
            && point.x < origin.x + width
            && point.y >= origin.y
            && point.y < origin.y + height
    })
    .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "{label} access cell must touch exactly one footprint cell, not {}",
            matches.len()
        ));
    }
    Ok(matches[0])
}

fn read_landmarks(
    document: &Value,
    contract: &'static MemberContract,
    layers: &TileLayers,
    grid: &Grid,
) -> Result<BTreeMap<String, Landmark>> {
    let program = contract
        .landmarks
        .iter()
        .map(|entry| (entry.id, (entry.role, entry.marker_class)))
        .collect::<BTreeMap<_, _>>();
    let objects = match tiled::layers_by_name(document)?.get("landmarks") {
        Some(layer) => tiled::named_objects(layer, "landmarks")?,
        None => BTreeMap::new(),
    };
    if objects.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != program.keys().copied().collect()
    {
        return Err("the landmark program differs from the accepted contract".into());
    }
    let mut landmarks = BTreeMap::new();
    for (id, value) in &objects {
        let label = format!("landmark {id}");
        let row = tiled::object(value, &label)?;
        if row.get("class").and_then(Value::as_str) != Some(contract::LANDMARK_CLASS) {
            return Err(format!("{label} must be a {}", contract::LANDMARK_CLASS));
        }
        let at = tiled::point(value, layers.width, layers.height, &label)?;
        let props = tiled::properties(value, &label)?;
        let role = tiled::property_string(&props, "role", &label)?;
        let (expected_role, marker) = program[id.as_str()];
        if role != expected_role {
            return Err(format!("{label} role differs from the accepted contract"));
        }
        if !grid.passable.contains(&at) {
            return Err(format!("{label} stands on a blocked cell"));
        }
        require_marker(grid, at, marker, &label)?;
        landmarks.insert(
            id.clone(),
            Landmark {
                id: id.clone(),
                role,
                at,
            },
        );
    }
    Ok(landmarks)
}

fn read_transitions(
    document: &Value,
    contract: &'static MemberContract,
    layers: &TileLayers,
    grid: &Grid,
) -> Result<BTreeMap<String, Transition>> {
    let program = contract
        .transitions
        .iter()
        .map(|entry| {
            (
                entry.id,
                (
                    entry.target_member,
                    entry.paired_transition,
                    entry.direction,
                    entry.marker_class,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let objects = match tiled::layers_by_name(document)?.get("transitions") {
        Some(layer) => tiled::named_objects(layer, "transitions")?,
        None => BTreeMap::new(),
    };
    if objects.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != program.keys().copied().collect()
    {
        return Err(format!(
            "the {} transition program differs from the accepted contract",
            contract.id
        ));
    }
    let mut transitions = BTreeMap::new();
    for (id, value) in &objects {
        let label = format!("transition {id}");
        let row = tiled::object(value, &label)?;
        if row.get("class").and_then(Value::as_str) != Some(contract::TRANSITION_CLASS) {
            return Err(format!("{label} must be a {}", contract::TRANSITION_CLASS));
        }
        let marker = tiled::point(value, layers.width, layers.height, &label)?;
        let props = tiled::properties(value, &label)?;
        let (target, pair, direction, marker_class) = program[id.as_str()];
        if tiled::property_string(&props, "target_member", &label)? != target
            || tiled::property_string(&props, "paired_transition", &label)? != pair
            || tiled::property_string(&props, "direction", &label)? != direction
        {
            return Err(format!("{label} differs from the accepted contract"));
        }
        let access = Point {
            x: tiled::property_int(&props, "access_cell_x", &label)?,
            y: tiled::property_int(&props, "access_cell_y", &label)?,
        };
        if access.x >= layers.width || access.y >= layers.height {
            return Err(format!("{label} access cell leaves the envelope"));
        }
        if marker.x.abs_diff(access.x) + marker.y.abs_diff(access.y) != 1 {
            return Err(format!(
                "{label} access cell must be cardinally adjacent to its marker"
            ));
        }
        if !grid.passable.contains(&marker) || !grid.passable.contains(&access) {
            return Err(format!("{label} marker or access cell is blocked"));
        }
        require_marker(grid, marker, marker_class, &label)?;
        transitions.insert(
            id.clone(),
            Transition {
                id: id.clone(),
                member: contract.id.into(),
                target_member: target.into(),
                paired_transition: pair.into(),
                direction: direction.into(),
                marker,
                access,
            },
        );
    }
    Ok(transitions)
}

/// An authored feature and its painted tile must agree. This is the half of
/// declared-versus-painted equality that object metadata cannot check on its
/// own: the object says where the feature is, the grid says what is drawn
/// there, and a feature nobody painted is an authoring defect.
fn require_marker(grid: &Grid, at: Point, marker: &str, label: &str) -> Result<()> {
    if marker.is_empty() {
        return Ok(());
    }
    if grid.cells[at.y][at.x]
        .iter()
        .flatten()
        .any(|terrain| terrain == marker)
    {
        return Ok(());
    }
    Err(format!("{label} is not painted with its {marker} marker"))
}
