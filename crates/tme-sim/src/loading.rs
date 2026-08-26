use std::collections::{HashMap, HashSet};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;
use serde_path_to_error::Segment;
use sha2::{Digest, Sha256};
use tme_rules::content::{
    CatalogProfileKey, CatalogV6, ResearchBoundary, WorldTemplateV3, boundary_policy,
    scan_raw_documents,
};
use tme_rules::{GameDefinition, ValidatedWorldSeed};

use crate::fixture::script::{validate_script_references, validate_script_shape};
use crate::fixture::{
    SIMULATION_SCENARIO_KIND, SIMULATION_SCENARIO_SCHEMA_VERSION, SIMULATION_SEED_KIND,
    SIMULATION_SEED_SCHEMA_VERSION, SimulationScenarioV1, SimulationSeedV3,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticComponent {
    Scenario,
    Catalog,
    WorldTemplate,
    SimulationSeed,
    Script,
    Bundle,
}

impl DiagnosticComponent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scenario => "scenario",
            Self::Catalog => "catalog",
            Self::WorldTemplate => "world_template",
            Self::SimulationSeed => "simulation_seed",
            Self::Script => "script",
            Self::Bundle => "bundle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentDiagnostic {
    pub component: DiagnosticComponent,
    pub pointer: String,
    pub message: String,
}

impl ContentDiagnostic {
    fn new(
        component: DiagnosticComponent,
        pointer: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            component,
            pointer: pointer.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub diagnostics: Vec<ContentDiagnostic>,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}{}: {}",
                    diagnostic.component.as_str(),
                    diagnostic.pointer,
                    diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        formatter.write_str(&message)
    }
}

impl std::error::Error for LoadError {}

#[derive(Debug, Clone)]
pub(crate) struct LoadedSimulation {
    pub(crate) scenario: SimulationScenarioV1,
    pub(crate) world_seed: ValidatedWorldSeed,
    pub(crate) scenario_loaded_event: tme_rules::Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DefinitionCacheKey {
    catalog_digest: [u8; 32],
    world_template_digest: [u8; 32],
    catalog_profile: String,
}

#[derive(Debug, Clone)]
struct CachedDefinition {
    definition: Arc<GameDefinition>,
    realms: Vec<String>,
    levels: Vec<tme_rules::WorldSite>,
}

#[derive(Debug, Default)]
pub(crate) struct ValidationBatchContext {
    definitions: HashMap<DefinitionCacheKey, CachedDefinition>,
    cache_hits: usize,
    definition_compiles: usize,
}

impl ValidationBatchContext {
    #[cfg(test)]
    pub(crate) fn cache_hits(&self) -> usize {
        self.cache_hits
    }

