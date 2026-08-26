use std::collections::{BTreeMap, HashMap};
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use super::{
    ActorDefinitionDef, BankDef, DamageLabelDef, ItemDef, LairDefinitionDef, LockerVaultDef,
    LootTableDef, PhysicalDamageAffinityProfileDef, ProfessionActionDef, QuestDef,
    ResearchBoundary, RulesDef, ScavengingProfileDef, ServiceDefinitionDef, SkillCatalogDef,
    SpawnGroupDef, SpellDef, SummonTemplateDef, TerrainDef, ValidationError,
};

pub const CATALOG_SCHEMA_VERSION: u32 = 6;
pub const CATALOG_KIND: &str = "catalog";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CatalogRegistryKey(String);

impl CatalogRegistryKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::new(vec![
                "catalog registry key must be non-empty".to_string(),
            ]));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CatalogRegistryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for CatalogRegistryKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for CatalogRegistryKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CatalogProfileKey(String);

impl CatalogProfileKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::new(vec![
                "catalog profile key must be non-empty".to_string(),
            ]));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CatalogProfileKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<String> for CatalogProfileKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for CatalogProfileKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogProfileDef {
    pub rules_profile: CatalogRegistryKey,
    #[serde(deserialize_with = "deserialize_required_nullable_registry_key")]
    pub skill_catalog: Option<CatalogRegistryKey>,
    pub damage_labels: Vec<CatalogRegistryKey>,
    pub terrains: Vec<CatalogRegistryKey>,
    pub items: Vec<CatalogRegistryKey>,
    pub spells: Vec<CatalogRegistryKey>,
    pub quests: Vec<CatalogRegistryKey>,
    pub actor_definitions: Vec<CatalogRegistryKey>,
    pub physical_damage_affinity_profiles: Vec<CatalogRegistryKey>,
    pub loot_tables: Vec<CatalogRegistryKey>,
    pub spawn_groups: Vec<CatalogRegistryKey>,
    pub lair_definitions: Vec<CatalogRegistryKey>,
    pub summon_templates: Vec<CatalogRegistryKey>,
    pub profession_actions: Vec<CatalogRegistryKey>,
    pub service_definitions: Vec<CatalogRegistryKey>,
    pub banks: Vec<CatalogRegistryKey>,
    pub locker_vaults: Vec<CatalogRegistryKey>,
}

