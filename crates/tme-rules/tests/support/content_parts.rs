//! Explicit four-part content seam for tme-rules integration tests.
//!
//! This helper never parses a Simulation Scenario, resolves its reference
//! graph, or reads a script. Callers provide the Catalog profile literally and
//! the helper reads only the explicit Catalog, World Template, and rules-owned
//! Simulation Seed payload selected by the test.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tme_rules::engine::{GameDefinition, ValidatedWorldSeed};
use tme_rules::{CatalogProfileKey, CatalogV6, Engine, WorldSeedDef, WorldTemplateV3};

#[derive(Debug, Clone)]
pub struct ContentParts {
    pub catalog: Value,
    pub catalog_profile: String,
    pub world_template: Value,
    pub world_seed: Value,
}

impl ContentParts {
    pub fn from_values(
        catalog: Value,
        catalog_profile: impl Into<String>,
        world_template: Value,
        world_seed: Value,
    ) -> Self {
        Self {
            catalog,
            catalog_profile: catalog_profile.into(),
            world_template,
            world_seed,
        }
    }

    pub fn from_paths(
        catalog_path: impl AsRef<Path>,
        catalog_profile: impl Into<String>,
        world_template_path: impl AsRef<Path>,
        simulation_seed_path: impl AsRef<Path>,
    ) -> Self {
        let catalog = read_json(catalog_path.as_ref());
        let world_template = read_json(world_template_path.as_ref());
        let seed_envelope = read_json(simulation_seed_path.as_ref());
        let world_seed = seed_payload(&seed_envelope);
        Self::from_values(catalog, catalog_profile, world_template, world_seed)
    }

    pub fn tracked(case_id: &str, catalog_profile: &str) -> Self {
        let root = prototype_root();
        Self::from_paths(
            root.join("catalogs/prototype_catalog_v6.json"),
            catalog_profile,
            root.join(format!("world_templates/{case_id}.json")),
            root.join(format!("simulation_seeds/{case_id}.json")),
        )
    }

    pub fn decode(
        &self,
    ) -> Result<(CatalogV6, CatalogProfileKey, WorldTemplateV3, WorldSeedDef), String> {
        let catalog = serde_json::from_value(self.catalog.clone())
            .map_err(|error| format!("catalog: {error}"))?;
        let profile = CatalogProfileKey::new(self.catalog_profile.clone())
            .map_err(|error| error.to_string())?;
        let mut world_template = self.world_template.clone();
        complete_compact_world_template_v3_levels(&mut world_template);
        let template = serde_json::from_value(world_template)
            .map_err(|error| format!("world_template: {error}"))?;
        let seed = serde_json::from_value(self.world_seed.clone())
            .map_err(|error| format!("simulation_seed: {error}"))?;
        Ok((catalog, profile, template, seed))
    }

    pub fn validated_seed(&self) -> Result<ValidatedWorldSeed, String> {
        let (catalog, profile, template, seed) = self.decode()?;
        let definition = GameDefinition::from_content(catalog, profile, template)
            .map_err(|error| error.to_string())?;
        ValidatedWorldSeed::new(definition, seed).map_err(|error| error.to_string())
    }

    pub fn definition(&self) -> Result<Arc<GameDefinition>, String> {
        let (catalog, profile, template, _) = self.decode()?;
        GameDefinition::from_content(catalog, profile, template).map_err(|error| error.to_string())
    }

    pub fn engine(&self, rng_seed: u64) -> Result<Engine, String> {
        Engine::new(self.validated_seed()?, rng_seed).map_err(|error| error.to_string())
    }

    pub fn rules_source_mut(&mut self) -> &mut Value {
        let key = self.profile_value()["rules_profile"]
            .as_str()
            .expect("rules profile key")
            .to_string();
        &mut self.catalog["rules_profiles"][&key]
    }

    pub fn skill_catalog_mut(&mut self) -> Option<&mut Value> {
        let key = self.profile_value()["skill_catalog"].as_str()?.to_string();
        Some(&mut self.catalog["skill_catalogs"][&key])
    }

    pub fn selected_mut(&mut self, registry: &str, index: usize) -> &mut Value {
        let key = self.profile_value()[registry][index]
            .as_str()
            .unwrap_or_else(|| panic!("{registry}[{index}] registry key"))
            .to_string();
        &mut self.catalog[registry][&key]
    }

    pub fn selected_by_runtime_id_mut(&mut self, registry: &str, id: &str) -> &mut Value {
        let key = self.profile_value()[registry]
            .as_array()
            .unwrap_or_else(|| panic!("{registry} profile selection"))
            .iter()
            .find_map(|key| {
                let key = key.as_str()?;
                (self.catalog[registry][key]["id"] == id).then(|| key.to_string())
            })
            .unwrap_or_else(|| panic!("selected {registry} row with runtime id {id:?}"));
        &mut self.catalog[registry][&key]
    }