    #[cfg(test)]
    pub(crate) fn definition_compiles(&self) -> usize {
        self.definition_compiles
    }
}

struct JsonDocument {
    value: Value,
    digest: [u8; 32],
}

#[derive(Debug, Deserialize)]
struct MinimumScenarioEnvelope {
    clean_content: bool,
    research_boundary: ResearchBoundary,
    catalog: String,
    catalog_profile: String,
    world_template: String,
    simulation_seed: String,
}

#[derive(Debug, Deserialize)]
struct MinimumCatalogBoundary {
    clean_content: bool,
    research_boundary: ResearchBoundary,
}

pub(crate) fn load_simulation(path: &Path) -> Result<LoadedSimulation, LoadError> {
    load_simulation_with_context(path, &mut ValidationBatchContext::default())
}

pub(crate) fn load_simulation_with_context(
    path: &Path,
    context: &mut ValidationBatchContext,
) -> Result<LoadedSimulation, LoadError> {
    let scenario_target = canonical_regular_file(path, DiagnosticComponent::Scenario, "")?;
    let root = scenario_target.parent().ok_or_else(|| LoadError {
        diagnostics: vec![ContentDiagnostic::new(
            DiagnosticComponent::Scenario,
            "",
            "canonical scenario target has no parent directory",
        )],
    })?;
    let scenario_raw = read_json(&scenario_target, DiagnosticComponent::Scenario)?;

    // This read locates the graph and establishes its boundary policy. It is
    // deliberately not acceptance; strict deserialization happens below.
    let minimum: MinimumScenarioEnvelope = strict_deserialize(
        scenario_raw.clone(),
        DiagnosticComponent::Scenario,
        "Simulation Scenario graph selectors",
    )?;

    let catalog_target = resolve_component(root, &minimum.catalog, "catalog")?;
    let world_template_target = resolve_component(root, &minimum.world_template, "world_template")?;
    let simulation_seed_target =
        resolve_component(root, &minimum.simulation_seed, "simulation_seed")?;
    reject_duplicate_targets([
        ("catalog", &catalog_target),
        ("world_template", &world_template_target),
        ("simulation_seed", &simulation_seed_target),
    ])?;

    let catalog_document = read_json_document(&catalog_target, DiagnosticComponent::Catalog)?;
    let world_template_document =
        read_json_document(&world_template_target, DiagnosticComponent::WorldTemplate)?;
    let catalog_raw = catalog_document.value;
    let world_template_raw = world_template_document.value;
    let simulation_seed_raw =
        read_json(&simulation_seed_target, DiagnosticComponent::SimulationSeed)?;

    let catalog_boundary: MinimumCatalogBoundary = strict_deserialize(
        catalog_raw.clone(),
        DiagnosticComponent::Catalog,
        "Catalog boundary",
    )?;
    let scenario_policy = boundary_policy(minimum.clean_content, &minimum.research_boundary)
        .map_err(|error| validation_error(DiagnosticComponent::Scenario, error))?;
    let catalog_policy = boundary_policy(
        catalog_boundary.clean_content,
        &catalog_boundary.research_boundary,
    )
    .map_err(|error| validation_error(DiagnosticComponent::Catalog, error))?;
    if scenario_policy != catalog_policy {
        return Err(LoadError {
            diagnostics: vec![ContentDiagnostic::new(
                DiagnosticComponent::Bundle,
                "",
                "scenario and catalog clean/marked classifications must agree",
            )],
        });
    }

    let definition_key = DefinitionCacheKey {
        catalog_digest: catalog_document.digest,
        world_template_digest: world_template_document.digest,
        catalog_profile: minimum.catalog_profile.clone(),
    };
    let definition_is_cached = context.definitions.contains_key(&definition_key);

    scan_raw_documents(scenario_policy, [("scenario", &scenario_raw)])
        .map_err(|error| validation_error(DiagnosticComponent::Scenario, error))?;
    if !definition_is_cached {
        scan_raw_documents(scenario_policy, [("catalog", &catalog_raw)])
            .map_err(|error| validation_error(DiagnosticComponent::Catalog, error))?;
        scan_raw_documents(scenario_policy, [("world_template", &world_template_raw)])
            .map_err(|error| validation_error(DiagnosticComponent::WorldTemplate, error))?;
    }
    scan_raw_documents(scenario_policy, [("simulation_seed", &simulation_seed_raw)])
        .map_err(|error| validation_error(DiagnosticComponent::SimulationSeed, error))?;

    let scenario: SimulationScenarioV1 = strict_deserialize(
        scenario_raw,
        DiagnosticComponent::Scenario,
        "Simulation Scenario 1",
    )?;
    if scenario.schema_version != SIMULATION_SCENARIO_SCHEMA_VERSION {
        return Err(message_error(
            DiagnosticComponent::Scenario,
            "/schema_version",
            format!("schema_version must be {SIMULATION_SCENARIO_SCHEMA_VERSION}"),
        ));
    }
    if scenario.kind != SIMULATION_SCENARIO_KIND {
        return Err(message_error(
            DiagnosticComponent::Scenario,
            "/kind",
            format!("kind must be {SIMULATION_SCENARIO_KIND:?}"),
        ));
    }
    if scenario.id.trim().is_empty()
        || scenario.name.trim().is_empty()
        || scenario.description.trim().is_empty()
    {
        return Err(message_error(
            DiagnosticComponent::Scenario,
            "",
            "id, name, and description must be non-empty",
        ));
    }
    // The strict object must contain the same graph selectors used by the
    // minimum read; this guards future accidental divergence in that phase.
    if scenario.catalog != minimum.catalog
        || scenario.catalog_profile != minimum.catalog_profile
        || scenario.world_template != minimum.world_template
        || scenario.simulation_seed != minimum.simulation_seed
    {
        return Err(message_error(
            DiagnosticComponent::Bundle,
            "",
            "strict scenario selectors differ from the minimum graph read",
        ));
    }

    let uncached_definition_inputs = if definition_is_cached {
        None
    } else {
        let catalog: CatalogV6 =
            strict_deserialize(catalog_raw, DiagnosticComponent::Catalog, "Catalog 6")?;
        let world_template: WorldTemplateV3 = strict_deserialize(
            world_template_raw,
            DiagnosticComponent::WorldTemplate,
            "World Template 3",
        )?;
        Some((catalog, world_template))
    };
    let simulation_seed: SimulationSeedV3 = strict_deserialize(
        simulation_seed_raw,
        DiagnosticComponent::SimulationSeed,
        "Simulation Seed 3",
    )?;
    if simulation_seed.schema_version != SIMULATION_SEED_SCHEMA_VERSION {
        return Err(message_error(
            DiagnosticComponent::SimulationSeed,
            "/schema_version",
            format!("schema_version must be {SIMULATION_SEED_SCHEMA_VERSION}"),
        ));
    }
    if simulation_seed.kind != SIMULATION_SEED_KIND {
        return Err(message_error(
            DiagnosticComponent::SimulationSeed,
            "/kind",
            format!("kind must be {SIMULATION_SEED_KIND:?}"),
        ));
    }
    if simulation_seed.id.trim().is_empty() {
        return Err(message_error(
            DiagnosticComponent::SimulationSeed,
            "/id",
            "id must be non-empty",
        ));
    }

    let profile = CatalogProfileKey::new(scenario.catalog_profile.clone()).map_err(|error| {
        message_error(
            DiagnosticComponent::Scenario,
            "/catalog_profile",
            error.to_string(),
        )
    })?;
    let cached = if let Some(cached) = context.definitions.get(&definition_key).cloned() {
        context.cache_hits += 1;
        cached
    } else {
        let (catalog, world_template) = uncached_definition_inputs
            .expect("an uncached definition must retain parsed definition inputs");
        let realms = world_template.realms.keys().cloned().collect();
        let levels = world_template
            .realms
            .iter()
            .flat_map(|(realm_id, realm)| {
                realm
                    .levels
                    .keys()
                    .map(move |level_id| tme_rules::WorldSite::new(realm_id, level_id))
            })
            .collect();
        let definition = GameDefinition::from_content(catalog, profile, world_template)
            .map_err(|error| validation_error(DiagnosticComponent::Bundle, error))?;
        let cached = CachedDefinition {
            definition,
            realms,
            levels,
        };
        context.definition_compiles += 1;
        context.definitions.insert(definition_key, cached.clone());
        cached
    };
    let scenario_loaded_event = tme_rules::Event::ScenarioLoaded {
        id: scenario.id.clone(),
        name: scenario.name.clone(),
        realms: cached.realms,
        levels: cached.levels,
    };
    let world_seed = ValidatedWorldSeed::new(cached.definition, simulation_seed.into_world_seed())
        .map_err(|error| validation_error(DiagnosticComponent::SimulationSeed, error))?;
    let controlled_actor_ids = world_seed.controlled_actor_ids();
    if controlled_actor_ids.len() != 1 {
        return Err(message_error(
            DiagnosticComponent::SimulationSeed,
            "/actors",
            format!(
                "Simulation Seed 3 requires exactly one player actor, found {}",
                controlled_actor_ids.len()
            ),
        ));
    }

    let shape_errors = validate_script_shape(&scenario.script);
    if !shape_errors.is_empty() {
        return Err(LoadError {
            diagnostics: shape_errors
                .into_iter()
                .map(|(pointer, message)| {
                    ContentDiagnostic::new(DiagnosticComponent::Script, pointer, message)
                })
                .collect(),
        });
    }
    let reference_errors = validate_script_references(&scenario.script, &world_seed);
    if !reference_errors.is_empty() {
        return Err(LoadError {
            diagnostics: reference_errors
                .into_iter()
                .map(|(pointer, message)| {
                    ContentDiagnostic::new(DiagnosticComponent::Script, pointer, message)
                })
                .collect(),
        });
    }

    Ok(LoadedSimulation {
        scenario,
        world_seed,
        scenario_loaded_event,
    })
}

fn strict_deserialize<T: serde::de::DeserializeOwned>(
    value: Value,
    component: DiagnosticComponent,
    contract: &str,
) -> Result<T, LoadError> {
    let encoded = serde_json::to_string(&value).expect("JSON Value must serialize");
    let mut deserializer = serde_json::Deserializer::from_str(&encoded);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let inner_message = stable_serde_message(error.inner());
        let pointer = serde_error_pointer(&value, error.path(), &inner_message);
        message_error(
            component,
            pointer,
            format!("invalid {contract}: {inner_message}"),
        )
    })
}