fn deserialize_required_nullable_registry_key<'de, D>(
    deserializer: D,
) -> Result<Option<CatalogRegistryKey>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CatalogRegistryKey>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::CatalogProfileDef;

    #[test]
    fn catalog_profile_skill_catalog_is_required_nullable() {
        let valid = serde_json::json!({
            "rules_profile": "rules/base",
            "skill_catalog": null,
            "damage_labels": [],
            "terrains": [],
            "items": [],
            "spells": [],
            "quests": [],
            "actor_definitions": [],
            "physical_damage_affinity_profiles": [],
            "loot_tables": [],
            "spawn_groups": [],
            "lair_definitions": [],
            "summon_templates": [],
            "profession_actions": [],
            "service_definitions": [],
            "banks": [],
            "locker_vaults": []
        });
        let decoded: CatalogProfileDef = serde_json::from_value(valid.clone())
            .expect("explicit null skill catalog should decode");
        assert!(decoded.skill_catalog.is_none());

        let mut missing = valid;
        missing
            .as_object_mut()
            .expect("profile object")
            .remove("skill_catalog");
        assert!(serde_json::from_value::<CatalogProfileDef>(missing).is_err());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogV6 {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub clean_content: bool,
    pub research_boundary: ResearchBoundary,
    pub rules_profiles: BTreeMap<CatalogRegistryKey, RulesDef>,
    pub skill_catalogs: BTreeMap<CatalogRegistryKey, SkillCatalogDef>,
    pub damage_labels: BTreeMap<CatalogRegistryKey, DamageLabelDef>,
    pub terrains: BTreeMap<CatalogRegistryKey, TerrainDef>,
    pub items: BTreeMap<CatalogRegistryKey, ItemDef>,
    pub spells: BTreeMap<CatalogRegistryKey, SpellDef>,
    pub quests: BTreeMap<CatalogRegistryKey, QuestDef>,
    pub actor_definitions: BTreeMap<CatalogRegistryKey, ActorDefinitionDef>,
    pub scavenging_profiles: BTreeMap<CatalogRegistryKey, ScavengingProfileDef>,
    pub physical_damage_affinity_profiles:
        BTreeMap<CatalogRegistryKey, PhysicalDamageAffinityProfileDef>,
    pub loot_tables: BTreeMap<CatalogRegistryKey, LootTableDef>,
    pub spawn_groups: BTreeMap<CatalogRegistryKey, SpawnGroupDef>,
    pub lair_definitions: BTreeMap<CatalogRegistryKey, LairDefinitionDef>,
    pub summon_templates: BTreeMap<CatalogRegistryKey, SummonTemplateDef>,
    pub profession_actions: BTreeMap<CatalogRegistryKey, ProfessionActionDef>,
    pub service_definitions: BTreeMap<CatalogRegistryKey, ServiceDefinitionDef>,
    pub banks: BTreeMap<CatalogRegistryKey, BankDef>,
    pub locker_vaults: BTreeMap<CatalogRegistryKey, LockerVaultDef>,
    pub profiles: BTreeMap<CatalogProfileKey, CatalogProfileDef>,
}

/// Exact ordered immutable source selection for one catalog profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectedCatalog {
    pub catalog_id: String,
    pub clean_content: bool,
    pub research_boundary: ResearchBoundary,
    pub profile_key: CatalogProfileKey,
    pub rules: RulesDef,
    pub skill_catalog: Option<SkillCatalogDef>,
    pub damage_labels: Vec<DamageLabelDef>,
    pub terrains: Vec<TerrainDef>,
    pub items: Vec<ItemDef>,
    pub spells: Vec<SpellDef>,
    pub quests: Vec<QuestDef>,
    pub actor_definitions: Vec<ActorDefinitionDef>,
    pub scavenging_profiles: BTreeMap<CatalogRegistryKey, ScavengingProfileDef>,
    pub physical_damage_affinity_profiles: Vec<PhysicalDamageAffinityProfileDef>,
    pub loot_tables: Vec<LootTableDef>,
    pub spawn_groups: Vec<SpawnGroupDef>,
    pub lair_definitions: Vec<LairDefinitionDef>,
    pub summon_templates: Vec<SummonTemplateDef>,
    pub profession_actions: Vec<ProfessionActionDef>,
    pub service_definitions: Vec<ServiceDefinitionDef>,
    pub banks: Vec<BankDef>,
    pub locker_vaults: Vec<LockerVaultDef>,
}