    pub fn selected_len(&self, registry: &str) -> usize {
        self.profile_value()[registry]
            .as_array()
            .unwrap_or_else(|| panic!("{registry} profile selection"))
            .len()
    }

    pub fn push_selected(&mut self, registry: &str, key: &str, value: Value) {
        assert!(
            self.catalog[registry].get(key).is_none(),
            "duplicate test registry key {key}"
        );
        self.catalog[registry][key] = value;
        self.profile_value_mut()[registry]
            .as_array_mut()
            .unwrap_or_else(|| panic!("{registry} profile selection"))
            .push(Value::String(key.to_string()));
    }

    pub fn actors_mut(&mut self) -> &mut Value {
        &mut self.world_seed["actors"]
    }

    pub fn actor_definition_mut(&mut self, actor_index: usize) -> &mut Value {
        let definition_id = self.world_seed["actors"][actor_index]["actor_definition_id"]
            .as_str()
            .unwrap_or_else(|| panic!("actors[{actor_index}].actor_definition_id"))
            .to_string();
        let key = self.profile_value()["actor_definitions"]
            .as_array()
            .expect("actor_definitions profile selection")
            .iter()
            .find_map(|key| {
                let key = key.as_str()?;
                (self.catalog["actor_definitions"][key]["id"] == definition_id)
                    .then(|| key.to_string())
            })
            .unwrap_or_else(|| panic!("selected actor definition {definition_id:?}"));
        &mut self.catalog["actor_definitions"][&key]
    }

    pub fn actor_definition_by_actor_id_mut(&mut self, actor_id: &str) -> &mut Value {
        let actor_index = self.world_seed["actors"]
            .as_array()
            .expect("seed actors")
            .iter()
            .position(|actor| actor["id"] == actor_id)
            .unwrap_or_else(|| panic!("seed actor {actor_id:?}"));
        self.actor_definition_mut(actor_index)
    }

    pub fn summon_actor_definition_mut(&mut self, template_index: usize) -> &mut Value {
        let definition_id =
            self.selected_mut("summon_templates", template_index)["actor_definition_id"]
                .as_str()
                .unwrap_or_else(|| panic!("summon_templates[{template_index}].actor_definition_id"))
                .to_string();
        self.selected_by_runtime_id_mut("actor_definitions", &definition_id)
    }

    pub fn summon_actor_definition_by_template_id_mut(&mut self, template_id: &str) -> &mut Value {
        let definition_id =
            self.selected_by_runtime_id_mut("summon_templates", template_id)["actor_definition_id"]
                .as_str()
                .unwrap_or_else(|| panic!("summon template {template_id:?} actor_definition_id"))
                .to_string();
        self.selected_by_runtime_id_mut("actor_definitions", &definition_id)
    }

    pub fn item_instances_mut(&mut self) -> &mut Value {
        &mut self.world_seed["item_instances"]
    }

    pub fn ground_items_mut(&mut self) -> &mut Value {
        &mut self.world_seed["ground_items"]
    }

    pub fn service_instances_mut(&mut self) -> &mut Value {
        &mut self.world_seed["service_instances"]
    }

    pub fn merchant_inventories_mut(&mut self) -> &mut Value {
        &mut self.world_seed["merchant_inventories"]
    }

    pub fn template_levels_source_mut(&mut self) -> &mut Value {
        &mut self.world_template["realms"]["realm_0"]["levels"]
    }

    pub fn profile_value(&self) -> &Value {
        &self.catalog["profiles"][&self.catalog_profile]
    }

    pub fn profile_value_mut(&mut self) -> &mut Value {
        &mut self.catalog["profiles"][&self.catalog_profile]
    }
}

/// Complete compact, test-authored level rows into the one current World
/// Template 3 shape. This is fixture construction only: it does not recognize
/// or upgrade an older envelope and it is absent from production decoding.
fn complete_compact_world_template_v3_levels(world_template: &mut Value) {
    if world_template.get("schema_version") != Some(&Value::from(3)) {
        return;
    }
    let Some(realms) = world_template
        .get_mut("realms")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    for realm in realms.values_mut() {
        let Some(levels) = realm.get_mut("levels").and_then(Value::as_object_mut) else {
            continue;
        };
        for level in levels.values_mut() {
            let Some(level) = level.as_object_mut() else {
                continue;
            };
            level
                .entry("scene_role")
                .or_insert_with(|| Value::String("combat_space".to_string()));
            level
                .entry("presentation_mode")
                .or_insert_with(|| Value::String("combat_space".to_string()));
            level
                .entry("world_zoom")
                .or_insert_with(|| serde_json::json!({"screen_cell_pitch": [156, 104]}));
            let maximum_clear_sightline = level
                .get("width")
                .and_then(Value::as_u64)
                .unwrap_or(12)
                .clamp(1, 12);
            level
                .entry("maximum_clear_sightline")
                .or_insert_with(|| Value::from(maximum_clear_sightline));
            level.entry("staged_viewport").or_insert(Value::Null);
            level
                .entry("wall_terrain_ids")
                .or_insert_with(|| Value::Array(Vec::new()));
            level
                .entry("static_props")
                .or_insert_with(|| Value::Array(Vec::new()));
        }
    }
    complete_compact_world_template_v3_doors(world_template);
}