fn stable_serde_message(error: &serde_json::Error) -> String {
    error
        .to_string()
        .split_once(" at line ")
        .map_or_else(|| error.to_string(), |(message, _)| message.to_string())
}

fn serde_error_pointer(value: &Value, path: &serde_path_to_error::Path, message: &str) -> String {
    let mut cursor = value;
    let mut segments = Vec::new();
    for segment in path {
        match segment {
            Segment::Map { key } => {
                segments.push(key.clone());
                if let Some(next) = cursor.as_object().and_then(|object| object.get(key)) {
                    cursor = next;
                }
            }
            Segment::Seq { index } => {
                segments.push(index.to_string());
                if let Some(next) = cursor.as_array().and_then(|array| array.get(*index)) {
                    cursor = next;
                }
            }
            Segment::Enum { variant } => {
                if let Some(next) = cursor.as_object().and_then(|object| object.get(variant)) {
                    segments.push(variant.clone());
                    cursor = next;
                }
            }
            Segment::Unknown => {}
        }
    }
    if let Some(field) = serde_error_field(message)
        && segments.last().is_none_or(|last| last != field)
    {
        segments.push(field.to_string());
    }
    pointer_from_segments(&segments)
}

fn serde_error_field(message: &str) -> Option<&str> {
    ["unknown field `", "missing field `"]
        .into_iter()
        .find_map(|prefix| {
            let rest = message.strip_prefix(prefix)?;
            rest.split_once('`').map(|(field, _)| field)
        })
}

fn read_json(path: &Path, component: DiagnosticComponent) -> Result<Value, LoadError> {
    read_json_document(path, component).map(|document| document.value)
}

fn read_json_document(
    path: &Path,
    component: DiagnosticComponent,
) -> Result<JsonDocument, LoadError> {
    let input = std::fs::read(path)
        .map_err(|error| message_error(component, "", format!("{}: {error}", path.display())))?;
    let value = serde_json::from_slice(&input).map_err(|error| {
        message_error(
            component,
            "",
            format!("{}: invalid JSON: {error}", path.display()),
        )
    })?;
    Ok(JsonDocument {
        value,
        digest: Sha256::digest(&input).into(),
    })
}

fn resolve_component(root: &Path, reference: &str, field: &str) -> Result<PathBuf, LoadError> {
    validate_reference(reference).map_err(|message| {
        message_error(DiagnosticComponent::Scenario, format!("/{field}"), message)
    })?;
    let target = canonical_regular_file(
        &root.join(reference),
        DiagnosticComponent::Scenario,
        &format!("/{field}"),
    )?;
    if !target.starts_with(root) {
        return Err(message_error(
            DiagnosticComponent::Scenario,
            format!("/{field}"),
            "component target escapes the canonical scenario root",
        ));
    }
    Ok(target)
}

fn validate_reference(reference: &str) -> Result<(), String> {
    if reference.is_empty() {
        return Err("component reference must be non-empty".to_string());
    }
    if reference.contains('\\') {
        return Err("component reference must use forward slashes".to_string());
    }
    if reference.starts_with('/') {
        return Err("component reference must be relative".to_string());
    }
    let segments = reference.split('/').collect::<Vec<_>>();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err("component reference contains an empty, '.', or '..' segment".to_string());
    }
    if segments
        .first()
        .is_some_and(|segment| segment.len() >= 2 && segment.as_bytes()[1] == b':')
    {
        return Err("component reference must not contain a platform prefix".to_string());
    }
    let path = Path::new(reference);
    if path.components().any(|component| {
        matches!(
            component,
            PathComponent::RootDir | PathComponent::Prefix(_) | PathComponent::ParentDir
        )
    }) {
        return Err("component reference must be a bounded relative path".to_string());
    }
    Ok(())
}