impl CatalogV6 {
    pub fn select(
        &self,
        profile_key: &CatalogProfileKey,
    ) -> Result<SelectedCatalog, ValidationError> {
        let mut errors = Vec::new();
        self.validate_structure(&mut errors);
        if let Ok(policy) = super::boundary_policy(self.clean_content, &self.research_boundary) {
            match serde_json::to_value(self) {
                Ok(value) => {
                    if let Err(error) = super::scan_raw_documents(policy, [("catalog", &value)]) {
                        errors.extend(error.messages().iter().cloned());
                    }
                }
                Err(error) => errors.push(format!(
                    "catalog could not be serialized for boundary validation: {error}"
                )),
            }
        }
        let Some(profile) = self.profiles.get(profile_key) else {
            errors.push(format!(
                "catalog_profile {:?} does not exist in catalog.profiles",
                profile_key.as_str()
            ));
            return Err(ValidationError::new(errors));
        };

        let rules = select_one(
            "profiles.rules_profile",
            &profile.rules_profile,
            &self.rules_profiles,
            &mut errors,
        );
        let skill_catalog = profile.skill_catalog.as_ref().and_then(|key| {
            select_one(
                "profiles.skill_catalog",
                key,
                &self.skill_catalogs,
                &mut errors,
            )
        });
        let damage_labels = select_many(
            "profiles.damage_labels",
            &profile.damage_labels,
            &self.damage_labels,
            |row| row.id.as_str(),
            &mut errors,
        );
        let terrains = select_many(
            "profiles.terrains",
            &profile.terrains,
            &self.terrains,
            |row| row.id.as_str(),
            &mut errors,
        );
        let items = select_many(
            "profiles.items",
            &profile.items,
            &self.items,
            |row| row.id.as_str(),
            &mut errors,
        );
        let spells = select_many(
            "profiles.spells",
            &profile.spells,
            &self.spells,
            |row| row.id.as_str(),
            &mut errors,
        );
        let quests = select_many(
            "profiles.quests",
            &profile.quests,
            &self.quests,
            |row| row.id.as_str(),
            &mut errors,
        );
        let actor_definitions = select_many(
            "profiles.actor_definitions",
            &profile.actor_definitions,
            &self.actor_definitions,
            |row| row.id.as_str(),
            &mut errors,
        );
        let physical_damage_affinity_profiles = select_many(
            "profiles.physical_damage_affinity_profiles",
            &profile.physical_damage_affinity_profiles,
            &self.physical_damage_affinity_profiles,
            |row| row.id.as_str(),
            &mut errors,
        );
        let loot_tables = select_many(
            "profiles.loot_tables",
            &profile.loot_tables,
            &self.loot_tables,
            |row| row.id.as_str(),
            &mut errors,
        );
        let spawn_groups = select_many(
            "profiles.spawn_groups",
            &profile.spawn_groups,
            &self.spawn_groups,
            |row| row.id.as_str(),
            &mut errors,
        );
        let lair_definitions = select_many(
            "profiles.lair_definitions",
            &profile.lair_definitions,
            &self.lair_definitions,
            |row| row.id.as_str(),
            &mut errors,
        );
        let summon_templates = select_many(
            "profiles.summon_templates",
            &profile.summon_templates,
            &self.summon_templates,
            |row| row.id.as_str(),
            &mut errors,
        );
        let profession_actions = select_many(
            "profiles.profession_actions",
            &profile.profession_actions,
            &self.profession_actions,
            |row| row.id.as_str(),
            &mut errors,
        );
        let service_definitions = select_many(
            "profiles.service_definitions",
            &profile.service_definitions,
            &self.service_definitions,
            |row| row.id.as_str(),
            &mut errors,
        );
        let banks = select_many(
            "profiles.banks",
            &profile.banks,
            &self.banks,
            |row| row.id.as_str(),
            &mut errors,
        );
        let locker_vaults = select_many(
            "profiles.locker_vaults",
            &profile.locker_vaults,
            &self.locker_vaults,
            |row| row.id.as_str(),
            &mut errors,
        );

        if !errors.is_empty() {
            return Err(ValidationError::new(errors));
        }

        Ok(SelectedCatalog {
            catalog_id: self.id.clone(),
            clean_content: self.clean_content,
            research_boundary: self.research_boundary.clone(),
            profile_key: profile_key.clone(),
            rules: rules.expect("checked selected rules profile"),
            skill_catalog,
            damage_labels,
            terrains,
            items,
            spells,
            quests,
            actor_definitions,
            scavenging_profiles: self.scavenging_profiles.clone(),
            physical_damage_affinity_profiles,
            loot_tables,
            spawn_groups,
            lair_definitions,
            summon_templates,
            profession_actions,
            service_definitions,
            banks,
            locker_vaults,
        })
    }