/// Give compact test doors exact reciprocal bindings. Production content has
/// no such completion route; tracked documents must author both endpoints.
fn complete_compact_world_template_v3_doors(world_template: &mut Value) {
    let Some(topology) = world_template.get("topology").and_then(Value::as_object) else {
        return;
    };
    let mut rows = topology
        .iter()
        .filter(|(_, edge)| {
            edge["kind"]["kind"] == "door" && edge["kind"].get("binding_id").is_none()
        })
        .map(|(edge_id, edge)| {
            (
                edge_id.clone(),
                edge["at"].clone(),
                edge["target"]["location"].clone(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.cmp(&right.0));

    let mut consumed = std::collections::BTreeSet::new();
    let mut generated = Vec::new();
    for (edge_id, at, target) in &rows {
        if consumed.contains(edge_id) || target.is_null() {
            continue;
        }
        let reciprocal = rows
            .iter()
            .find(|(candidate_id, candidate_at, candidate_target)| {
                candidate_id != edge_id
                    && !consumed.contains(candidate_id)
                    && candidate_at == target
                    && candidate_target == at
            })
            .map(|(candidate_id, _, _)| candidate_id.clone());
        let binding_id = format!("test_binding/{edge_id}");
        let endpoint_id = format!("{binding_id}/a");
        let reciprocal_endpoint_id = format!("{binding_id}/b");
        let topology = world_template["topology"]
            .as_object_mut()
            .expect("topology object");
        set_compact_door_binding(
            &mut topology[edge_id]["kind"],
            &binding_id,
            &endpoint_id,
            &reciprocal_endpoint_id,
        );
        consumed.insert(edge_id.clone());
        if let Some(reciprocal_id) = reciprocal {
            set_compact_door_binding(
                &mut topology[&reciprocal_id]["kind"],
                &binding_id,
                &reciprocal_endpoint_id,
                &endpoint_id,
            );
            consumed.insert(reciprocal_id);
        } else {
            let mut kind = topology[edge_id]["kind"].clone();
            set_compact_door_binding(
                &mut kind,
                &binding_id,
                &reciprocal_endpoint_id,
                &endpoint_id,
            );
            generated.push((
                format!("{edge_id}/reciprocal"),
                serde_json::json!({
                    "at": target,
                    "target": {"kind": "position", "location": at},
                    "kind": kind,
                    "hidden": topology[edge_id]["hidden"].clone(),
                }),
            ));
        }
    }
    let topology = world_template["topology"]
        .as_object_mut()
        .expect("topology object");
    for (edge_id, edge) in generated {
        topology.insert(edge_id, edge);
    }
}

fn set_compact_door_binding(
    kind: &mut Value,
    binding_id: &str,
    endpoint_id: &str,
    reciprocal_endpoint_id: &str,
) {
    let object = kind.as_object_mut().expect("test door kind object");
    object.insert(
        "binding_id".to_string(),
        Value::String(binding_id.to_string()),
    );
    object.insert(
        "endpoint_id".to_string(),
        Value::String(endpoint_id.to_string()),
    );
    object.insert(
        "reciprocal_endpoint_id".to_string(),
        Value::String(reciprocal_endpoint_id.to_string()),
    );
}

fn prototype_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/test-corpus")
}

fn read_json(path: &Path) -> Value {
    let source =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&source).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn seed_payload(envelope: &Value) -> Value {
    let object = envelope
        .as_object()
        .expect("simulation seed source must be an object");
    serde_json::json!({
        "actors": object.get("actors").expect("seed actors"),
        "item_instances": object.get("item_instances").expect("seed item_instances"),
        "ground_items": object.get("ground_items").expect("seed ground_items"),
        "service_instances": object.get("service_instances").expect("seed service_instances"),
        "merchant_inventories": object.get("merchant_inventories").expect("seed merchant_inventories"),
        "ecology_sites": object.get("ecology_sites").expect("seed ecology_sites"),
    })
}