fn canonical_regular_file(
    path: &Path,
    component: DiagnosticComponent,
    pointer: &str,
) -> Result<PathBuf, LoadError> {
    let target = std::fs::canonicalize(path).map_err(|error| {
        message_error(component, pointer, format!("{}: {error}", path.display()))
    })?;
    let metadata = std::fs::metadata(&target).map_err(|error| {
        message_error(component, pointer, format!("{}: {error}", target.display()))
    })?;
    if !metadata.is_file() {
        return Err(message_error(
            component,
            pointer,
            format!("{} is not a regular file", target.display()),
        ));
    }
    Ok(target)
}

fn reject_duplicate_targets<const N: usize>(
    targets: [(&str, &PathBuf); N],
) -> Result<(), LoadError> {
    let mut seen = HashSet::new();
    for (field, target) in targets {
        if !seen.insert(target.clone()) {
            return Err(message_error(
                DiagnosticComponent::Scenario,
                format!("/{field}"),
                "component references must resolve to distinct canonical files",
            ));
        }
    }
    Ok(())
}

fn validation_error(
    component: DiagnosticComponent,
    error: tme_rules::content::ValidationError,
) -> LoadError {
    LoadError {
        diagnostics: error
            .messages()
            .iter()
            .map(|message| validation_diagnostic(component, message))
            .collect(),
    }
}

fn validation_diagnostic(
    fallback_component: DiagnosticComponent,
    message: &str,
) -> ContentDiagnostic {
    if let Some((component, rest)) = explicit_component_prefix(message) {
        if let Some((location, detail)) = rest.split_once(' ')
            && location.starts_with('/')
        {
            return ContentDiagnostic::new(component, location, detail);
        }
        if rest.starts_with('/') {
            return ContentDiagnostic::new(component, rest, message);
        }
        if let Some((pointer, detail)) = dotted_validation_location(rest) {
            return ContentDiagnostic::new(component, pointer, detail);
        }
        return ContentDiagnostic::new(component, "", rest);
    }
    if let Some((pointer, detail)) = dotted_validation_location(message) {
        ContentDiagnostic::new(
            inferred_validation_component(fallback_component, message),
            pointer,
            detail,
        )
    } else {
        ContentDiagnostic::new(fallback_component, "", message)
    }
}

fn inferred_validation_component(
    fallback: DiagnosticComponent,
    message: &str,
) -> DiagnosticComponent {
    if fallback != DiagnosticComponent::Bundle {
        return fallback;
    }
    let first = message
        .split_once(' ')
        .map_or(message, |(location, _)| location)
        .split(['.', '['])
        .next()
        .unwrap_or_default();
    match first {
        "realms" | "world_template" => DiagnosticComponent::WorldTemplate,
        _ => fallback,
    }
}

fn explicit_component_prefix(message: &str) -> Option<(DiagnosticComponent, &str)> {
    [
        (DiagnosticComponent::SimulationSeed, "simulation_seed"),
        (DiagnosticComponent::WorldTemplate, "world_template"),
        (DiagnosticComponent::Scenario, "scenario"),
        (DiagnosticComponent::Catalog, "catalog"),
        (DiagnosticComponent::Script, "script"),
        (DiagnosticComponent::Bundle, "bundle"),
    ]
    .into_iter()
    .find_map(|(component, label)| {
        let rest = message.strip_prefix(label)?;
        if rest.is_empty() || rest.starts_with(['/', '.', ' ']) {
            Some((
                component,
                rest.strip_prefix('.').unwrap_or(rest).trim_start(),
            ))
        } else {
            None
        }
    })
}

fn dotted_validation_location(message: &str) -> Option<(String, &str)> {
    let (location, detail) = message.split_once(' ').unwrap_or((message, ""));
    let location = location.trim_end_matches(':');
    let first = location.split(['.', '[']).next().unwrap_or_default();
    if !matches!(
        first,
        "actors"
            | "banks"
            | "clean_content"
            | "content"
            | "damage_labels"
            | "ground_items"
            | "id"
            | "item_instances"
            | "items"
            | "kind"
            | "locker_vaults"
            | "merchant_inventories"
            | "profession_actions"
            | "profiles"
            | "quests"
            | "realms"
            | "research_boundary"
            | "rules"
            | "rules_profiles"
            | "schema_version"
            | "service_definitions"
            | "service_instances"
            | "skill_catalog"
            | "skill_catalogs"
            | "spells"
            | "storage"
            | "summon_templates"
    ) {
        return None;
    }
    let mut segments = dotted_path_segments(location)?;
    if segments.first().is_some_and(|segment| segment == "content") {
        segments.remove(0);
    }
    Some((pointer_from_segments(&segments), detail.trim_start()))
}

fn dotted_path_segments(path: &str) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '.' => {
                if current.is_empty() {
                    if segments.is_empty() {
                        return None;
                    }
                    continue;
                }
                segments.push(std::mem::take(&mut current));
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
                let mut bracket = String::new();
                let mut quoted = false;
                let mut escaped = false;
                let mut closed = false;
                for next in chars.by_ref() {
                    if escaped {
                        bracket.push(next);
                        escaped = false;
                    } else if next == '\\' && quoted {
                        bracket.push(next);
                        escaped = true;
                    } else if next == '"' {
                        bracket.push(next);
                        quoted = !quoted;
                    } else if next == ']' && !quoted {
                        closed = true;
                        break;
                    } else {
                        bracket.push(next);
                    }
                }
                if !closed || quoted || escaped {
                    return None;
                }
                segments.push(normalize_bracket_segment(&bracket)?);
            }
            _ => current.push(character),
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    (!segments.is_empty()).then_some(segments)
}