    fn validate_structure(&self, errors: &mut Vec<String>) {
        if self.schema_version != CATALOG_SCHEMA_VERSION {
            errors.push(format!(
                "catalog.schema_version must be {CATALOG_SCHEMA_VERSION}"
            ));
        }
        if self.kind != CATALOG_KIND {
            errors.push(format!("catalog.kind must be {CATALOG_KIND:?}"));
        }
        if self.id.trim().is_empty() {
            errors.push("catalog.id must be non-empty".to_string());
        }
        super::validation::validate_research_boundary(
            self.clean_content,
            &self.research_boundary,
            "catalog",
            errors,
        );

        validate_registry("rules_profiles", &self.rules_profiles, errors);
        validate_registry("skill_catalogs", &self.skill_catalogs, errors);
        validate_registry("damage_labels", &self.damage_labels, errors);
        validate_registry("terrains", &self.terrains, errors);
        validate_registry("items", &self.items, errors);
        validate_registry("spells", &self.spells, errors);
        validate_registry("quests", &self.quests, errors);
        validate_registry("actor_definitions", &self.actor_definitions, errors);
        validate_registry("scavenging_profiles", &self.scavenging_profiles, errors);
        validate_registry(
            "physical_damage_affinity_profiles",
            &self.physical_damage_affinity_profiles,
            errors,
        );
        validate_registry("loot_tables", &self.loot_tables, errors);
        validate_registry("spawn_groups", &self.spawn_groups, errors);
        validate_registry("lair_definitions", &self.lair_definitions, errors);
        validate_registry("summon_templates", &self.summon_templates, errors);
        validate_registry("profession_actions", &self.profession_actions, errors);
        validate_registry("service_definitions", &self.service_definitions, errors);
        validate_registry("banks", &self.banks, errors);
        validate_registry("locker_vaults", &self.locker_vaults, errors);
        validate_registry("profiles", &self.profiles, errors);

        if self.profiles.is_empty() {
            errors.push("catalog.profiles must be non-empty".to_string());
        }
    }
}

fn validate_registry<K, V>(label: &str, rows: &BTreeMap<K, V>, errors: &mut Vec<String>)
where
    K: Ord + fmt::Debug + fmt::Display,
    V: Serialize,
{
    let mut exact_values = HashMap::<String, String>::new();
    for (key, value) in rows {
        if key.to_string().trim().is_empty() {
            errors.push(format!("catalog.{label} contains an empty registry key"));
        }
        let canonical = canonical_json(value);
        if let Some(previous) = exact_values.insert(canonical, key.to_string()) {
            errors.push(format!(
                "catalog.{label}[{key:?}] exactly duplicates catalog.{label}[{previous:?}]"
            ));
        }
    }
}

fn canonical_json<T: Serialize>(value: &T) -> String {
    fn sort(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let sorted = map
                    .into_iter()
                    .map(|(key, value)| (key, sort(value)))
                    .collect::<BTreeMap<_, _>>();
                serde_json::to_value(sorted).expect("canonical object is serializable")
            }
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(sort).collect())
            }
            other => other,
        }
    }

    let value = serde_json::to_value(value).expect("content source type must serialize");
    serde_json::to_string(&sort(value)).expect("canonical content value must serialize")
}

fn select_one<T: Clone>(
    label: &str,
    key: &CatalogRegistryKey,
    registry: &BTreeMap<CatalogRegistryKey, T>,
    errors: &mut Vec<String>,
) -> Option<T> {
    match registry.get(key) {
        Some(value) => Some(value.clone()),
        None => {
            errors.push(format!("{label} references unknown registry key {key:?}"));
            None
        }
    }
}

fn select_many<T, F>(
    label: &str,
    keys: &[CatalogRegistryKey],
    registry: &BTreeMap<CatalogRegistryKey, T>,
    runtime_id: F,
    errors: &mut Vec<String>,
) -> Vec<T>
where
    T: Clone,
    F: Fn(&T) -> &str,
{
    let mut selected = Vec::with_capacity(keys.len());
    let mut selected_keys = HashMap::<&CatalogRegistryKey, usize>::new();
    let mut runtime_ids = HashMap::<String, usize>::new();
    for (index, key) in keys.iter().enumerate() {
        if let Some(previous) = selected_keys.insert(key, index) {
            errors.push(format!(
                "{label}[{index}] duplicates selected registry key at {label}[{previous}]"
            ));
        }
        let Some(value) = registry.get(key) else {
            errors.push(format!(
                "{label}[{index}] references unknown registry key {key:?}"
            ));
            continue;
        };
        let id = runtime_id(value);
        if let Some(previous) = runtime_ids.insert(id.to_string(), index) {
            errors.push(format!(
                "{label}[{index}] selects runtime id {id:?} already selected at {label}[{previous}]"
            ));
        }
        selected.push(value.clone());
    }
    selected
}