fn normalize_bracket_segment(segment: &str) -> Option<String> {
    let trimmed = segment.trim();
    if let Ok(value) = serde_json::from_str::<String>(trimmed) {
        return (!value.is_empty()).then_some(value);
    }
    if trimmed.starts_with('"') || trimmed.ends_with('"') {
        return None;
    }
    for wrapper in ["CatalogRegistryKey(", "CatalogProfileKey("] {
        if let Some(inner) = trimmed
            .strip_prefix(wrapper)
            .and_then(|value| value.strip_suffix(')'))
        {
            let value = serde_json::from_str::<String>(inner).ok()?;
            return (!value.is_empty()).then_some(value);
        }
    }
    trimmed
        .chars()
        .all(|character| character.is_ascii_digit())
        .then(|| trimmed.to_string())
}

fn pointer_from_segments(segments: &[String]) -> String {
    if segments.is_empty() {
        return String::new();
    }
    let mut pointer = String::new();
    for segment in segments {
        pointer.push('/');
        pointer.push_str(&segment.replace('~', "~0").replace('/', "~1"));
    }
    pointer
}

fn message_error(
    component: DiagnosticComponent,
    pointer: impl Into<String>,
    message: impl Into<String>,
) -> LoadError {
    LoadError {
        diagnostics: vec![ContentDiagnostic::new(component, pointer, message)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_GRAPH: AtomicU64 = AtomicU64::new(0);

    struct TempGraph {
        root: PathBuf,
        scenario_path: PathBuf,
    }

    impl TempGraph {
        fn copy_from(name: &str) -> Self {
            let source_root =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/test-corpus");
            let source_scenario = source_root.join(name);
            let scenario: Value =
                serde_json::from_str(&std::fs::read_to_string(&source_scenario).unwrap()).unwrap();
            let unique = NEXT_TEMP_GRAPH.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("tme-sim-loading-{}-{unique}", std::process::id()));
            std::fs::create_dir(&root).unwrap();
            let scenario_path = root.join(name);
            std::fs::copy(&source_scenario, &scenario_path).unwrap();
            for field in ["catalog", "world_template", "simulation_seed"] {
                let reference = scenario[field].as_str().unwrap();
                let destination = root.join(reference);
                std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
                std::fs::copy(source_root.join(reference), destination).unwrap();
            }
            Self {
                root,
                scenario_path,
            }
        }

        fn scenario(&self) -> Value {
            serde_json::from_str(&std::fs::read_to_string(&self.scenario_path).unwrap()).unwrap()
        }

        fn write_scenario(&self, value: &Value) {
            std::fs::write(&self.scenario_path, serde_json::to_vec(value).unwrap()).unwrap();
        }

        fn component_path(&self, field: &str) -> PathBuf {
            self.root.join(self.scenario()[field].as_str().unwrap())
        }

        fn component(&self, field: &str) -> Value {
            serde_json::from_str(&std::fs::read_to_string(self.component_path(field)).unwrap())
                .unwrap()
        }

        fn write_component(&self, field: &str, value: &Value) {
            std::fs::write(
                self.component_path(field),
                serde_json::to_vec(value).unwrap(),
            )
            .unwrap();
        }
    }

    impl Drop for TempGraph {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    #[test]
    fn reference_contract_rejects_aliases_roots_and_platform_spelling() {
        for invalid in [
            "",
            "/catalog.json",
            "catalogs//catalog.json",
            "./catalog.json",
            "catalogs/../catalog.json",
            "catalogs\\catalog.json",
            "C:/catalog.json",
        ] {
            assert!(
                validate_reference(invalid).is_err(),
                "reference {invalid:?} must fail"
            );
        }
        assert!(validate_reference("catalogs/catalog.json").is_ok());
    }

    #[test]
    fn validation_path_parser_preserves_quoted_delimiters_and_rfc6901_escaping() {
        let (pointer, detail) =
            dotted_validation_location(r#"items["a]b/c~d"].name must be non-empty"#)
                .expect("quoted bracket key is a valid diagnostic path");
        assert_eq!(pointer, "/items/a]b~1c~0d/name");
        assert_eq!(detail, "must be non-empty");

        assert!(
            dotted_validation_location(r#"items["a\0b"].name must be non-empty"#).is_none(),
            "unsupported Rust debug escapes must fall back to the component root"
        );
    }

    #[test]
    fn tracked_first_room_loads_through_the_four_document_graph() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-corpus/first_room.json");
        let loaded = load_simulation(&path).unwrap();
        assert_eq!(loaded.scenario.id, "first_room");
        assert_eq!(loaded.scenario.script.len(), 2);
    }

    #[test]
    fn batch_context_reuses_only_exact_successful_definition_bytes() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut context = ValidationBatchContext::default();

        load_simulation_with_context(&graph.scenario_path, &mut context)
            .expect("first load must compile the definition");
        load_simulation_with_context(&graph.scenario_path, &mut context)
            .expect("second exact load must reuse the definition");
        assert_eq!(context.definition_compiles(), 1);
        assert_eq!(context.cache_hits(), 1);

        let catalog_path = graph.component_path("catalog");
        let mut catalog_bytes = std::fs::read(&catalog_path).unwrap();
        catalog_bytes.push(b'\n');
        std::fs::write(&catalog_path, catalog_bytes).unwrap();
        load_simulation_with_context(&graph.scenario_path, &mut context)
            .expect("semantically equal bytes remain valid");
        assert_eq!(
            context.definition_compiles(),
            2,
            "an exact-byte digest miss must compile a separate definition"
        );
        assert_eq!(context.cache_hits(), 1);
    }

    #[test]
    fn batch_context_keeps_seed_validation_fresh_and_never_caches_invalid_definitions() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut context = ValidationBatchContext::default();
        load_simulation_with_context(&graph.scenario_path, &mut context).unwrap();

        let mut seed = graph.component("simulation_seed");
        seed["actors"][0]["location"]["level"] = Value::String("missing_level".to_string());
        graph.write_component("simulation_seed", &seed);
        let error = load_simulation_with_context(&graph.scenario_path, &mut context)
            .expect_err("a cached definition must not bypass fresh seed validation");
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::SimulationSeed
        );
        assert_eq!(context.definition_compiles(), 1);
        assert_eq!(context.cache_hits(), 1);

        let invalid_graph = TempGraph::copy_from("first_room.json");
        let mut catalog = invalid_graph.component("catalog");
        catalog["unknown_cache_poison"] = Value::Bool(true);
        invalid_graph.write_component("catalog", &catalog);
        let mut invalid_context = ValidationBatchContext::default();
        for _ in 0..2 {
            load_simulation_with_context(&invalid_graph.scenario_path, &mut invalid_context)
                .expect_err("invalid definition must fail every time");
        }
        assert_eq!(invalid_context.definition_compiles(), 0);
        assert_eq!(invalid_context.cache_hits(), 0);
        assert!(invalid_context.definitions.is_empty());
    }

    #[test]
    fn definition_cache_identity_includes_the_selected_profile() {
        let catalog_digest = [1; 32];
        let world_template_digest = [2; 32];
        let first = DefinitionCacheKey {
            catalog_digest,
            world_template_digest,
            catalog_profile: "profile/first_room".to_string(),
        };
        let second = DefinitionCacheKey {
            catalog_digest,
            world_template_digest,
            catalog_profile: "profile/combat_labels".to_string(),
        };
        assert_ne!(first, second);
    }

    #[test]
    fn simulation_seed_three_rejects_a_rules_valid_multi_player_graph() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut seed = graph.component("simulation_seed");
        let mut second = seed["actors"][0].clone();
        second["id"] = serde_json::json!("player_two");
        second["location"]["position"] = serde_json::json!({"x": 2, "y": 3});
        second["carried"]["items"] = serde_json::json!([]);
        seed["actors"]
            .as_array_mut()
            .expect("seed actors")
            .push(second);
        graph.write_component("simulation_seed", &seed);

        let error =
            load_simulation(&graph.scenario_path).expect_err("simulator stays single-player");
        assert!(
            error
                .to_string()
                .contains("Simulation Seed 3 requires exactly one player actor, found 2")
        );
    }

    #[test]
    fn canonical_graph_accepts_a_relative_scenario_path() {
        let current = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
        let target = std::fs::canonicalize(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content/test-corpus/first_room.json"),
        )
        .unwrap();
        let current_components = current.components().collect::<Vec<_>>();
        let target_components = target.components().collect::<Vec<_>>();
        let common = current_components
            .iter()
            .zip(&target_components)
            .take_while(|(left, right)| left == right)
            .count();
        assert!(common > 0, "test path must share a filesystem root");

        let mut relative = PathBuf::new();
        for component in &current_components[common..] {
            assert!(
                matches!(component, PathComponent::Normal(_)),
                "canonical current directory must contain only normal trailing components"
            );
            relative.push("..");
        }
        for component in &target_components[common..] {
            relative.push(component.as_os_str());
        }

        assert!(!relative.is_absolute());
        assert_eq!(std::fs::canonicalize(&relative).unwrap(), target);
        let loaded = load_simulation(&relative).expect("relative scenario path should load");
        assert_eq!(loaded.scenario.id, "first_room");
    }

    #[test]
    fn every_tracked_simulation_loads_through_the_canonical_graph() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/test-corpus");
        let mut scenarios = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .filter(|path| {
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                    .and_then(|value| value["kind"].as_str().map(str::to_owned))
                    .as_deref()
                    == Some(SIMULATION_SCENARIO_KIND)
            })
            .collect::<Vec<_>>();
        scenarios.sort();
        assert_eq!(scenarios.len(), 52);
        for path in scenarios {
            load_simulation(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        }
    }

    #[test]
    fn strict_graph_rejects_inline_scenario_content_and_duplicate_targets() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario["rooms"] = serde_json::json!({});
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::Scenario
        );
        assert_eq!(error.diagnostics[0].pointer, "/rooms");
        assert!(error.to_string().contains("unknown field `rooms`"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario["world_template"] = scenario["catalog"].clone();
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].pointer, "/world_template");
        assert!(error.to_string().contains("distinct canonical files"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario.as_object_mut().unwrap().remove("catalog");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::Scenario
        );
        assert_eq!(error.diagnostics[0].pointer, "/catalog");
        assert!(
            error.diagnostics[0]
                .message
                .contains("missing field `catalog`")
        );

        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario["catalog_profile"] = serde_json::json!("");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::Scenario
        );
        assert_eq!(error.diagnostics[0].pointer, "/catalog_profile");
        assert!(error.diagnostics[0].message.contains("must be non-empty"));
    }

    #[test]
    fn strict_graph_rejects_missing_component_kind_and_boundary_mismatch() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut template = graph.component("world_template");
        template.as_object_mut().unwrap().remove("kind");
        graph.write_component("world_template", &template);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::WorldTemplate
        );
        assert_eq!(error.diagnostics[0].pointer, "/kind");
        assert!(error.to_string().contains("missing field `kind`"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut catalog = graph.component("catalog");
        catalog["clean_content"] = serde_json::json!(false);
        catalog["research_boundary"] = serde_json::json!({
            "status": "internal_parity_fixture",
            "notes": concat!("TME-", "PLACEHOLDER test graph"),
            "review_refs": ["test"]
        });
        graph.write_component("catalog", &catalog);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Bundle);
        assert!(error.to_string().contains("classifications must agree"));
    }

    #[test]
    fn diagnostics_preserve_nested_component_and_json_pointer_ownership() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut catalog = graph.component("catalog");
        let rules = catalog["rules_profiles"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        rules["legacy"] = serde_json::json!(true);
        graph.write_component("catalog", &catalog);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Catalog);
        assert!(error.diagnostics[0].pointer.starts_with("/rules_profiles/"));
        assert!(error.diagnostics[0].pointer.ends_with("/legacy"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut template = graph.component("world_template");
        template["realms"]["realm_0"]["levels"]["room_0"]["legacy"] = serde_json::json!(true);
        graph.write_component("world_template", &template);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::WorldTemplate
        );
        assert_eq!(
            error.diagnostics[0].pointer,
            "/realms/realm_0/levels/room_0/legacy"
        );

        let graph = TempGraph::copy_from("first_room.json");
        let mut seed = graph.component("simulation_seed");
        seed["actors"][0]["location"]["level"] = serde_json::json!("missing_level");
        graph.write_component("simulation_seed", &seed);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::SimulationSeed
        );
        assert_eq!(error.diagnostics[0].pointer, "/actors/0/location");
        assert_eq!(
            error.diagnostics[0].message,
            "realm/level does not exist in the selected world template"
        );

        // The terms below come from tests/fixtures/synthetic-terms.txt, the tracked
        // nonsense denylist that .cargo/config.toml configures for cargo-run
        // processes. They prove the REJECTION MECHANISM without the tree carrying a
        // real term. Point TME_BANNED_TERMS_FILE at a different list and this
        // assertion stops holding — by construction, not by defect: a tree that
        // carries no real term cannot write a fixture the real list rejects.
        let graph = TempGraph::copy_from("first_room.json");
        let mut catalog = graph.component("catalog");
        catalog["id"] = serde_json::json!("zorbelquux");
        graph.write_component("catalog", &catalog);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Catalog);
        assert_eq!(error.diagnostics[0].pointer, "/id");
        assert!(error.diagnostics[0].message.contains("banned source term"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut catalog = graph.component("catalog");
        let duplicate = catalog["rules_profiles"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .clone();
        catalog["rules_profiles"]
            .as_object_mut()
            .unwrap()
            .insert("rules/duplicate".to_string(), duplicate);
        graph.write_component("catalog", &catalog);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Catalog);
        assert_eq!(
            error.diagnostics[0].pointer,
            "/rules_profiles/rules~1duplicate"
        );
        assert!(error.diagnostics[0].message.contains("exactly duplicates"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut catalog = graph.component("catalog");
        let selected_item_key = catalog["profiles"]["profile/first_room"]["items"][0]
            .as_str()
            .unwrap()
            .to_string();
        catalog["items"][&selected_item_key]["name"] = serde_json::json!("");
        graph.write_component("catalog", &catalog);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Bundle);
        assert_eq!(error.diagnostics[0].pointer, "/items/0/name");
        assert!(error.diagnostics[0].message.contains("must be non-empty"));

        let graph = TempGraph::copy_from("gold_bank_locker_storage.json");
        let mut catalog = graph.component("catalog");
        let bank_key = catalog["profiles"]["profile/gold_bank_locker_storage"]["banks"][0]
            .as_str()
            .unwrap()
            .to_string();
        catalog["banks"][&bank_key]["transaction_cap_gold"] = serde_json::json!(0);
        graph.write_component("catalog", &catalog);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Bundle);
        assert_eq!(
            error.diagnostics[0].pointer,
            "/storage/banks/0/transaction_cap_gold"
        );
        assert!(error.diagnostics[0].message.contains("must be positive"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut template = graph.component("world_template");
        let level = template["realms"]["realm_0"]["levels"]
            .as_object_mut()
            .unwrap()
            .remove("room_0")
            .unwrap();
        template["realms"]["realm_0"]["levels"]
            .as_object_mut()
            .unwrap()
            .insert("zone/a.b".to_string(), level);
        template["realms"]["realm_0"]["levels"]["zone/a.b"]["width"] = serde_json::json!(0);
        graph.write_component("world_template", &template);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::WorldTemplate
        );
        assert_eq!(
            error.diagnostics[0].pointer,
            "/realms/realm_0/levels/zone~1a.b/width"
        );

        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario["research_boundary"]["review_refs"] = serde_json::json!([]);
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::Scenario
        );
        assert_eq!(
            error.diagnostics[0].pointer,
            "/research_boundary/review_refs"
        );

        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario["description"] = serde_json::json!("zorbelquux material");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::Scenario
        );
        assert_eq!(error.diagnostics[0].pointer, "/description");
        assert!(error.diagnostics[0].message.contains("banned source term"));
    }

    #[test]
    fn canonical_graph_reports_wrong_kind_and_missing_component_paths() {
        let graph = TempGraph::copy_from("first_room.json");
        let mut template = graph.component("world_template");
        template["kind"] = serde_json::json!("catalog");
        graph.write_component("world_template", &template);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::WorldTemplate
        );
        assert_eq!(error.diagnostics[0].pointer, "/kind");
        assert!(error.diagnostics[0].message.contains("world_template"));

        let graph = TempGraph::copy_from("first_room.json");
        let mut scenario = graph.scenario();
        scenario["world_template"] = serde_json::json!("missing-template.json");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].component,
            DiagnosticComponent::Scenario
        );
        assert_eq!(error.diagnostics[0].pointer, "/world_template");
        assert!(error.diagnostics[0].message.contains("os error 2"));
    }

    #[test]
    fn sim_owned_envelopes_reject_wrong_schema_versions_and_kinds() {
        for (field, value, pointer) in [
            ("schema_version", serde_json::json!(2), "/schema_version"),
            ("kind", serde_json::json!("catalog"), "/kind"),
        ] {
            let graph = TempGraph::copy_from("first_room.json");
            let mut scenario = graph.scenario();
            scenario[field] = value;
            graph.write_scenario(&scenario);
            let error = load_simulation(&graph.scenario_path).unwrap_err();
            assert_eq!(
                error.diagnostics[0].component,
                DiagnosticComponent::Scenario
            );
            assert_eq!(error.diagnostics[0].pointer, pointer);
        }

        for (field, value, pointer) in [
            ("schema_version", serde_json::json!(2), "/schema_version"),
            ("kind", serde_json::json!("catalog"), "/kind"),
        ] {
            let graph = TempGraph::copy_from("first_room.json");
            let mut seed = graph.component("simulation_seed");
            seed[field] = value;
            graph.write_component("simulation_seed", &seed);
            let error = load_simulation(&graph.scenario_path).unwrap_err();
            assert_eq!(
                error.diagnostics[0].component,
                DiagnosticComponent::SimulationSeed
            );
            assert_eq!(error.diagnostics[0].pointer, pointer);
        }
    }

    #[cfg(unix)]
    #[test]
    fn canonical_graph_accepts_symlinked_scenario_and_safe_inside_root_component() {
        use std::os::unix::fs::symlink;

        let graph = TempGraph::copy_from("first_room.json");
        let scenario_alias = graph.root.join("scenario-link.json");
        symlink(&graph.scenario_path, &scenario_alias).unwrap();
        load_simulation(&scenario_alias).expect("scenario symlink resolves to its canonical root");

        let catalog_target = graph.component_path("catalog");
        let catalog_alias = graph.root.join("catalog-link.json");
        symlink(&catalog_target, &catalog_alias).unwrap();
        let mut scenario = graph.scenario();
        scenario["catalog"] = serde_json::json!("catalog-link.json");
        graph.write_scenario(&scenario);
        load_simulation(&graph.scenario_path)
            .expect("component symlink stays inside the canonical scenario root");
    }

    #[cfg(unix)]
    #[test]
    fn canonical_graph_rejects_component_symlink_escape() {
        use std::os::unix::fs::symlink;

        let graph = TempGraph::copy_from("first_room.json");
        let outside = std::env::temp_dir().join(format!(
            "tme-sim-outside-catalog-{}-{}",
            std::process::id(),
            NEXT_TEMP_GRAPH.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::copy(graph.component_path("catalog"), &outside).unwrap();
        let alias = graph.root.join("catalog-link.json");
        symlink(&outside, &alias).unwrap();
        let mut scenario = graph.scenario();
        scenario["catalog"] = serde_json::json!("catalog-link.json");
        graph.write_scenario(&scenario);

        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].pointer, "/catalog");
        assert!(
            error
                .to_string()
                .contains("escapes the canonical scenario root")
        );
        std::fs::remove_file(outside).unwrap();
    }

    #[test]
    fn script_references_are_checked_against_the_validated_seed() {
        let graph = TempGraph::copy_from("npc_quest_interactions.json");
        let mut scenario = graph.scenario();
        scenario["script"][0]["interact_with_npc"]["npc_actor_id"] =
            serde_json::json!("missing_npc");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(error.diagnostics[0].component, DiagnosticComponent::Script);
        assert_eq!(
            error.diagnostics[0].pointer,
            "/script/0/interact_with_npc/npc_actor_id"
        );
        assert!(error.to_string().contains("references unknown NPC"));

        let graph = TempGraph::copy_from("npc_quest_interactions.json");
        let mut scenario = graph.scenario();
        scenario["script"][1]["interact_with_npc"]["item_instance_id"] =
            serde_json::json!("missing_item");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].pointer,
            "/script/1/interact_with_npc/item_instance_id"
        );
        assert!(
            error
                .to_string()
                .contains("references unknown item instance")
        );

        let graph = TempGraph::copy_from("npc_quest_interactions.json");
        let mut scenario = graph.scenario();
        scenario["script"][0]["interact_with_npc"]["interaction_id"] =
            serde_json::json!("missing_interaction");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert_eq!(
            error.diagnostics[0].pointer,
            "/script/0/interact_with_npc/interaction_id"
        );
        assert!(error.to_string().contains("references unknown interaction"));

        let graph = TempGraph::copy_from("npc_quest_interactions.json");
        let mut scenario = graph.scenario();
        scenario["script"][0]["interact_with_npc"]["item_instance_id"] =
            serde_json::json!("player_signal_token");
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must be null when the interaction has no carried_item requirement")
        );

        let graph = TempGraph::copy_from("npc_quest_interactions.json");
        let mut scenario = graph.scenario();
        scenario["script"][1]["interact_with_npc"]["item_instance_id"] = Value::Null;
        graph.write_scenario(&scenario);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("must select the required carried item")
        );

        let graph = TempGraph::copy_from("npc_quest_interactions.json");
        let mut seed = graph.component("simulation_seed");
        let wayfinder = seed["actors"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|actor| actor["id"] == "wayfinder")
            .unwrap();
        wayfinder["npc"]["interactions"][1]["transaction"]["requirements"][1]["quantity"] =
            serde_json::json!(2);
        graph.write_component("simulation_seed", &seed);
        let error = load_simulation(&graph.scenario_path).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not match the interaction carried_item requirement")
        );
    }
}
