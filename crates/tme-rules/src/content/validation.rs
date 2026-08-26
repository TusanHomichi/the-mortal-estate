use std::collections::{BTreeMap, HashMap, HashSet};

use crate::model::{
    ActorKind, ItemBindingState, ItemPlacementKind, MAX_SKILL_LEVEL, SpellCastClass,
    SpellCastingMethod, SpellCatalogState, SpellDurationPolicy, SpellEffectFamily,
    SpellResistanceMitigation, SpellTargetKind,
};

use super::ai::validate_actor_ai_definition;
use super::armor::validate_armor_definition;
use super::weapons::validate_weapon_definition;
use super::*;

mod boundary;
mod quests;
mod services;
mod social;
mod transactions;
mod world_seed;
mod world_template;

pub(crate) use boundary::validate_research_boundary;
pub use boundary::{
    BannedTerms, ContentBoundaryPolicy, TermsError, boundary_policy, scan_raw_documents,
    scan_raw_documents_with,
};
pub use world_seed::{
    SeedEcologyGroupView, SeedItemValidationView, SeedMerchantCapabilityView,
    SeedPromotionCapabilityView, SeedServiceCapabilityKind, SeedSpellTeachingPairView,
    SeedWorldPositionStatus, WorldSeedValidationContext,
};

use social::{validate_actor_social_profile, validate_spell_social_definition};

const CANONICAL_DAMAGE_LABEL_IDS: &[&str] = &["fatal", "heavy", "light", "moderate", "severe"];
const SPELL_TERRAIN_OVERLAY_PASSABILITY: &[&str] = &[
    "blocked",
    "passable",
    "hindered",
    "unknown",
    "remove_overlay",
];
const SPELL_TERRAIN_OVERLAY_SIGHT: &[&str] =
    &["blocked", "obscured", "clear", "unknown", "remove_overlay"];
const SPELL_TERRAIN_OVERLAY_HAZARD: &[&str] =
    &["fire", "cold", "storm", "poison", "lava", "unknown"];
const SPELL_DOOR_CONTROL_ACTIONS: &[&str] = &["open", "close", "reveal_secret", "hide_secret"];
const SPELL_ITEM_UTILITY_ACTIONS: &[&str] = &["identify", "enchant_weapon", "transform_item"];
const SPELL_LOCATE_SUBJECTS: &[&str] = &["actor", "item", "level"];
const SPELL_SCRY_SCOPES: &[&str] = &["level", "coordinate"];
const MONSTER_ABILITY_KINDS: &[&str] = &["spell", "special_attack"];
const MONSTER_ABILITY_TARGET_POLICIES: &[&str] = &["nearest_hostile", "self"];
const PROFESSION_ACTION_KINDS: &[&str] = &["hide", "martial_hand_block"];
const SPELL_TEACHING_LANES: &[&str] = &[
    "wizard_magic",
    "thaumaturge_magic",
    "thief_magic",
    "knight_magic",
];

pub(crate) fn is_spell_teaching_lane(track_id: &str) -> bool {
    SPELL_TEACHING_LANES.contains(&track_id)
}
const ACTIVE_EFFECT_STACKING: &[&str] =
    &["replace_same_kind", "stack_instance", "refresh_duration"];

fn is_status_like_spell_family(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::Concealment
            | SpellEffectFamily::ControlStatus
            | SpellEffectFamily::Curse
            | SpellEffectFamily::Darkness
            | SpellEffectFamily::FallProtection
            | SpellEffectFamily::Light
            | SpellEffectFamily::Poison
            | SpellEffectFamily::Protection
            | SpellEffectFamily::Resistance
            | SpellEffectFamily::Speed
            | SpellEffectFamily::Vision
            | SpellEffectFamily::WaterBreathing
            | SpellEffectFamily::WeaponEnchant
    )
}

fn requires_item_target(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::WeaponEnchant
    )
}

fn requires_bu_target(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::DoorControl
            | SpellEffectFamily::SecretDetection
            | SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::WeaponEnchant
            | SpellEffectFamily::Locate
            | SpellEffectFamily::Portal
            | SpellEffectFamily::Scry
    )
}

fn requires_spell_potency(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::Curse
            | SpellEffectFamily::DirectDamage
            | SpellEffectFamily::Healing
            | SpellEffectFamily::Poison
    )
}

fn monster_supports_target(kind: SpellTargetKind) -> bool {
    matches!(
        kind,
        SpellTargetKind::Actor
            | SpellTargetKind::Area
            | SpellTargetKind::Coordinate
            | SpellTargetKind::None
            | SpellTargetKind::SelfTarget
    )
}

fn monster_supports_effect(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::AttributeBuff
            | SpellEffectFamily::ControlStatus
            | SpellEffectFamily::Curse
            | SpellEffectFamily::Darkness
            | SpellEffectFamily::DirectDamage
            | SpellEffectFamily::Healing
            | SpellEffectFamily::Light
            | SpellEffectFamily::Poison
            | SpellEffectFamily::PoisonCure
            | SpellEffectFamily::Protection
            | SpellEffectFamily::Resistance
            | SpellEffectFamily::Summon
            | SpellEffectFamily::TerrainOverlay
    )
}

fn monster_effect_supports_target(family: SpellEffectFamily, target: SpellTargetKind) -> bool {
    match family {
        SpellEffectFamily::AttributeBuff
        | SpellEffectFamily::ControlStatus
        | SpellEffectFamily::Curse
        | SpellEffectFamily::Healing
        | SpellEffectFamily::Poison
        | SpellEffectFamily::PoisonCure
        | SpellEffectFamily::Protection
        | SpellEffectFamily::Resistance => {
            matches!(target, SpellTargetKind::Actor | SpellTargetKind::SelfTarget)
        }
        SpellEffectFamily::DirectDamage => matches!(
            target,
            SpellTargetKind::Actor | SpellTargetKind::Area | SpellTargetKind::Coordinate
        ),
        SpellEffectFamily::Darkness
        | SpellEffectFamily::Light
        | SpellEffectFamily::TerrainOverlay => {
            matches!(target, SpellTargetKind::Area | SpellTargetKind::Coordinate)
        }
        SpellEffectFamily::Summon => target == SpellTargetKind::Coordinate,
        _ => false,
    }
}

fn gcd_u32(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[derive(Debug, Clone)]
struct StorageValidationView {
    banks: Vec<BankDef>,
    locker_vaults: Vec<LockerVaultDef>,
}

#[derive(Debug, Clone)]
pub(super) struct ValidationBundle {
    clean_content: bool,
    rules: RulesDef,
    skill_catalog: Option<SkillCatalogDef>,
    world_template: WorldTemplateV3,
    terrains: Vec<TerrainDef>,
    quests: Vec<QuestDef>,
    actor_definitions: Vec<ActorDefinitionDef>,
    scavenging_profiles: BTreeMap<CatalogRegistryKey, ScavengingProfileDef>,
    physical_damage_affinity_profiles: Vec<PhysicalDamageAffinityProfileDef>,
    loot_tables: Vec<LootTableDef>,
    spawn_groups: Vec<SpawnGroupDef>,
    lair_definitions: Vec<LairDefinitionDef>,
    summon_templates: Vec<SummonTemplateDef>,
    items: Vec<ItemDef>,
    item_instances: BTreeMap<String, ItemInstanceSeedDef>,
    damage_labels: Vec<DamageLabelDef>,
    spells: Vec<SpellDef>,
    service_definitions: Vec<ServiceDefinitionDef>,
    profession_actions: Vec<ProfessionActionDef>,
    storage: StorageValidationView,
}

impl SelectedCatalog {
    pub fn validate_with_template(
        &self,
        template: &WorldTemplateV3,
    ) -> Result<(), ValidationError> {
        let policy = boundary_policy(self.clean_content, &self.research_boundary)?;
        let template_value = serde_json::to_value(template).map_err(|error| {
            ValidationError::new(vec![format!(
                "world_template could not be serialized for boundary validation: {error}"
            )])
        })?;
        scan_raw_documents(policy, [("world_template", &template_value)])?;

        let mut errors = Vec::new();
        world_template::validate_envelope(template, &mut errors);
        world_template::validate_template(
            template,
            &self.terrains,
            self.clean_content,
            &mut errors,
        );

        ValidationBundle::definition_only(self, template).validate_definition(errors)
    }
}

impl WorldTemplateV3 {
    pub fn validate_with(&self, catalog: &SelectedCatalog) -> Result<(), ValidationError> {
        catalog.validate_with_template(self)
    }
}

impl WorldSeedDef {
    pub fn validate_with(
        &self,
        catalog: &SelectedCatalog,
        template: &WorldTemplateV3,
    ) -> Result<(), ValidationError> {
        catalog.validate_with_template(template)?;
        let context = world_seed::SourceWorldSeedValidationContext::new(catalog, template);
        self.validate_with_context(&context)
    }

    pub fn validate_with_context(
        &self,
        context: &impl WorldSeedValidationContext,
    ) -> Result<(), ValidationError> {
        let seed_value = serde_json::to_value(self).map_err(|error| {
            ValidationError::new(vec![format!(
                "simulation_seed could not be serialized for boundary validation: {error}"
            )])
        })?;
        scan_raw_documents(
            context.boundary_policy(),
            [("simulation_seed", &seed_value)],
        )?;
        world_seed::validate_world_seed(self, context)
    }
}

impl ValidationBundle {
    fn definition_only(catalog: &SelectedCatalog, template: &WorldTemplateV3) -> Self {
        Self {
            clean_content: catalog.clean_content,
            rules: catalog.rules.clone(),
            skill_catalog: catalog.skill_catalog.clone(),
            world_template: template.clone(),
            terrains: catalog.terrains.clone(),
            quests: catalog.quests.clone(),
            actor_definitions: catalog.actor_definitions.clone(),
            scavenging_profiles: catalog.scavenging_profiles.clone(),
            physical_damage_affinity_profiles: catalog.physical_damage_affinity_profiles.clone(),
            loot_tables: catalog.loot_tables.clone(),
            spawn_groups: catalog.spawn_groups.clone(),
            lair_definitions: catalog.lair_definitions.clone(),
            summon_templates: catalog.summon_templates.clone(),
            items: catalog.items.clone(),
            item_instances: BTreeMap::new(),
            damage_labels: catalog.damage_labels.clone(),
            spells: catalog.spells.clone(),
            service_definitions: catalog.service_definitions.clone(),
            profession_actions: catalog.profession_actions.clone(),
            storage: StorageValidationView {
                banks: catalog.banks.clone(),
                locker_vaults: catalog.locker_vaults.clone(),
            },
        }
    }

    fn validate_definition(&self, mut errors: Vec<String>) -> Result<(), ValidationError> {
        self.validate_rules(!self.clean_content, &mut errors);
        if let Some(skill_catalog) = &self.skill_catalog {
            skill_catalog.validate_intrinsic("skill_catalog", &mut errors);
        }
        self.validate_items(&mut errors);
        self.validate_actor_definitions(&mut errors);
        self.validate_ecology_definitions(&mut errors);
        self.validate_summon_templates(&mut errors);
        self.validate_labels(&mut errors);
        self.validate_spells(!self.clean_content, &mut errors);
        self.validate_quests(&mut errors);
        self.validate_storage(&mut errors);
        self.validate_profession_actions(&mut errors);
        validate_service_definition_intrinsics(self, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::new(errors))
        }
    }
}

fn validate_service_definition_intrinsics(bundle: &ValidationBundle, errors: &mut Vec<String>) {
    let mut definition_ids = HashMap::new();
    let mut transaction_grant_instance_ids = HashMap::new();
    for (definition_index, definition) in bundle.service_definitions.iter().enumerate() {
        let label = format!("service_definitions[{definition_index}]");
        if definition.id.trim().is_empty() {
            errors.push(format!("{label}.id must be non-empty"));
        } else if let Some(previous) =
            definition_ids.insert(definition.id.as_str(), definition_index)
        {
            errors.push(format!(
                "{label}.id duplicates service_definitions[{previous}].id"
            ));
        }
        if definition.name.trim().is_empty() {
            errors.push(format!("{label}.name must be non-empty"));
        }
        if definition.capabilities.is_empty() {
            errors.push(format!("{label}.capabilities must be a non-empty list"));
        }
        let mut capability_ids = HashMap::new();
        let mut capability_kinds = HashMap::new();
        for (capability_index, capability) in definition.capabilities.iter().enumerate() {
            let capability_label = format!("{label}.capabilities[{capability_index}]");
            let id = services::capability_id(capability);
            if id.trim().is_empty() {
                errors.push(format!("{capability_label}.id must be non-empty"));
            } else if let Some(previous) = capability_ids.insert(id, capability_index) {
                errors.push(format!(
                    "{capability_label}.id duplicates {label}.capabilities[{previous}].id"
                ));
            }
            let kind = match capability {
                ServiceCapabilityDef::SkillTraining { .. } => "skill_training",
                ServiceCapabilityDef::SkillCritique { .. } => "skill_critique",
                ServiceCapabilityDef::SpellTeaching { .. } => "spell_teaching",
                ServiceCapabilityDef::ClassPromotion { .. } => "class_promotion",
                ServiceCapabilityDef::ServiceTransaction { .. } => "service_transaction",
                ServiceCapabilityDef::Merchant { .. } => "merchant",
                ServiceCapabilityDef::ItemService { .. } => "item_service",
                ServiceCapabilityDef::Restoration { .. } => "restoration",
                ServiceCapabilityDef::Bank { .. } => "bank",
                ServiceCapabilityDef::Locker { .. } => "locker",
            };
            if let Some(previous) = capability_kinds.insert(kind, capability_index) {
                errors.push(format!(
                    "{capability_label}.kind duplicates {label}.capabilities[{previous}].kind"
                ));
            }
            match capability {
                ServiceCapabilityDef::SkillTraining { offers, .. } => {
                    bundle.validate_training_offers(&capability_label, offers, errors);
                }
                ServiceCapabilityDef::SpellTeaching {
                    training_capability_id,
                    teachings,
                    ..
                } => {
                    if training_capability_id.trim().is_empty() {
                        errors.push(format!(
                            "{capability_label}.training_capability_id must be non-empty"
                        ));
                    }
                    if teachings.is_empty() {
                        errors.push(format!(
                            "{capability_label}.teachings must be a non-empty list"
                        ));
                    }
                    validate_spell_teaching_definition(
                        bundle,
                        definition,
                        &capability_label,
                        training_capability_id,
                        teachings,
                        errors,
                    );
                }
                ServiceCapabilityDef::ClassPromotion { transaction, .. } => {
                    let transaction_label = format!("{capability_label}.transaction");
                    let summary = bundle.validate_transaction(
                        &transaction_label,
                        transaction,
                        transactions::TransactionPolicy::Promotion,
                        errors,
                    );
                    validate_promotion_definition(
                        bundle,
                        &transaction_label,
                        transaction,
                        &summary,
                        errors,
                    );
                    bundle.record_transaction_grants(
                        &transaction_label,
                        &summary,
                        &mut transaction_grant_instance_ids,
                        errors,
                    );
                }
                ServiceCapabilityDef::ServiceTransaction { transactions, .. } => {
                    if transactions.is_empty() {
                        errors.push(format!(
                            "{capability_label}.transactions must be a non-empty list"
                        ));
                    }
                    let mut transaction_ids = HashMap::new();
                    for (transaction_index, transaction) in transactions.iter().enumerate() {
                        let transaction_label =
                            format!("{capability_label}.transactions[{transaction_index}]");
                        if let Some(previous) =
                            transaction_ids.insert(transaction.id.as_str(), transaction_index)
                        {
                            errors.push(format!(
                                "{transaction_label}.id duplicates {capability_label}.transactions[{previous}].id"
                            ));
                        }
                        let summary = bundle.validate_transaction(
                            &transaction_label,
                            transaction,
                            transactions::TransactionPolicy::GenericService,
                            errors,
                        );
                        bundle.record_transaction_grants(
                            &transaction_label,
                            &summary,
                            &mut transaction_grant_instance_ids,
                            errors,
                        );
                    }
                }
                ServiceCapabilityDef::Merchant { player_sales, .. } => {
                    if player_sales
                        .as_ref()
                        .is_some_and(|policy| policy.pawn_listing_multiplier == 0)
                    {
                        errors.push(format!(
                            "{capability_label}.player_sales.pawn_listing_multiplier must be positive"
                        ));
                    }
                }
                ServiceCapabilityDef::Bank { bank_id, .. } => {
                    if !bundle.storage.banks.iter().any(|bank| bank.id == *bank_id) {
                        errors.push(format!(
                            "{capability_label}.bank_id references unknown bank {bank_id:?}"
                        ));
                    }
                }
                ServiceCapabilityDef::Locker { vault_id, .. } => {
                    if !bundle
                        .storage
                        .locker_vaults
                        .iter()
                        .any(|vault| vault.id == *vault_id)
                    {
                        errors.push(format!(
                            "{capability_label}.vault_id references unknown locker vault {vault_id:?}"
                        ));
                    }
                }
                ServiceCapabilityDef::ItemService { operations, .. } => {
                    if operations.is_empty() {
                        errors.push(format!(
                            "{capability_label}.operations must be a non-empty list"
                        ));
                    }
                    let mut operation_kinds = HashSet::new();
                    for (operation_index, operation) in operations.iter().enumerate() {
                        let operation_label =
                            format!("{capability_label}.operations[{operation_index}]");
                        let kind = match operation {
                            ItemServiceOperationDef::Appraise {} => "appraise",
                            ItemServiceOperationDef::Identify { gold_cost } => {
                                if *gold_cost < 0 {
                                    errors.push(format!(
                                        "{operation_label}.gold_cost must be non-negative"
                                    ));
                                }
                                "identify"
                            }
                            ItemServiceOperationDef::EnchantWeapon {
                                gold_cost,
                                tags,
                                remaining_rounds,
                                ..
                            } => {
                                if *gold_cost < 0 {
                                    errors.push(format!(
                                        "{operation_label}.gold_cost must be non-negative"
                                    ));
                                }
                                if tags.is_empty() || tags.iter().any(|tag| tag.trim().is_empty()) {
                                    errors.push(format!(
                                        "{operation_label}.tags must be a non-empty list of non-empty strings"
                                    ));
                                }
                                let mut sorted = tags.clone();
                                sorted.sort();
                                sorted.dedup();
                                if sorted != *tags {
                                    errors.push(format!(
                                        "{operation_label}.tags must be sorted and unique"
                                    ));
                                }
                                if remaining_rounds.is_some_and(|rounds| rounds == 0) {
                                    errors.push(format!(
                                        "{operation_label}.remaining_rounds must be positive when present"
                                    ));
                                }
                                "enchant_weapon"
                            }
                        };
                        if !operation_kinds.insert(kind) {
                            errors.push(format!(
                                "{operation_label}.kind must be unique within the capability"
                            ));
                        }
                    }
                }
                ServiceCapabilityDef::Restoration { operations, .. } => {
                    if operations.is_empty() {
                        errors.push(format!(
                            "{capability_label}.operations must be a non-empty list"
                        ));
                    }
                    let mut transaction_ids = HashMap::new();
                    for (operation_index, operation) in operations.iter().enumerate() {
                        let operation_label =
                            format!("{capability_label}.operations[{operation_index}]");
                        if let Some(previous) = transaction_ids
                            .insert(operation.transaction.id.as_str(), operation_index)
                        {
                            errors.push(format!(
                                "{operation_label}.transaction.id duplicates {capability_label}.operations[{previous}].transaction.id"
                            ));
                        }
                        bundle.validate_transaction(
                            &format!("{operation_label}.transaction"),
                            &operation.transaction,
                            transactions::TransactionPolicy::Restoration,
                            errors,
                        );
                        if matches!(operation.outcome, RestorationOutcomeDef::PriestResurrection) {
                            let charges_gold =
                                operation
                                    .transaction
                                    .requirements
                                    .iter()
                                    .any(|requirement| {
                                        matches!(
                                            requirement,
                                            TransactionRequirementDef::MinimumCarriedGold { .. }
                                        )
                                    })
                                    || operation.transaction.costs.iter().any(|cost| {
                                        matches!(cost, TransactionCostDef::CarriedGold { .. })
                                    });
                            let charges_item =
                                operation
                                    .transaction
                                    .requirements
                                    .iter()
                                    .any(|requirement| {
                                        matches!(
                                            requirement,
                                            TransactionRequirementDef::CarriedItem { .. }
                                        )
                                    })
                                    || operation.transaction.costs.iter().any(|cost| {
                                        matches!(
                                            cost,
                                            TransactionCostDef::SelectedCarriedItem { .. }
                                        )
                                    });
                            if charges_gold {
                                errors.push(format!(
                                    "{operation_label}.transaction must not charge carried gold for priest_resurrection"
                                ));
                            }
                            if charges_item {
                                errors.push(format!(
                                    "{operation_label}.transaction must not require or consume an item for priest_resurrection"
                                ));
                            }
                        }
                    }
                }
                ServiceCapabilityDef::SkillCritique { .. } => {}
            }
        }
    }
}

fn validate_spell_teaching_definition(
    bundle: &ValidationBundle,
    definition: &ServiceDefinitionDef,
    capability_label: &str,
    training_capability_id: &str,
    teachings: &[SpellTeachingDef],
    errors: &mut Vec<String>,
) {
    let training = definition
        .capabilities
        .iter()
        .find(|candidate| services::capability_id(candidate) == training_capability_id);
    let training_offers = match training {
        Some(ServiceCapabilityDef::SkillTraining { offers, .. }) => Some(offers.as_slice()),
        Some(_) => {
            errors.push(format!(
                "{capability_label}.training_capability_id must reference skill_training in the same service definition"
            ));
            None
        }
        None => {
            errors.push(format!(
                "{capability_label}.training_capability_id does not reference a capability in the same service definition"
            ));
            None
        }
    };
    let magic_offers = training_offers
        .into_iter()
        .flatten()
        .filter(|offer| SPELL_TEACHING_LANES.contains(&offer.track_id.as_str()))
        .collect::<Vec<_>>();
    let teaching_lane = if magic_offers.len() == 1 {
        Some(magic_offers[0].track_id.as_str())
    } else {
        errors.push(format!(
            "{capability_label}.training_capability_id must reference training with exactly one magic-lane offer"
        ));
        None
    };

    let mut taught = HashSet::new();
    for (teaching_index, teaching) in teachings.iter().enumerate() {
        let teaching_label = format!("{capability_label}.teachings[{teaching_index}]");
        if teaching.spell_id.trim().is_empty() {
            errors.push(format!("{teaching_label}.spell_id must be non-empty"));
            continue;
        }
        if !taught.insert(teaching.spell_id.as_str()) {
            errors.push(format!(
                "{teaching_label}.spell_id must be unique within the capability"
            ));
        }
        let Some(spell) = bundle
            .spells
            .iter()
            .find(|spell| spell.id == teaching.spell_id)
        else {
            errors.push(format!(
                "{teaching_label}.spell_id references unknown spell {:?}",
                teaching.spell_id
            ));
            continue;
        };
        if spell.lane.as_deref() == Some("knight_magic") {
            errors.push(format!(
                "{teaching_label} must not teach knight_magic; Knight spells are promotion grants"
            ));
        }
        if teaching_lane.is_some_and(|lane| spell.lane.as_deref() != Some(lane)) {
            errors.push(format!(
                "{teaching_label}.spell_id must match the trainer magic lane"
            ));
        }
        if spell
            .skill_requirement
            .is_none_or(|requirement| requirement <= 0)
        {
            errors.push(format!(
                "{teaching_label}.spell_id must reference a spell with a positive skill_requirement"
            ));
        }
        if spell.mp_cost.is_none_or(|cost| cost <= 0) {
            errors.push(format!(
                "{teaching_label}.spell_id must reference a spell with a positive mp_cost"
            ));
        }
        if spell.acquisition.is_none() {
            errors.push(format!(
                "{teaching_label}.spell_id must reference a spell with acquisition"
            ));
        }
    }
}

fn validate_promotion_definition(
    bundle: &ValidationBundle,
    transaction_label: &str,
    transaction: &TransactionDef,
    summary: &transactions::TransactionSummary,
    errors: &mut Vec<String>,
) {
    if summary.source_class_id.as_deref() != Some("fighter")
        || summary.target_class_id.as_deref() != Some("knight")
    {
        errors.push(format!(
            "{transaction_label} must promote current class fighter to knight"
        ));
    }
    let minimum_level = transaction
        .requirements
        .iter()
        .find_map(|requirement| match requirement {
            TransactionRequirementDef::MinimumLevel { level } => Some(*level),
            _ => None,
        });
    if minimum_level != Some(8) {
        errors.push(format!("{transaction_label} minimum_level must be 8"));
    }
    let karma = transaction
        .requirements
        .iter()
        .find_map(|requirement| match requirement {
            TransactionRequirementDef::ExactKarma { karma_points } => Some(*karma_points),
            _ => None,
        });
    if karma != Some(0) {
        errors.push(format!("{transaction_label} exact_karma must be 0"));
    }
    let empty_position =
        transaction
            .requirements
            .iter()
            .find_map(|requirement| match requirement {
                TransactionRequirementDef::CarriedPositionEmpty { position } => Some(*position),
                _ => None,
            });
    if empty_position != Some(crate::model::CarriedPosition::RightHand) {
        errors.push(format!(
            "{transaction_label} carried_position_empty must use right_hand"
        ));
    }
    let item_reward = transaction.rewards.iter().find_map(|reward| match reward {
        TransactionRewardDef::Item {
            item_definition_id,
            position,
            ..
        } => Some((item_definition_id, *position)),
        _ => None,
    });
    if let Some((item_definition_id, position)) = item_reward {
        if position != crate::model::CarriedPosition::RightHand {
            errors.push(format!(
                "{transaction_label} item reward must use right_hand"
            ));
        }
        if let Some(item) = bundle
            .items
            .iter()
            .find(|item| item.id == *item_definition_id)
        {
            if !item.valid_placements.contains(&ItemPlacementKind::Hand)
                || !item
                    .valid_placements
                    .contains(&ItemPlacementKind::RingFinger)
            {
                errors.push(format!(
                    "{transaction_label} item reward definition must allow hand and ring_finger placement"
                ));
            }
            if !item
                .capability
                .as_ref()
                .and_then(|capability| capability.spell_focus_for.as_ref())
                .is_some_and(|lanes| lanes.iter().any(|lane| lane == "knight_magic"))
            {
                errors.push(format!(
                    "{transaction_label} item reward definition must focus knight_magic"
                ));
            }
        }
    }
    let granted_spell_ids = transaction
        .rewards
        .iter()
        .filter_map(|reward| match reward {
            TransactionRewardDef::Spell { spell_id } => Some(spell_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    if granted_spell_ids.len() != 5 {
        errors.push(format!(
            "{transaction_label} must contain exactly five spell rewards"
        ));
    }
    let mut spell_ids = HashSet::new();
    for (index, spell_id) in granted_spell_ids.iter().enumerate() {
        let label = format!("{transaction_label}.spell_rewards[{index}]");
        if spell_id.trim().is_empty() {
            errors.push(format!("{label} must be non-empty"));
            continue;
        }
        if !spell_ids.insert(spell_id.as_str()) {
            errors.push(format!("{label} must be unique"));
        }
        match bundle.spells.iter().find(|spell| spell.id == **spell_id) {
            Some(spell) if spell.lane.as_deref() != Some("knight_magic") => {
                errors.push(format!("{label} must reference a knight_magic spell"));
            }
            Some(_) => {}
            None => errors.push(format!("{label} is not a spell id")),
        }
    }
}

impl ValidationBundle {
    fn validate_storage(&self, errors: &mut Vec<String>) {
        let mut bank_ids = HashMap::new();
        for (index, bank) in self.storage.banks.iter().enumerate() {
            if bank.id.trim().is_empty() {
                errors.push(format!("storage.banks[{index}].id must be non-empty"));
            } else if let Some(previous) = bank_ids.insert(bank.id.as_str(), index) {
                errors.push(format!(
                    "storage.banks[{index}].id duplicates storage.banks[{previous}].id"
                ));
            }
            if bank.transaction_cap_gold <= 0 {
                errors.push(format!(
                    "storage.banks[{index}].transaction_cap_gold must be positive"
                ));
            }
        }

        let mut vault_ids = HashMap::new();
        for (index, vault) in self.storage.locker_vaults.iter().enumerate() {
            if vault.id.trim().is_empty() {
                errors.push(format!(
                    "storage.locker_vaults[{index}].id must be non-empty"
                ));
            } else if let Some(previous) = vault_ids.insert(vault.id.as_str(), index) {
                errors.push(format!(
                    "storage.locker_vaults[{index}].id duplicates storage.locker_vaults[{previous}].id"
                ));
            }
            if vault.capacity == 0 {
                errors.push(format!(
                    "storage.locker_vaults[{index}].capacity must be positive"
                ));
            }
        }
    }

    fn validate_rules(&self, internal_parity_fixture: bool, errors: &mut Vec<String>) {
        self.rules
            .progression
            .validate(&[], &self.service_definitions, errors);
        self.rules.skills.validate_intrinsic("rules.skills", errors);
        self.rules.combat.validate_intrinsic("rules.combat", errors);

        let magic = &self.rules.magic;
        if magic.warmup.units == 0 {
            errors.push("rules.magic.warmup.units must be positive".to_string());
        }
        let interruption = &magic.damage_interruption;
        if interruption.numerator == 0 {
            errors.push("rules.magic.damage_interruption.numerator must be positive".to_string());
        }
        if interruption.denominator == 0 {
            errors.push("rules.magic.damage_interruption.denominator must be positive".to_string());
        }
        if interruption.numerator >= interruption.denominator {
            errors.push(
                "rules.magic.damage_interruption.numerator must be less than denominator"
                    .to_string(),
            );
        }
        let resistance = &magic.resistance;
        if resistance.denominator == 0 {
            errors.push("rules.magic.resistance.denominator must be positive".to_string());
        }
        for (path, evidence_state) in [
            (
                "rules.magic.warmup.evidence_state",
                magic.warmup.evidence_state,
            ),
            (
                "rules.magic.damage_interruption.evidence_state",
                interruption.evidence_state,
            ),
            (
                "rules.magic.resistance.denominator_evidence_state",
                resistance.denominator_evidence_state,
            ),
            (
                "rules.magic.resistance.resolution_evidence_state",
                resistance.resolution_evidence_state,
            ),
            (
                "rules.magic.casting_practice.evidence_state",
                magic.casting_practice.evidence_state,
            ),
            (
                "rules.magic.thaum_above_skill.evidence_state",
                magic.thaum_above_skill.evidence_state,
            ),
            (
                "rules.magic.kill_experience.fraction_evidence_state",
                magic.kill_experience.fraction_evidence_state,
            ),
            (
                "rules.magic.kill_experience.rounding_evidence_state",
                magic.kill_experience.rounding_evidence_state,
            ),
            (
                "rules.magic.mp_recovery.evidence_state",
                magic.mp_recovery.evidence_state,
            ),
            (
                "rules.magic.effect_families.raise_dead.evidence_state",
                magic.effect_families.raise_dead.evidence_state,
            ),
        ] {
            if self.clean_content
                && evidence_state != MagicRuleEvidenceStateDef::OriginalProvisional
            {
                errors.push(format!(
                    "{path} must be original_provisional in clean content"
                ));
            }
            if evidence_state == MagicRuleEvidenceStateDef::TargetRelease
                && !internal_parity_fixture
            {
                errors.push(format!(
                    "{path} target_release is allowed only in a marked internal parity fixture"
                ));
            }
        }
        if resistance.resolution_evidence_state != MagicRuleEvidenceStateDef::OriginalProvisional {
            errors.push(
                "rules.magic.resistance.resolution_evidence_state must be original_provisional"
                    .to_string(),
            );
        }
        let practice = &magic.casting_practice;
        if practice.minimum_raw_points == 0 {
            errors.push(
                "rules.magic.casting_practice.minimum_raw_points must be positive".to_string(),
            );
        }
        if practice.raw_points_per_mp == 0 {
            errors.push(
                "rules.magic.casting_practice.raw_points_per_mp must be positive".to_string(),
            );
        }
        if practice.primary_attribute_points_per_bonus == 0 {
            errors.push(
                "rules.magic.casting_practice.primary_attribute_points_per_bonus must be positive"
                    .to_string(),
            );
        }
        if practice.primary_attribute_points_per_bonus > 0 {
            let maximum_attribute_bonus = u64::try_from(i32::MAX).expect("i32::MAX fits u64")
                / u64::from(practice.primary_attribute_points_per_bonus);
            let maximum_mp = u64::try_from(i32::MAX).expect("i32::MAX fits u64");
            let maximum_base = maximum_mp
                .checked_mul(practice.raw_points_per_mp)
                .map(|scaled| scaled.max(practice.minimum_raw_points));
            if maximum_base
                .and_then(|base| base.checked_add(maximum_attribute_bonus))
                .is_none()
            {
                errors.push(
                    "rules.magic.casting_practice arithmetic exceeds supported range".to_string(),
                );
            }
        }
        if practice.evidence_state != MagicRuleEvidenceStateDef::OriginalProvisional {
            errors.push(
                "rules.magic.casting_practice.evidence_state must be original_provisional"
                    .to_string(),
            );
        }
        let attempt = &magic.thaum_above_skill;
        if attempt.roll_denominator == 0 {
            errors.push(
                "rules.magic.thaum_above_skill.roll_denominator must be positive".to_string(),
            );
        }
        if attempt.penalty_per_missing_level == 0 {
            errors.push(
                "rules.magic.thaum_above_skill.penalty_per_missing_level must be positive"
                    .to_string(),
            );
        }
        if u32::from(MAX_SKILL_LEVEL)
            .checked_mul(attempt.penalty_per_missing_level)
            .is_none()
        {
            errors.push(
                "rules.magic.thaum_above_skill maximum gap arithmetic exceeds supported range"
                    .to_string(),
            );
        }
        if attempt.minimum_success_threshold == 0
            || attempt.minimum_success_threshold > attempt.roll_denominator
        {
            errors.push(
                "rules.magic.thaum_above_skill.minimum_success_threshold must be in 1..=roll_denominator"
                    .to_string(),
            );
        }
        if attempt.evidence_state != MagicRuleEvidenceStateDef::OriginalProvisional {
            errors.push(
                "rules.magic.thaum_above_skill.evidence_state must be original_provisional"
                    .to_string(),
            );
        }
        for (name, fraction) in [
            ("directed", &magic.kill_experience.directed),
            ("area_or_illusion", &magic.kill_experience.area_or_illusion),
        ] {
            if fraction.numerator == 0 || fraction.denominator == 0 {
                errors.push(format!(
                    "rules.magic.kill_experience.{name} numerator and denominator must be positive"
                ));
            } else {
                if fraction.numerator > fraction.denominator {
                    errors.push(format!(
                        "rules.magic.kill_experience.{name} must not exceed one"
                    ));
                }
                if gcd_u32(fraction.numerator, fraction.denominator) != 1 {
                    errors.push(format!(
                        "rules.magic.kill_experience.{name} must be reduced"
                    ));
                }
            }
        }
        if magic.kill_experience.rounding_evidence_state
            != MagicRuleEvidenceStateDef::OriginalProvisional
        {
            errors.push(
                "rules.magic.kill_experience.rounding_evidence_state must be original_provisional"
                    .to_string(),
            );
        }
        if magic.mp_recovery.evidence_state != MagicRuleEvidenceStateDef::OriginalProvisional {
            errors.push(
                "rules.magic.mp_recovery.evidence_state must be original_provisional".to_string(),
            );
        }
        let raise_dead = &magic.effect_families.raise_dead;
        if raise_dead.roll_denominator == 0 {
            errors.push(
                "rules.magic.effect_families.raise_dead.roll_denominator must be positive"
                    .to_string(),
            );
        }
        if raise_dead.success_threshold_per_magic_level == 0 {
            errors.push(
                "rules.magic.effect_families.raise_dead.success_threshold_per_magic_level must be positive"
                    .to_string(),
            );
        }
        if raise_dead.minimum_success_threshold == 0
            || raise_dead.minimum_success_threshold > raise_dead.roll_denominator
        {
            errors.push(
                "rules.magic.effect_families.raise_dead.minimum_success_threshold must be in 1..=roll_denominator"
                    .to_string(),
            );
        }
        if u32::from(MAX_SKILL_LEVEL)
            .checked_mul(raise_dead.success_threshold_per_magic_level)
            .is_none()
        {
            errors.push(
                "rules.magic.effect_families.raise_dead arithmetic exceeds supported range"
                    .to_string(),
            );
        }
        if raise_dead.evidence_state != MagicRuleEvidenceStateDef::OriginalProvisional {
            errors.push(
                "rules.magic.effect_families.raise_dead.evidence_state must be original_provisional"
                    .to_string(),
            );
        }
        for (index, actor) in self.actor_definitions.iter().enumerate() {
            let mut traits = HashSet::new();
            for trait_value in &actor.creature_traits {
                if !traits.insert(*trait_value) {
                    errors.push(format!(
                        "actor_definitions[{index}].creature_traits must be unique"
                    ));
                }
            }
            if actor.magic_resistance.natural_save_twentieths > resistance.denominator {
                errors.push(format!(
                    "actor_definitions[{index}].magic_resistance.natural_save_twentieths must not exceed rules.magic.resistance.denominator"
                ));
            }
            if actor.magic_resistance.evidence_state
                != crate::model::MagicRuleEvidenceState::OriginalProvisional
            {
                errors.push(format!(
                    "actor_definitions[{index}].magic_resistance.evidence_state must be original_provisional"
                ));
            }
        }
        if self.rules.movement.controlled_path_points <= 0 {
            errors.push("rules.movement.controlled_path_points must be positive".to_string());
        }
        if self.rules.movement.automatic_step_points <= 0 {
            errors.push("rules.movement.automatic_step_points must be positive".to_string());
        }
        let burden = &self.rules.burden;
        if burden.coin_burden_per_gold == 0 {
            errors.push("rules.burden.coin_burden_per_gold must be positive".to_string());
        }
        if burden.lightly_loaded_max_per_strength == 0 {
            errors
                .push("rules.burden.lightly_loaded_max_per_strength must be positive".to_string());
        }
        if burden.lightly_loaded_max_per_strength >= burden.moderately_loaded_max_per_strength {
            errors.push(
                "rules.burden.moderately_loaded_max_per_strength must be greater than lightly_loaded_max_per_strength"
                    .to_string(),
            );
        }
        if burden.moderately_loaded_max_per_strength >= burden.heavily_loaded_max_per_strength {
            errors.push(
                "rules.burden.heavily_loaded_max_per_strength must be greater than moderately_loaded_max_per_strength"
                    .to_string(),
            );
        }

        let resources = &self.rules.resources;
        for (name, value) in [
            ("active_hp_recovery", resources.active_hp_recovery),
            ("inactive_hp_recovery", resources.inactive_hp_recovery),
            (
                "inactive_stamina_recovery",
                resources.inactive_stamina_recovery,
            ),
            ("mp_recovery", resources.mp_recovery),
            (
                "normal_movement_stamina_cost",
                resources.normal_movement_stamina_cost,
            ),
            (
                "rapid_movement_stamina_cost",
                resources.rapid_movement_stamina_cost,
            ),
        ] {
            if value <= 0 {
                errors.push(format!("rules.resources.{name} must be positive"));
            }
        }
        if resources.recovery_interval_units == 0 {
            errors.push("rules.resources.recovery_interval_units must be positive".to_string());
        }
        if resources.inactive_hp_recovery <= resources.active_hp_recovery {
            errors.push(
                "rules.resources.inactive_hp_recovery must be greater than active_hp_recovery"
                    .to_string(),
            );
        }
        if resources.rapid_movement_stamina_cost <= resources.normal_movement_stamina_cost {
            errors.push(
                "rules.resources.rapid_movement_stamina_cost must be greater than normal_movement_stamina_cost"
                    .to_string(),
            );
        }
    }

    fn validate_items(&self, errors: &mut Vec<String>) {
        let mut ids = HashMap::new();
        for (index, item) in self.items.iter().enumerate() {
            if item.id.trim().is_empty() {
                errors.push(format!("items[{index}].id must be non-empty"));
            }
            if let Some(previous) = ids.insert(item.id.clone(), index) {
                errors.push(format!("items[{index}].id duplicates items[{previous}].id"));
            }
            if item.name.trim().is_empty() {
                errors.push(format!("items[{index}].name must be non-empty"));
            }
            if item.kind.trim().is_empty() {
                errors.push(format!("items[{index}].kind must be non-empty"));
            }
            if item.kind == "currency" {
                errors.push(format!(
                    "items[{index}].kind must not be currency; use carried.gold"
                ));
            }
            if item.valid_placements.is_empty() {
                errors.push(format!("items[{index}].valid_placements must not be empty"));
            }
            let mut placements = std::collections::HashSet::new();
            for placement in &item.valid_placements {
                if !placements.insert(*placement) {
                    errors.push(format!(
                        "items[{index}].valid_placements contains duplicate {placement:?}"
                    ));
                }
            }
            if item.kind == "weapon" {
                match item.weapon.as_ref() {
                    Some(weapon) => validate_weapon_definition(
                        weapon,
                        &format!("items[{index}].weapon"),
                        errors,
                    ),
                    None => {
                        errors.push(format!("items[{index}].weapon must be present for weapons"))
                    }
                }
            } else if item.weapon.is_some() {
                errors.push(format!("items[{index}].weapon is only valid for weapons"));
            }

            if let Some(armor) = item.armor.as_ref() {
                if matches!(item.kind.as_str(), "weapon" | "consumable") {
                    errors.push(format!(
                        "items[{index}].armor is invalid for {} items",
                        item.kind
                    ));
                }
                validate_armor_definition(
                    armor,
                    &item.valid_placements,
                    &format!("items[{index}].armor"),
                    errors,
                );
            }

            if item.kind == "consumable" {
                match &item.consumable {
                    Some(consumable) => {
                        if consumable.effect != "healing" {
                            errors
                                .push(format!("items[{index}].consumable.effect must be healing"));
                        }
                        if consumable.heal_per_round <= 0 {
                            errors.push(format!(
                                "items[{index}].consumable.heal_per_round must be positive"
                            ));
                        }
                    }
                    None => {
                        errors.push(format!(
                            "items[{index}].consumable must be present for consumables"
                        ));
                    }
                }
            } else if item.consumable.is_some() {
                errors.push(format!(
                    "items[{index}].consumable is only valid for consumables"
                ));
            }

            self.validate_item_capability(item, index, errors);
        }
    }

    fn validate_item_definition_reference<'a>(
        &'a self,
        label: &str,
        item_definition_id: &str,
        errors: &mut Vec<String>,
    ) -> Option<&'a ItemDef> {
        if item_definition_id.trim().is_empty() {
            return None;
        }
        let definition = self.items.iter().find(|item| item.id == item_definition_id);
        if definition.is_none() {
            errors.push(format!(
                "{label} references unknown item definition {item_definition_id:?}"
            ));
        }
        definition
    }

    fn validate_item_instance_definition_and_stack_value(
        &self,
        label: &str,
        instance: &ItemInstanceSeedDef,
        errors: &mut Vec<String>,
    ) {
        if !matches!(instance.binding, ItemBindingState::Unrestricted) && instance.quantity != 1 {
            errors.push(format!(
                "{label}.quantity must be 1 for a tied item instance"
            ));
        }
        if let ItemBindingState::Bound { character_id } = &instance.binding
            && character_id.as_str().trim().is_empty()
        {
            errors.push(format!("{label}.binding.character_id must be non-empty"));
        }
        let definition_label = format!("{label}.definition_id");
        if instance.definition_id.trim().is_empty() {
            errors.push(format!("{definition_label} must be non-empty"));
            return;
        }
        let Some(definition) = self.validate_item_definition_reference(
            &definition_label,
            &instance.definition_id,
            errors,
        ) else {
            return;
        };
        if definition
            .capability
            .as_ref()
            .and_then(|capability| capability.spell_book_for.as_ref())
            .is_some()
        {
            if instance.quantity != 1 {
                errors.push(format!("{label}.quantity must be 1 for a Spell Book"));
            }
            match &instance.binding {
                ItemBindingState::Bound { .. } => {}
                ItemBindingState::Unrestricted | ItemBindingState::BindOnFirstCharacterTouch => {
                    errors.push(format!("{label}.binding must be bound for a Spell Book"));
                }
            }
        }
        let unit_value_gold = definition.economy.unit_value_gold;
        if unit_value_gold.is_some_and(|unit_value_gold| {
            unit_value_gold
                .checked_mul(u64::from(instance.quantity))
                .is_none()
        }) {
            errors.push(format!(
                "{label}.quantity * unit_value_gold must be <= {}",
                u64::MAX
            ));
        }
        if definition
            .economy
            .unit_burden
            .checked_mul(u64::from(instance.quantity))
            .is_none()
        {
            errors.push(format!(
                "{label}.quantity * unit_burden must be <= {}",
                u64::MAX
            ));
        }
    }

    fn validate_item_instance_reference(
        registry: &BTreeMap<String, ItemInstanceSeedDef>,
        owners: &mut HashMap<String, String>,
        instance_id: &str,
        label: &str,
        errors: &mut Vec<String>,
    ) {
        if instance_id.trim().is_empty() {
            errors.push(format!("{label} must be non-empty"));
            return;
        }
        if !registry.contains_key(instance_id) {
            errors.push(format!(
                "{label} references unknown item instance {instance_id:?}"
            ));
            return;
        }
        if let Some(previous) = owners.insert(instance_id.to_string(), label.to_string()) {
            errors.push(format!(
                "item instance {instance_id:?} is referenced more than once ({previous} and {label})"
            ));
        }
    }

    fn validate_positioned_item(
        &self,
        registry: &BTreeMap<String, ItemInstanceSeedDef>,
        instance_id: &str,
        position: crate::model::CarriedPosition,
        _current_class_id: Option<&str>,
        label: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(instance) = registry.get(instance_id) else {
            return;
        };
        let Some(definition) = self
            .items
            .iter()
            .find(|definition| definition.id == instance.definition_id)
        else {
            return;
        };
        if !definition
            .valid_placements
            .contains(&position.placement_kind())
        {
            errors.push(format!(
                "{label} cannot occupy carried position {:?}",
                position.label()
            ));
        }
        if !position.is_sack_item() && instance.quantity != 1 {
            errors.push(format!("{label} must have quantity 1 outside the sack"));
        }
    }

    fn validate_summon_templates(&self, errors: &mut Vec<String>) {
        let mut template_ids = HashMap::new();

        for (index, template) in self.summon_templates.iter().enumerate() {
            let prefix = format!("summon_templates[{index}]");
            if template.id.trim().is_empty() {
                errors.push(format!("{prefix}.id must be non-empty"));
            } else {
                if let Some(previous) = template_ids.insert(template.id.clone(), index) {
                    errors.push(format!(
                        "{prefix}.id duplicates summon_templates[{previous}].id"
                    ));
                }
            }
            match self
                .actor_definitions
                .iter()
                .find(|definition| definition.id == template.actor_definition_id)
            {
                Some(definition)
                    if definition.kind == ActorKind::Monster && definition.ai.is_some() => {}
                Some(_) => errors.push(format!(
                    "{prefix}.actor_definition_id must reference a monster definition with AI"
                )),
                None => errors.push(format!(
                    "{prefix}.actor_definition_id references unknown selected actor definition"
                )),
            }

            let mut local_owners = HashMap::new();
            if template.carried.gold.left_hand < 0
                || template.carried.gold.right_hand < 0
                || template.carried.gold.sack < 0
            {
                errors.push(format!("{prefix}.carried.gold values must be non-negative"));
            }
            if template.carried.gold.checked_total().is_none() {
                errors.push(format!(
                    "{prefix}.carried.gold total must fit a signed 64-bit integer"
                ));
            }
            for (instance_id, instance) in &template.item_instances {
                if instance_id.trim().is_empty() {
                    errors.push(format!("{prefix}.item_instances keys must be non-empty"));
                }
                self.validate_item_instance_definition_and_stack_value(
                    &format!("{prefix}.item_instances[{instance_id:?}]"),
                    instance,
                    errors,
                );
                if instance.quantity == 0 {
                    errors.push(format!(
                        "{prefix}.item_instances[{instance_id:?}].quantity must be positive"
                    ));
                }
            }
            let mut positions = HashMap::new();
            for (item_index, positioned) in template.carried.items.iter().enumerate() {
                let label = format!("{prefix}.carried.items[{item_index}].item_instance_id");
                Self::validate_item_instance_reference(
                    &template.item_instances,
                    &mut local_owners,
                    &positioned.item_instance_id,
                    &label,
                    errors,
                );
                if let Some(previous) = positions.insert(positioned.position, item_index) {
                    errors.push(format!(
                        "{prefix}.carried.items[{item_index}].position duplicates {prefix}.carried.items[{previous}].position"
                    ));
                }
                self.validate_positioned_item(
                    &template.item_instances,
                    &positioned.item_instance_id,
                    positioned.position,
                    None,
                    &label,
                    errors,
                );
            }
            for (position, amount) in [
                (
                    crate::model::CarriedPosition::LeftHand,
                    template.carried.gold.left_hand,
                ),
                (
                    crate::model::CarriedPosition::RightHand,
                    template.carried.gold.right_hand,
                ),
            ] {
                if amount > 0 && positions.contains_key(&position) {
                    errors.push(format!(
                        "{prefix}.carried cannot place an item and gold in {}",
                        position.label()
                    ));
                }
            }
            for instance_id in template.item_instances.keys() {
                if !local_owners.contains_key(instance_id) {
                    errors.push(format!(
                        "{prefix} item instance {instance_id:?} has no owner or location"
                    ));
                }
            }

            self.validate_active_effect_entries(
                &format!("{prefix}.active_effects"),
                &template.active_effects,
                errors,
            );
        }
    }

    fn validate_actor_definitions(&self, errors: &mut Vec<String>) {
        for (id, profile) in &self.scavenging_profiles {
            let prefix = format!("scavenging_profiles[{id:?}]");
            if profile.search_radius > 6 {
                errors.push(format!("{prefix}.search_radius must be at most 6"));
            }
            if !(1..=100).contains(&profile.balm_below_hp_percent) {
                errors.push(format!(
                    "{prefix}.balm_below_hp_percent must be between 1 and 100"
                ));
            }
            if !(1..=100).contains(&profile.balm_chance_denominator)
                || profile.balm_chance_numerator > profile.balm_chance_denominator
            {
                errors.push(format!("{prefix}.balm_chance must be a valid fraction"));
            }
            if profile.uses_healing_balm
                && (!profile.collects_ground_items || !profile.equips_items)
            {
                errors.push(format!(
                    "{prefix}.uses_healing_balm requires ground collection and equipping"
                ));
            }
            if (profile.searches_corpses || profile.collects_ground_items || profile.collects_gold)
                && profile.search_radius == 0
            {
                errors.push(format!(
                    "{prefix}.search_radius must be nonzero when scavenging is enabled"
                ));
            }
        }
        let mut profile_ids = HashSet::new();
        for (index, profile) in self.physical_damage_affinity_profiles.iter().enumerate() {
            let prefix = format!("physical_damage_affinity_profiles[{index}]");
            if profile.id.trim().is_empty() {
                errors.push(format!("{prefix}.id must be non-empty"));
            } else if !profile_ids.insert(profile.id.as_str()) {
                errors.push(format!("{prefix}.id must be unique"));
            }
            let mut kinds = HashSet::new();
            for (response_index, response) in profile.responses.iter().enumerate() {
                let label = format!("{prefix}.responses[{response_index}]");
                if !kinds.insert(response.damage_kind) {
                    errors.push(format!("{label}.damage_kind must be unique"));
                }
                if response.denominator == 0 {
                    errors.push(format!("{label}.denominator must be positive"));
                }
            }
            for kind in [
                crate::model::PhysicalDamageKind::Cutting,
                crate::model::PhysicalDamageKind::Piercing,
                crate::model::PhysicalDamageKind::Crushing,
            ] {
                if !kinds.contains(&kind) {
                    errors.push(format!("{prefix}.responses is missing {}", kind.label()));
                }
            }
            if profile.responses.len() != 3 {
                errors.push(format!(
                    "{prefix}.responses must contain exactly three rows"
                ));
            }
        }

        let mut ids = HashSet::new();
        for (index, definition) in self.actor_definitions.iter().enumerate() {
            let prefix = format!("actor_definitions[{index}]");
            if definition.id.trim().is_empty() {
                errors.push(format!("{prefix}.id must be non-empty"));
            } else if !ids.insert(definition.id.as_str()) {
                errors.push(format!("{prefix}.id must be unique"));
            }
            if definition.name.trim().is_empty() {
                errors.push(format!("{prefix}.name must be non-empty"));
            }
            if definition.stats.hp <= 0
                || definition.stats.attack < 0
                || definition.stats.defense < 0
            {
                errors.push(format!(
                    "{prefix}.stats must use positive HP and non-negative attack/defense"
                ));
            }
            if definition.kind == ActorKind::Player
                && definition.death.remains != crate::model::CorpseDisposition::SearchableCorpse
            {
                errors.push(format!(
                    "{prefix}.death.remains must be searchable_corpse for players"
                ));
            }
            let is_summon_definition = self
                .summon_templates
                .iter()
                .any(|template| template.actor_definition_id == definition.id);
            let character_aligned = matches!(
                definition.social.alignment_source,
                SocialAlignmentSourceDef::Character {}
            );
            validate_actor_social_profile(
                &definition.social,
                definition.kind,
                character_aligned,
                definition.ai.is_some(),
                is_summon_definition,
                &prefix,
                errors,
            );
            match (definition.kind, definition.ai.as_ref()) {
                (ActorKind::Player, Some(_)) => {
                    errors.push(format!("{prefix}.ai is forbidden for players"));
                }
                (ActorKind::Monster, None) => {
                    errors.push(format!("{prefix}.ai is required for monsters"));
                }
                (_, Some(ai)) => validate_actor_ai_definition(ai, &format!("{prefix}.ai"), errors),
                _ => {}
            }
            if definition.xp_value.is_some_and(|value| value < 0) {
                errors.push(format!("{prefix}.xp_value must be non-negative"));
            }
            if definition.xp_value.is_some() && definition.kind != ActorKind::Monster {
                errors.push(format!("{prefix}.xp_value is only valid for monsters"));
            }
            if !profile_ids.contains(definition.physical_damage_affinity_profile_id.as_str()) {
                errors.push(format!(
                    "{prefix}.physical_damage_affinity_profile_id references unknown selected profile"
                ));
            }
            if let Some(profile_id) = definition.scavenging_profile_id.as_deref()
                && !self
                    .scavenging_profiles
                    .keys()
                    .any(|candidate| candidate.as_str() == profile_id)
            {
                errors.push(format!(
                    "{prefix}.scavenging_profile_id references unknown selected profile"
                ));
            }
            self.validate_monster_abilities(
                &definition.monster_abilities,
                definition.kind == ActorKind::Monster,
                &prefix,
                errors,
            );
        }
    }

    fn validate_ecology_definitions(&self, errors: &mut Vec<String>) {
        let actor_kinds = self
            .actor_definitions
            .iter()
            .map(|row| (row.id.as_str(), row.kind))
            .collect::<HashMap<_, _>>();
        let item_by_id = self
            .items
            .iter()
            .map(|row| (row.id.as_str(), row))
            .collect::<HashMap<_, _>>();
        let mut loot_ids = HashSet::new();
        for (index, table) in self.loot_tables.iter().enumerate() {
            let prefix = format!("loot_tables[{index}]");
            if table.id().trim().is_empty() || !loot_ids.insert(table.id()) {
                errors.push(format!("{prefix}.id must be non-empty and unique"));
            }
            if table.entries().is_empty() {
                errors.push(format!("{prefix}.entries must be non-empty"));
            }
            match (table.family, table.maximum_non_gold_drops()) {
                (LootTableFamilyDef::Ordinary, None) => errors.push(format!(
                    "{prefix}.maximum_non_gold_drops is required for an ordinary table"
                )),
                (LootTableFamilyDef::Ordinary, Some(cap)) if !(1..=2).contains(&cap) => {
                    errors.push(format!(
                        "{prefix}.maximum_non_gold_drops must be within 1..=2"
                    ));
                }
                (LootTableFamilyDef::Signature, Some(_)) => errors.push(format!(
                    "{prefix}.maximum_non_gold_drops is forbidden for a signature table"
                )),
                _ => {}
            }
            let maximum_possible_non_gold = table
                .entries()
                .iter()
                .filter(|entry| !matches!(entry, LootEntryDef::Gold { .. }))
                .count();
            if table.is_signature() && maximum_possible_non_gold > 3 {
                errors.push(format!(
                    "{prefix} signature table may select at most three non-gold results"
                ));
            }
            let mut entry_ids = HashSet::new();
            let mut positions = HashSet::new();
            for (entry_index, entry) in table.entries().iter().enumerate() {
                let label = format!("{prefix}.entries[{entry_index}]");
                if entry.id().trim().is_empty() || !entry_ids.insert(entry.id()) {
                    errors.push(format!("{label}.id must be non-empty and unique"));
                }
                let (chance_numerator, chance_denominator) = entry.chance();
                if chance_numerator == 0
                    || chance_denominator == 0
                    || chance_numerator > chance_denominator
                {
                    errors.push(format!("{label}.chance must be within 1..=denominator"));
                }
                let independent_positions = match entry {
                    LootEntryDef::Item {
                        item_definition_id,
                        quantity,
                        position,
                        ..
                    } => {
                        let selected_item = item_by_id.get(item_definition_id.as_str());
                        if selected_item.is_none() {
                            errors.push(format!("{label}.item_definition_id is not selected"));
                        } else if selected_item.is_some_and(|item| {
                            !item.valid_placements.contains(&position.placement_kind())
                        }) {
                            errors.push(format!(
                                "{label}.position is not valid for the selected item definition"
                            ));
                        }
                        if *quantity == 0 {
                            errors.push(format!("{label}.quantity must be positive"));
                        }
                        HashSet::from([position.label()])
                    }
                    LootEntryDef::ItemChoice { members, .. } => {
                        if members.len() < 2 {
                            errors.push(format!("{label}.members must contain at least two rows"));
                        }
                        let mut member_ids = HashSet::new();
                        let mut member_definitions = HashSet::new();
                        let mut member_positions = HashSet::new();
                        for (member_index, member) in members.iter().enumerate() {
                            let member_label = format!("{label}.members[{member_index}]");
                            if member.member_id.trim().is_empty()
                                || !member_ids.insert(member.member_id.as_str())
                            {
                                errors.push(format!(
                                    "{member_label}.member_id must be non-empty and unique"
                                ));
                            }
                            if !member_definitions.insert(member.item_definition_id.as_str()) {
                                errors.push(format!(
                                    "{member_label}.item_definition_id must be unique within its choice group"
                                ));
                            }
                            let selected_item = item_by_id.get(member.item_definition_id.as_str());
                            if selected_item.is_none() {
                                errors.push(format!(
                                    "{member_label}.item_definition_id is not selected"
                                ));
                            } else if selected_item.is_some_and(|item| {
                                !item
                                    .valid_placements
                                    .contains(&member.position.placement_kind())
                            }) {
                                errors.push(format!(
                                    "{member_label}.position is not valid for the selected item definition"
                                ));
                            }
                            if member.quantity == 0 {
                                errors.push(format!("{member_label}.quantity must be positive"));
                            }
                            member_positions.insert(member.position.label());
                        }
                        member_positions
                    }
                    LootEntryDef::Gold {
                        minimum_amount,
                        maximum_amount,
                        position,
                        ..
                    } => {
                        let valid_range = *minimum_amount > 0
                            && maximum_amount >= minimum_amount
                            && maximum_amount
                                .checked_sub(*minimum_amount)
                                .and_then(|difference| difference.checked_add(1))
                                .is_some_and(|span| u32::try_from(span).is_ok());
                        if !valid_range {
                            errors.push(format!(
                                "{label}.gold range must be positive, ordered, and bounded"
                            ));
                        }
                        HashSet::from([position.label()])
                    }
                };
                if independent_positions
                    .iter()
                    .any(|position| positions.contains(position))
                {
                    errors.push(format!(
                        "{label} duplicates a possible carried position from an independent outcome"
                    ));
                }
                positions.extend(independent_positions);
            }
        }

        let mut group_ids = HashSet::new();
        for (index, group) in self.spawn_groups.iter().enumerate() {
            let prefix = format!("spawn_groups[{index}]");
            if group.id.trim().is_empty() || !group_ids.insert(group.id.as_str()) {
                errors.push(format!("{prefix}.id must be non-empty and unique"));
            }
            let valid_count = match group.ecology_kind {
                EcologyKindDef::Solitary => group.members.len() == 1,
                EcologyKindDef::Pack => group.members.len() >= 2,
                EcologyKindDef::Lair => !group.members.is_empty(),
            };
            if !valid_count {
                errors.push(format!(
                    "{prefix}.members cardinality does not match ecology_kind"
                ));
            }
            let mut member_ids = HashSet::new();
            for (member_index, member) in group.members.iter().enumerate() {
                let label = format!("{prefix}.members[{member_index}]");
                if member.member_id.trim().is_empty()
                    || !member_ids.insert(member.member_id.as_str())
                {
                    errors.push(format!("{label}.member_id must be non-empty and unique"));
                }
                if actor_kinds.get(member.actor_definition_id.as_str()) != Some(&ActorKind::Monster)
                {
                    errors.push(format!(
                        "{label}.actor_definition_id must reference a selected monster"
                    ));
                }
                if member
                    .loot_table_id
                    .as_ref()
                    .is_some_and(|id| !loot_ids.contains(id.as_str()))
                {
                    errors.push(format!("{label}.loot_table_id is not selected"));
                }
            }
            match group.reset {
                SpawnResetDef::FullSite { delay_units: 0 } => {
                    errors.push(format!("{prefix}.reset.delay_units must be positive"));
                }
                SpawnResetDef::FullSite { .. } => {}
                SpawnResetDef::SlotReplenishment {
                    slot_delay_units,
                    full_clear_delay_units,
                } => {
                    if slot_delay_units == 0 {
                        errors.push(format!("{prefix}.reset.slot_delay_units must be positive"));
                    }
                    if full_clear_delay_units == 0 {
                        errors.push(format!(
                            "{prefix}.reset.full_clear_delay_units must be positive"
                        ));
                    }
                    if full_clear_delay_units <= slot_delay_units {
                        errors.push(format!(
                            "{prefix}.reset.full_clear_delay_units must exceed slot_delay_units"
                        ));
                    }
                }
            }
        }

        let group_facts = self
            .spawn_groups
            .iter()
            .map(|group| (group.id.as_str(), (group.ecology_kind, group.reset)))
            .collect::<HashMap<_, _>>();
        let mut lair_ids = HashSet::new();
        for (index, lair) in self.lair_definitions.iter().enumerate() {
            let prefix = format!("lair_definitions[{index}]");
            if lair.id.trim().is_empty() || !lair_ids.insert(lair.id.as_str()) {
                errors.push(format!("{prefix}.id must be non-empty and unique"));
            }
            if lair.name.trim().is_empty() {
                errors.push(format!("{prefix}.name must be non-empty"));
            }
            if group_facts
                .get(lair.spawn_group_id.as_str())
                .map(|(kind, _)| kind)
                != Some(&EcologyKindDef::Lair)
            {
                errors.push(format!(
                    "{prefix}.spawn_group_id must reference a selected lair group"
                ));
            } else if group_facts
                .get(lair.spawn_group_id.as_str())
                .is_some_and(|(_, reset)| !matches!(reset, SpawnResetDef::FullSite { .. }))
            {
                errors.push(format!(
                    "{prefix}.spawn_group_id must reference a full-site reset group"
                ));
            }
        }
    }

    fn validate_monster_abilities(
        &self,
        abilities: &MonsterAbilityList,
        owner_is_monster: bool,
        prefix: &str,
        errors: &mut Vec<String>,
    ) {
        if !abilities.is_empty() && !owner_is_monster {
            errors.push(format!(
                "{prefix}.monster_abilities is only valid for monsters"
            ));
        }

        if abilities.is_empty() {
            return;
        }

        let spell_by_id: HashMap<&str, &SpellDef> = self
            .spells
            .iter()
            .map(|spell| (spell.id.as_str(), spell))
            .collect();
        let mut ids = HashMap::new();

        for (index, ability) in abilities.iter().enumerate() {
            let label = format!("{prefix}.monster_abilities[{index}]");

            if ability.id.trim().is_empty() {
                errors.push(format!("{label}.id must be non-empty"));
            } else if let Some(previous) = ids.insert(ability.id.as_str(), index) {
                errors.push(format!(
                    "{label}.id duplicates {prefix}.monster_abilities[{previous}].id"
                ));
            }

            if !MONSTER_ABILITY_KINDS.contains(&ability.kind.as_str()) {
                errors.push(format!(
                    "{label}.kind must be one of {}",
                    MONSTER_ABILITY_KINDS.join(", ")
                ));
            }

            if ability.spell_id.trim().is_empty() {
                errors.push(format!("{label}.spell_id must be non-empty"));
            } else if let Some(spell) = spell_by_id.get(ability.spell_id.as_str()) {
                let target_kind = spell.target.as_ref().map(|target| target.kind);
                match target_kind {
                    Some(kind) if monster_supports_target(kind) => {}
                    _ => errors.push(format!(
                        "{label}.spell_id references a spell with unsupported monster target kind"
                    )),
                }

                let effect_family = spell.effect.as_ref().map(|effect| effect.family);
                match effect_family {
                    Some(family) if monster_supports_effect(family) => {}
                    _ => errors.push(format!(
                        "{label}.spell_id references a spell with unsupported monster effect family"
                    )),
                }
                let resolved_target_kind = if ability.target_policy.as_deref() == Some("self") {
                    Some(SpellTargetKind::SelfTarget)
                } else {
                    target_kind
                };
                if let (Some(family), Some(kind)) = (effect_family, resolved_target_kind)
                    && monster_supports_effect(family)
                    && target_kind.is_some_and(monster_supports_target)
                    && monster_supports_target(kind)
                    && !monster_effect_supports_target(family, kind)
                {
                    errors.push(format!(
                        "{label}.spell_id references an unsupported monster effect/target combination"
                    ));
                }
                if spell.casting.as_ref().map(|casting| casting.method)
                    != Some(SpellCastingMethod::Direct)
                {
                    errors.push(format!(
                        "{label}.spell_id must reference a direct-cast spell"
                    ));
                }
            } else {
                errors.push(format!("{label}.spell_id references unknown spell"));
            }

            if ability.cooldown_rounds < 1 {
                errors.push(format!("{label}.cooldown_rounds must be >= 1"));
            }

            if let Some(target_policy) = ability.target_policy.as_deref()
                && !MONSTER_ABILITY_TARGET_POLICIES.contains(&target_policy)
            {
                errors.push(format!(
                    "{label}.target_policy must be one of {}",
                    MONSTER_ABILITY_TARGET_POLICIES.join(", ")
                ));
            }
        }
    }

    fn validate_item_capability(&self, item: &ItemDef, index: usize, errors: &mut Vec<String>) {
        let cap = match &item.capability {
            Some(c) => c,
            None => return,
        };

        let label = format!("items[{index}].capability");

        if item.kind == "consumable" && cap.block_value.is_some() {
            errors.push(format!(
                "{label}.block_value is invalid for consumable items"
            ));
        }

        if let Some(bv) = cap.block_value
            && bv < 0
        {
            errors.push(format!("{label}.block_value must be >= 0, got {bv}"));
        }

        if let Some(ref taxonomy) = cap.taxonomy_id
            && taxonomy.is_empty()
        {
            errors.push(format!("{label}.taxonomy_id must not be empty"));
        }

        if let Some(training_tracks) = &cap.training_focus_for {
            if training_tracks.is_empty() {
                errors.push(format!("{label}.training_focus_for must not be empty"));
            }
            let mut seen = std::collections::HashSet::new();
            for (track_index, track_id) in training_tracks.iter().enumerate() {
                if track_id.trim().is_empty() {
                    errors.push(format!(
                        "{label}.training_focus_for[{track_index}] must be non-empty"
                    ));
                    continue;
                }
                if !seen.insert(track_id.as_str()) {
                    errors.push(format!(
                        "{label}.training_focus_for must not contain duplicates"
                    ));
                }
                if self
                    .skill_catalog
                    .as_ref()
                    .and_then(|catalog| catalog.track(track_id))
                    .is_none()
                {
                    errors.push(format!(
                        "{label}.training_focus_for[{track_index}] references unknown skill catalog track {track_id:?}"
                    ));
                }
            }
        }

        if let Some(book_tracks) = &cap.spell_book_for {
            if item.kind != "book" {
                errors.push(format!(
                    "{label}.spell_book_for is valid only for book items"
                ));
            }
            if !item.valid_placements.contains(&ItemPlacementKind::Hand)
                || !item.valid_placements.contains(&ItemPlacementKind::Sack)
            {
                errors.push(format!(
                    "{label}.spell_book_for requires hand and sack valid placements"
                ));
            }
            if book_tracks.is_empty() {
                errors.push(format!("{label}.spell_book_for must not be empty"));
            }
            let mut seen = HashSet::new();
            for (track_index, track_id) in book_tracks.iter().enumerate() {
                if !matches!(
                    track_id.as_str(),
                    "wizard_magic" | "thaumaturge_magic" | "thief_magic"
                ) {
                    errors.push(format!(
                        "{label}.spell_book_for[{track_index}] must be wizard_magic, thaumaturge_magic, or thief_magic"
                    ));
                }
                if !seen.insert(track_id.as_str()) {
                    errors.push(format!(
                        "{label}.spell_book_for must not contain duplicates"
                    ));
                }
                if cap
                    .training_focus_for
                    .as_ref()
                    .is_some_and(|tracks| tracks.contains(track_id))
                {
                    errors.push(format!(
                        "{label}.{track_id} must not appear in both spell_book_for and training_focus_for"
                    ));
                }
                if self
                    .skill_catalog
                    .as_ref()
                    .and_then(|catalog| catalog.track(track_id))
                    .is_none()
                {
                    errors.push(format!(
                        "{label}.spell_book_for[{track_index}] references unknown skill catalog track {track_id:?}"
                    ));
                }
            }
        }

        if let Some(ref adds) = cap.attribute_adds {
            for (i, bonus) in adds.iter().enumerate() {
                if bonus.stat.is_empty() {
                    errors.push(format!(
                        "{label}.attribute_adds[{i}].stat must not be empty"
                    ));
                }
            }
        }

        if let Some(ref adds) = cap.resource_adds {
            for (i, bonus) in adds.iter().enumerate() {
                if bonus.stat.is_empty() {
                    errors.push(format!("{label}.resource_adds[{i}].stat must not be empty"));
                }
            }
        }

        if let Some(ref boosts) = cap.resistance_boosts {
            if boosts.is_empty() {
                errors.push(format!("{label}.resistance_boosts must not be empty"));
            }
            let mut tags = HashSet::new();
            for (i, boost) in boosts.iter().enumerate() {
                if boost.tag.trim().is_empty() {
                    errors.push(format!(
                        "{label}.resistance_boosts[{i}].tag must not be empty"
                    ));
                }
                if boost.bonus_twentieths == 0
                    || boost.bonus_twentieths > self.rules.magic.resistance.denominator
                {
                    errors.push(format!("{label}.resistance_boosts[{i}].bonus_twentieths must be in 1..=rules.magic.resistance.denominator"));
                }
                if !tags.insert(boost.tag.as_str()) {
                    errors.push(format!("{label}.resistance_boosts tags must be unique"));
                }
            }
        }
        if let Some(multiplier) = &cap.mp_recovery_multiplier {
            if item.kind == "consumable" {
                errors.push(format!(
                    "{label}.mp_recovery_multiplier is invalid for consumable items"
                ));
            }
            if !item.valid_placements.iter().any(|placement| {
                matches!(
                    placement,
                    ItemPlacementKind::Head
                        | ItemPlacementKind::Neck
                        | ItemPlacementKind::Arm
                        | ItemPlacementKind::Gloves
                        | ItemPlacementKind::InnerArmor
                        | ItemPlacementKind::OuterArmor
                        | ItemPlacementKind::Boots
                )
            }) {
                errors.push(format!(
                    "{label}.mp_recovery_multiplier requires a worn valid placement"
                ));
            }
            if multiplier.numerator == 0 || multiplier.denominator == 0 {
                errors.push(format!(
                    "{label}.mp_recovery_multiplier numerator and denominator must be positive"
                ));
            } else {
                if multiplier.numerator < multiplier.denominator {
                    errors.push(format!(
                        "{label}.mp_recovery_multiplier must not reduce MP recovery"
                    ));
                }
                if gcd_u32(multiplier.numerator, multiplier.denominator) != 1 {
                    errors.push(format!("{label}.mp_recovery_multiplier must be reduced"));
                }
            }
            if multiplier.evidence_state
                != crate::model::MagicRuleEvidenceState::OriginalProvisional
            {
                errors.push(format!(
                    "{label}.mp_recovery_multiplier.evidence_state must be original_provisional"
                ));
            }
            if multiplier.denominator > 0 {
                let scaled = i64::from(self.rules.resources.mp_recovery)
                    .checked_mul(i64::from(multiplier.numerator))
                    .map(|value| value / i64::from(multiplier.denominator));
                if scaled.is_none_or(|value| value > i64::from(i32::MAX)) {
                    errors.push(format!(
                        "{label}.mp_recovery_multiplier result exceeds supported range"
                    ));
                }
            }
        }
    }

    fn validate_labels(&self, errors: &mut Vec<String>) {
        if self.damage_labels.is_empty() {
            errors.push("damage_labels must be non-empty".to_string());
        }
        let mut ids = HashMap::new();
        for (index, label) in self.damage_labels.iter().enumerate() {
            if label.id.trim().is_empty() {
                errors.push(format!("damage_labels[{index}].id must be non-empty"));
            }
            if let Some(previous) = ids.insert(label.id.clone(), index) {
                errors.push(format!(
                    "damage_labels[{index}].id duplicates damage_labels[{previous}].id"
                ));
            }
            if label.name.trim().is_empty() {
                errors.push(format!("damage_labels[{index}].name must be non-empty"));
            }
        }

        let mut actual = self
            .damage_labels
            .iter()
            .map(|label| label.id.as_str())
            .collect::<Vec<_>>();
        actual.sort_unstable();
        if actual.as_slice() != CANONICAL_DAMAGE_LABEL_IDS {
            errors.push(format!(
                "damage_labels must declare exactly these ids: {}",
                CANONICAL_DAMAGE_LABEL_IDS.join(", ")
            ));
        }
    }

    fn validate_active_effect_entries(
        &self,
        prefix: &str,
        active_effects: &[ActiveEffectDef],
        errors: &mut Vec<String>,
    ) {
        const SOURCE_KINDS: &[&str] = &["actor", "fixture", "item", "spell"];
        const STACKING: &[&str] = &["replace_same_kind", "stack_instance", "refresh_duration"];
        let mut instances = HashMap::new();
        for (effect_index, effect) in active_effects.iter().enumerate() {
            let label = format!("{prefix}[{effect_index}]");
            if effect.instance_id.trim().is_empty() {
                errors.push(format!("{label}.instance_id must be non-empty"));
            } else if let Some(previous) =
                instances.insert(effect.instance_id.clone(), effect_index)
            {
                errors.push(format!(
                    "{label}.instance_id duplicates {prefix}[{previous}].instance_id"
                ));
            }
            if effect.effect_id.trim().is_empty() {
                errors.push(format!("{label}.effect_id must be non-empty"));
            }
            if !SOURCE_KINDS.contains(&effect.source.kind.as_str()) {
                errors.push(format!(
                    "{label}.source.kind must be one of {}",
                    SOURCE_KINDS.join(", ")
                ));
            }
            if effect.source.id.trim().is_empty() {
                errors.push(format!("{label}.source.id must be non-empty"));
            }
            if effect.kind.trim().is_empty() {
                errors.push(format!("{label}.kind must be non-empty"));
            }
            for (tag_index, tag) in effect.tags.iter().enumerate() {
                if tag.trim().is_empty() {
                    errors.push(format!("{label}.tags[{tag_index}] must be non-empty"));
                }
            }
            let mut resistance_boosts = HashSet::new();
            for (boost_index, boost) in effect.resistance_boosts.iter().enumerate() {
                if boost.tag.trim().is_empty() {
                    errors.push(format!(
                        "{label}.resistance_boosts[{boost_index}].tag must be non-empty"
                    ));
                }
                if boost.bonus_twentieths == 0
                    || boost.bonus_twentieths > self.rules.magic.resistance.denominator
                {
                    errors.push(format!("{label}.resistance_boosts[{boost_index}].bonus_twentieths must be in 1..=rules.magic.resistance.denominator"));
                }
                if !resistance_boosts.insert(boost.tag.as_str()) {
                    errors.push(format!("{label}.resistance_boosts tags must be unique"));
                }
            }
            if effect.potency < 0 {
                errors.push(format!("{label}.potency must be non-negative"));
            }
            if matches!(effect.remaining_rounds, Some(rounds) if rounds <= 0) {
                errors.push(format!("{label}.remaining_rounds must be positive"));
            }
            if effect
                .until_condition
                .as_ref()
                .is_some_and(|condition| condition.trim().is_empty())
            {
                errors.push(format!("{label}.until_condition must be non-empty"));
            }
            if !STACKING.contains(&effect.stacking.as_str()) {
                errors.push(format!(
                    "{label}.stacking must be one of {}",
                    STACKING.join(", ")
                ));
            }
            if effect.start_delay_rounds < 0 {
                errors.push(format!("{label}.start_delay_rounds must be non-negative"));
            }
            if effect.tick_interval_rounds <= 0 {
                errors.push(format!("{label}.tick_interval_rounds must be positive"));
            }
        }
    }

    fn validate_spells(&self, internal_parity_fixture: bool, errors: &mut Vec<String>) {
        let mut ids = HashMap::new();
        let catalog_mode = self
            .spells
            .iter()
            .any(|spell| spell.catalog_entry.is_some());
        if catalog_mode && !internal_parity_fixture {
            errors.push(
                "spell catalog entries are only valid in a marked internal_parity_fixture"
                    .to_string(),
            );
        }
        let mut catalog_row_ids = HashMap::new();
        for (index, spell) in self.spells.iter().enumerate() {
            if spell.id.trim().is_empty() {
                errors.push(format!("spells[{index}].id must be non-empty"));
            }
            if let Some(previous) = ids.insert(spell.id.clone(), index) {
                errors.push(format!(
                    "spells[{index}].id duplicates spells[{previous}].id"
                ));
            }
            if spell.name.trim().is_empty() {
                errors.push(format!("spells[{index}].name must be non-empty"));
            }
            if spell.status != "stub" && spell.status != "draft" {
                errors.push(format!("spells[{index}].status must be stub or draft"));
            }
            validate_spell_social_definition(spell, index, errors);
            if catalog_mode && spell.catalog_entry.is_none() {
                errors.push(format!(
                    "spells[{index}].catalog_entry is required when any spell has catalog metadata"
                ));
            }
            if let Some(entry) = &spell.catalog_entry {
                self.validate_spell_catalog_entry(
                    index,
                    spell,
                    entry,
                    &mut catalog_row_ids,
                    errors,
                );
            }
            let topic_only_catalog_row = internal_parity_fixture
                && spell
                    .catalog_entry
                    .as_ref()
                    .is_some_and(|entry| entry.acquisition_row_id.is_none());
            if spell.casting.is_none() && !topic_only_catalog_row {
                errors.push(format!(
                    "spells[{index}].casting is required for operational spells"
                ));
            }
            if spell
                .effect
                .as_ref()
                .is_some_and(|effect| effect.family != SpellEffectFamily::DirectDamage)
                && spell.casting.as_ref().is_some_and(|casting| {
                    matches!(
                        casting.cast_class,
                        SpellCastClass::Path | SpellCastClass::PathOrCharacter
                    )
                })
            {
                errors.push(format!(
                    "spells[{index}].casting.cast_class may be path or path_or_character only for direct_damage"
                ));
            }
            if let Some(req) = spell.skill_requirement
                && req < 1
            {
                errors.push(format!(
                    "spells[{index}].skill_requirement must be >= 1, got {req}"
                ));
            }
            if spell.lane.as_deref() == Some("knight_magic") {
                if spell.skill_requirement.is_some() {
                    errors.push(format!(
                        "spells[{index}].skill_requirement must be absent for knight_magic"
                    ));
                }
                if spell.mp_cost != Some(3) {
                    errors.push(format!(
                        "spells[{index}].mp_cost must be 3 for knight_magic"
                    ));
                }
                if spell.acquisition.is_some() {
                    errors.push(format!(
                        "spells[{index}].acquisition must be absent for knight_magic"
                    ));
                }
                if spell.casting.as_ref().map(|casting| casting.method)
                    != Some(SpellCastingMethod::Direct)
                {
                    errors.push(format!(
                        "spells[{index}].casting.method must be direct for knight_magic"
                    ));
                }
            }
            self.validate_spell_effect(index, spell, errors);
            self.validate_spell_target(index, spell, errors);
            self.validate_spell_acquisition(index, spell, errors);
        }
    }

    fn validate_spell_catalog_entry<'a>(
        &self,
        index: usize,
        spell: &'a SpellDef,
        entry: &'a SpellCatalogEntryDef,
        catalog_row_ids: &mut HashMap<&'a str, usize>,
        errors: &mut Vec<String>,
    ) {
        let label = format!("spells[{index}].catalog_entry");
        if entry.row_id.trim().is_empty() {
            errors.push(format!("{label}.row_id must be non-empty"));
        } else {
            if entry.row_id != spell.id {
                errors.push(format!("{label}.row_id must equal spells[{index}].id"));
            }
            if let Some(previous) = catalog_row_ids.insert(entry.row_id.as_str(), index) {
                errors.push(format!(
                    "{label}.row_id duplicates spells[{previous}].catalog_entry.row_id"
                ));
            }
        }
        if entry.topic_id.trim().is_empty() {
            errors.push(format!("{label}.topic_id must be non-empty"));
        }
        if entry.variant_id.trim().is_empty() {
            errors.push(format!("{label}.variant_id must be non-empty"));
        }
        if entry
            .acquisition_row_id
            .as_ref()
            .is_some_and(|row_id| row_id.trim().is_empty())
        {
            errors.push(format!(
                "{label}.acquisition_row_id must be non-empty when present"
            ));
        }
        let mut question_ids = HashSet::new();
        for (question_index, question_id) in entry.open_question_ids.iter().enumerate() {
            if question_id.trim().is_empty() {
                errors.push(format!(
                    "{label}.open_question_ids[{question_index}] must be non-empty"
                ));
            } else if !question_ids.insert(question_id.as_str()) {
                errors.push(format!(
                    "{label}.open_question_ids[{question_index}] duplicates {question_id:?}"
                ));
            }
        }
        let mut resistance_tags = HashSet::new();
        for (tag_index, tag) in entry.resistance_tags.iter().enumerate() {
            if tag.trim().is_empty() {
                errors.push(format!(
                    "{label}.resistance_tags[{tag_index}] must be non-empty"
                ));
            } else if !resistance_tags.insert(tag.as_str()) {
                errors.push(format!(
                    "{label}.resistance_tags[{tag_index}] duplicates {tag:?}"
                ));
            }
        }
        if entry.resistance_mitigation_mode.is_some() && entry.resistance_tags.is_empty() {
            errors.push(format!(
                "{label}.resistance_mitigation_mode requires at least one resistance tag"
            ));
        }
        let client_fields_present = [
            entry.client_row_id.is_some(),
            entry.client_spell_id.is_some(),
            entry.client_verb_type.is_some(),
            entry.client_powerable.is_some(),
            entry.client_spell_poem_id.is_some(),
            entry.client_offensive.is_some(),
        ];
        if client_fields_present.iter().any(|present| *present)
            && !client_fields_present.iter().all(|present| *present)
        {
            errors.push(format!(
                "{label} client metadata must be complete when linked"
            ));
        }
        if entry
            .client_row_id
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            errors.push(format!(
                "{label}.client_row_id must be non-empty when present"
            ));
        }
        if entry.client_spell_id == Some(0) {
            errors.push(format!(
                "{label}.client_spell_id must be positive when present"
            ));
        }
        if entry.client_spell_poem_id == Some(0) {
            errors.push(format!(
                "{label}.client_spell_poem_id must be positive when present"
            ));
        }
        if entry.target_kind.is_none() && entry.state != SpellCatalogState::OpenEvidence {
            errors.push(format!(
                "{label}.target_kind may be absent only for open_evidence"
            ));
        }
        if let Some(effect) = &spell.effect
            && effect.family != entry.effect_family
        {
            errors.push(format!(
                "spells[{index}].effect.family must match {label}.effect_family"
            ));
        }
        if let Some(target) = &spell.target
            && Some(target.kind) != entry.target_kind
        {
            errors.push(format!(
                "spells[{index}].target.kind must match {label}.target_kind"
            ));
        }

        if entry.acquisition_row_id.is_none() {
            for (field, present) in [
                ("lane", spell.lane.is_some()),
                ("skill_requirement", spell.skill_requirement.is_some()),
                ("mp_cost", spell.mp_cost.is_some()),
                ("acquisition", spell.acquisition.is_some()),
            ] {
                if present {
                    errors.push(format!(
                        "spells[{index}].{field} must be absent for a topic-only catalog entry"
                    ));
                }
            }
            return;
        }

        let lane = spell.lane.as_deref();
        if !matches!(
            lane,
            Some("knight_magic" | "thaumaturge_magic" | "thief_magic" | "wizard_magic")
        ) {
            errors.push(format!(
                "spells[{index}].lane must be a current magic lane for a class-linked catalog entry"
            ));
        }
        if !matches!(spell.mp_cost, Some(cost) if cost > 0) {
            errors.push(format!(
                "spells[{index}].mp_cost must be positive for a class-linked catalog entry"
            ));
        }
        if lane != Some("knight_magic") {
            if !matches!(spell.skill_requirement, Some(level) if level > 0) {
                errors.push(format!(
                    "spells[{index}].skill_requirement must be positive for a non-Knight class-linked catalog entry"
                ));
            }
            if spell.acquisition.is_none() {
                errors.push(format!(
                    "spells[{index}].acquisition must be present for a non-Knight class-linked catalog entry"
                ));
            }
        }
    }

    fn validate_spell_effect(&self, index: usize, spell: &SpellDef, errors: &mut Vec<String>) {
        let Some(effect) = &spell.effect else {
            return;
        };

        if let Some(duration) = &effect.duration {
            if duration.policy == SpellDurationPolicy::Rounds && duration.rounds.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.duration.rounds must be positive for rounds duration"
                ));
            }
            if let Some(rounds) = duration.rounds
                && rounds <= 0
            {
                errors.push(format!(
                    "spells[{index}].effect.duration.rounds must be positive for rounds duration"
                ));
            }
            if let Some(tick_interval_rounds) = duration.tick_interval_rounds
                && tick_interval_rounds <= 0
            {
                errors.push(format!(
                    "spells[{index}].effect.duration.tick_interval_rounds must be positive"
                ));
            }
        }

        if is_status_like_spell_family(effect.family)
            && !(effect.family == SpellEffectFamily::Concealment && effect.door_control.is_some())
        {
            if effect.status_kind.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.status_kind must be present for {} spells",
                    effect.family
                ));
            }
            if effect.duration.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.duration must be present for {} spells",
                    effect.family
                ));
            }
            if matches!(
                effect.family,
                SpellEffectFamily::Concealment
                    | SpellEffectFamily::FallProtection
                    | SpellEffectFamily::Speed
                    | SpellEffectFamily::Vision
                    | SpellEffectFamily::WaterBreathing
            ) && effect.stacking.is_none()
            {
                errors.push(format!(
                    "spells[{index}].effect.stacking must be present for {} spells",
                    effect.family
                ));
            }
        }

        let required_status_kind = match effect.family {
            SpellEffectFamily::Concealment if effect.door_control.is_none() => Some("hidden"),
            SpellEffectFamily::FallProtection => Some("fall_protection"),
            SpellEffectFamily::Speed => Some("speed"),
            SpellEffectFamily::Vision => Some("night_vision"),
            SpellEffectFamily::WaterBreathing => Some("water_breathing"),
            _ => None,
        };
        if let Some(required) = required_status_kind
            && effect.status_kind.as_deref() != Some(required)
        {
            errors.push(format!(
                "spells[{index}].effect.status_kind must be {required} for {} spells",
                effect.family
            ));
        }

        if requires_spell_potency(effect.family)
            && !matches!(effect.potency, Some(potency) if potency > 0)
        {
            errors.push(format!(
                "spells[{index}].effect.potency must be positive for {} spells",
                effect.family
            ));
        }

        if let Some(start_delay_rounds) = effect.start_delay_rounds
            && start_delay_rounds < 0
        {
            errors.push(format!(
                "spells[{index}].effect.start_delay_rounds must be non-negative"
            ));
        }

        let incoming_family = matches!(
            effect.family,
            SpellEffectFamily::DirectDamage
                | SpellEffectFamily::InstantDeath
                | SpellEffectFamily::ControlStatus
                | SpellEffectFamily::Poison
        );
        let boost_family = matches!(
            effect.family,
            SpellEffectFamily::Protection | SpellEffectFamily::Resistance
        );
        match effect.resistance.as_ref() {
            Some(SpellResistanceDef::Incoming { tag, mitigation }) => {
                if !incoming_family {
                    errors.push(format!(
                        "spells[{index}].effect.resistance incoming role is not valid for {}",
                        effect.family
                    ));
                }
                if tag.trim().is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.resistance.tag must be non-empty"
                    ));
                }
                if matches!(
                    effect.family,
                    SpellEffectFamily::ControlStatus | SpellEffectFamily::Poison
                ) && !matches!(mitigation, SpellResistanceMitigation::Negate)
                {
                    errors.push(format!(
                        "spells[{index}].effect.resistance.mitigation must be negate for {}",
                        effect.family
                    ));
                }
                match mitigation {
                    SpellResistanceMitigation::HalfDamage { minimum_damage, .. } if *minimum_damage <= 0 => errors.push(format!("spells[{index}].effect.resistance.mitigation.minimum_damage must be positive")),
                    SpellResistanceMitigation::MinimumDamage { damage } if *damage <= 0 => errors.push(format!("spells[{index}].effect.resistance.mitigation.damage must be positive")),
                    _ => {}
                }
            }
            Some(SpellResistanceDef::Boost { boosts }) => {
                if !boost_family {
                    errors.push(format!(
                        "spells[{index}].effect.resistance boost role is not valid for {}",
                        effect.family
                    ));
                }
                if boosts.is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.resistance.boosts must be non-empty"
                    ));
                }
                let mut tags = HashSet::new();
                for (boost_index, boost) in boosts.iter().enumerate() {
                    if boost.tag.trim().is_empty() {
                        errors.push(format!("spells[{index}].effect.resistance.boosts[{boost_index}].tag must be non-empty"));
                    }
                    if boost.bonus_twentieths == 0
                        || boost.bonus_twentieths > self.rules.magic.resistance.denominator
                    {
                        errors.push(format!("spells[{index}].effect.resistance.boosts[{boost_index}].bonus_twentieths must be in 1..=rules.magic.resistance.denominator"));
                    }
                    if !tags.insert(boost.tag.as_str()) {
                        errors.push(format!(
                            "spells[{index}].effect.resistance.boosts tags must be unique"
                        ));
                    }
                }
            }
            None if incoming_family => errors.push(format!(
                "spells[{index}].effect.resistance must use the incoming role for {}",
                effect.family
            )),
            None if boost_family => errors.push(format!(
                "spells[{index}].effect.resistance must use the boost role for {}",
                effect.family
            )),
            None => {}
        }

        let payloads = [
            ("banish", effect.banish.is_some()),
            ("instant_death", effect.instant_death.is_some()),
            ("raise_dead", effect.raise_dead.is_some()),
            ("turn_undead", effect.turn_undead.is_some()),
        ];
        for (family, present) in payloads {
            if present && effect.family.label() != family {
                errors.push(format!(
                    "spells[{index}].effect.{family} is valid only for {family} spells"
                ));
            }
        }
        match effect.family {
            SpellEffectFamily::Banish => match &effect.banish {
                Some(banish) => {
                    let expected = [
                        crate::model::CreatureTrait::Demon,
                        crate::model::CreatureTrait::Phantasm,
                    ];
                    if banish.eligible_traits.as_slice() != expected {
                        errors.push(format!(
                            "spells[{index}].effect.banish.eligible_traits must equal demon, phantasm"
                        ));
                    }
                }
                None => errors.push(format!(
                    "spells[{index}].effect.banish must be present for banish spells"
                )),
            },
            SpellEffectFamily::InstantDeath => match &effect.instant_death {
                Some(definition) if definition.damage_per_magic_level > 0 => {}
                Some(_) => errors.push(format!(
                    "spells[{index}].effect.instant_death.damage_per_magic_level must be positive"
                )),
                None => errors.push(format!(
                    "spells[{index}].effect.instant_death must be present for instant_death spells"
                )),
            },
            SpellEffectFamily::RaiseDead => match &effect.raise_dead {
                Some(definition)
                    if definition.method == crate::model::ResurrectionMethod::Thaumaturge => {}
                Some(_) => errors.push(format!(
                    "spells[{index}].effect.raise_dead.method must be thaumaturge"
                )),
                None => errors.push(format!(
                    "spells[{index}].effect.raise_dead must be present for raise_dead spells"
                )),
            },
            SpellEffectFamily::TurnUndead => match &effect.turn_undead {
                Some(definition)
                    if definition.eligible_trait == crate::model::CreatureTrait::Undead => {}
                Some(_) => errors.push(format!(
                    "spells[{index}].effect.turn_undead.eligible_trait must be undead"
                )),
                None => errors.push(format!(
                    "spells[{index}].effect.turn_undead must be present for turn_undead spells"
                )),
            },
            _ => {}
        }

        if effect.family == SpellEffectFamily::Concealment && effect.door_control.is_some() {
            if effect.duration.is_none() {
                errors.push(format!(
                    "spells[{index}].effect.duration must be present for door concealment"
                ));
            }
            if effect
                .door_control
                .as_ref()
                .is_some_and(|control| control.action != "hide_secret")
            {
                errors.push(format!(
                    "spells[{index}].effect.door_control.action must be hide_secret for door concealment"
                ));
            }
            if effect.status_kind.is_some() || effect.stacking.is_some() {
                errors.push(format!(
                    "spells[{index}].effect.status_kind and stacking must be absent for door concealment"
                ));
            }
        }

        if let Some(stacking) = effect.stacking.as_deref()
            && !ACTIVE_EFFECT_STACKING.contains(&stacking)
        {
            errors.push(format!(
                "spells[{index}].effect.stacking must be one of {}",
                ACTIVE_EFFECT_STACKING.join(", ")
            ));
        }

        if effect.family == SpellEffectFamily::Summon && effect.summon_actor_id.is_none() {
            errors.push(format!(
                "spells[{index}].effect.summon_actor_id must be present for summon spells"
            ));
        }
        if effect.family == SpellEffectFamily::Summon && effect.duration.is_none() {
            errors.push(format!(
                "spells[{index}].effect.duration must be present for summon spells"
            ));
        }
        if effect.family == SpellEffectFamily::Summon
            && let Some(summon_actor_id) = effect.summon_actor_id.as_deref()
            && !self
                .summon_templates
                .iter()
                .any(|template| template.id == summon_actor_id)
        {
            errors.push(format!(
                "spells[{index}].effect.summon_actor_id '{summon_actor_id}' is not a summon_templates id"
            ));
        }

        if effect.family == SpellEffectFamily::TerrainOverlay {
            match &effect.terrain_overlay {
                Some(terrain_overlay)
                    if terrain_overlay.passability.is_some()
                        || terrain_overlay.sight.is_some()
                        || terrain_overlay.hazard.is_some()
                        || terrain_overlay.move_cost.is_some() => {}
                _ => errors.push(format!(
                    "spells[{index}].effect.terrain_overlay must declare passability, sight, hazard, or move_cost"
                )),
            }
        }

        if let Some(terrain_overlay) = &effect.terrain_overlay {
            if let Some(passability) = terrain_overlay.passability.as_deref()
                && !SPELL_TERRAIN_OVERLAY_PASSABILITY.contains(&passability)
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.passability must be one of {}",
                    SPELL_TERRAIN_OVERLAY_PASSABILITY.join(", ")
                ));
            }
            if let Some(sight) = terrain_overlay.sight.as_deref()
                && !SPELL_TERRAIN_OVERLAY_SIGHT.contains(&sight)
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.sight must be one of {}",
                    SPELL_TERRAIN_OVERLAY_SIGHT.join(", ")
                ));
            }
            if let Some(hazard) = terrain_overlay.hazard.as_deref()
                && !SPELL_TERRAIN_OVERLAY_HAZARD.contains(&hazard)
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.hazard must be one of {}",
                    SPELL_TERRAIN_OVERLAY_HAZARD.join(", ")
                ));
            }
            if let Some(move_cost) = terrain_overlay.move_cost
                && move_cost <= 0
            {
                errors.push(format!(
                    "spells[{index}].effect.terrain_overlay.move_cost must be positive"
                ));
            }
        }

        if let Some(door_control) = &effect.door_control
            && !SPELL_DOOR_CONTROL_ACTIONS.contains(&door_control.action.as_str())
        {
            errors.push(format!(
                "spells[{index}].effect.door_control.action must be one of {}",
                SPELL_DOOR_CONTROL_ACTIONS.join(", ")
            ));
        }

        if let Some(item_utility) = &effect.item_utility {
            if !SPELL_ITEM_UTILITY_ACTIONS.contains(&item_utility.action.as_str()) {
                errors.push(format!(
                    "spells[{index}].effect.item_utility.action must be one of {}",
                    SPELL_ITEM_UTILITY_ACTIONS.join(", ")
                ));
            }
            for (tag_index, tag) in item_utility.tags.iter().enumerate() {
                if tag.trim().is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.item_utility.tags[{tag_index}] must be non-empty"
                    ));
                }
            }
            if let Some(output_item_definition_id) = &item_utility.output_item_definition_id {
                let item_label =
                    format!("spells[{index}].effect.item_utility.output_item_definition_id");
                if output_item_definition_id.trim().is_empty() {
                    errors.push(format!("{item_label} must be non-empty"));
                } else {
                    self.validate_item_definition_reference(
                        &item_label,
                        output_item_definition_id,
                        errors,
                    );
                }
            }
        }

        if let Some(locate) = &effect.locate {
            if !SPELL_LOCATE_SUBJECTS.contains(&locate.subject.as_str()) {
                errors.push(format!(
                    "spells[{index}].effect.locate.subject must be one of {}",
                    SPELL_LOCATE_SUBJECTS.join(", ")
                ));
            }
            if locate.id.trim().is_empty() {
                errors.push(format!(
                    "spells[{index}].effect.locate.id must be non-empty"
                ));
            }
        }

        if effect.family == SpellEffectFamily::Scry && effect.scry.is_none() {
            errors.push(format!(
                "spells[{index}].effect.scry must be present for scry spells"
            ));
        } else if let Some(scry) = &effect.scry {
            self.validate_spell_scry(index, scry, errors);
        }

        if let Some(portal) = &effect.portal {
            self.validate_spell_portal(index, portal, errors);
        }
    }

    fn validate_spell_target(&self, index: usize, spell: &SpellDef, errors: &mut Vec<String>) {
        if let Some(effect) = &spell.effect
            && requires_bu_target(effect.family)
            && spell.target.is_none()
        {
            errors.push(format!(
                "spells[{index}].target must be present for {} spells",
                effect.family
            ));
            return;
        }

        if let Some(effect) = &spell.effect
            && requires_item_target(effect.family)
            && spell.target.is_none()
        {
            errors.push(format!(
                "spells[{index}].target must be present for {} spells",
                effect.family
            ));
            return;
        }

        if let Some(effect) = &spell.effect
            && effect.family == SpellEffectFamily::Summon
            && spell.target.is_none()
        {
            errors.push(format!(
                "spells[{index}].target must be present for summon spells"
            ));
            return;
        }

        let Some(target) = &spell.target else {
            if let Some(effect) = &spell.effect
                && matches!(
                    effect.family,
                    SpellEffectFamily::RaiseDead | SpellEffectFamily::TurnUndead
                )
            {
                errors.push(format!(
                    "spells[{index}].target.kind must be none for {} spells",
                    effect.family
                ));
            }
            return;
        };

        if target.kind != SpellTargetKind::Item && target.item_location.is_some() {
            errors.push(format!(
                "spells[{index}].target.item_location is only valid for item targets"
            ));
        }

        if let Some(effect) = &spell.effect {
            let expected = match effect.family {
                SpellEffectFamily::Banish | SpellEffectFamily::InstantDeath => {
                    Some(SpellTargetKind::Actor)
                }
                SpellEffectFamily::FallProtection
                | SpellEffectFamily::Speed
                | SpellEffectFamily::Vision => Some(SpellTargetKind::SelfTarget),
                SpellEffectFamily::RaiseDead | SpellEffectFamily::TurnUndead => {
                    Some(SpellTargetKind::None)
                }
                SpellEffectFamily::WaterBreathing => Some(SpellTargetKind::Actor),
                SpellEffectFamily::Concealment if effect.door_control.is_some() => {
                    Some(SpellTargetKind::Door)
                }
                SpellEffectFamily::Concealment => Some(SpellTargetKind::SelfTarget),
                _ => None,
            };
            if expected.is_some_and(|expected| target.kind != expected) {
                errors.push(format!(
                    "spells[{index}].target.kind must be {} for {} spells",
                    expected.expect("checked expected target").label(),
                    effect.family
                ));
            }
        }

        if target.kind == SpellTargetKind::SelfTarget {
            if target.range.is_some() {
                errors.push(format!(
                    "spells[{index}].target.range is invalid for self target"
                ));
            }
            if target.requires_visible.is_some() {
                errors.push(format!(
                    "spells[{index}].target.requires_visible is invalid for self target"
                ));
            }
            if target.area.is_some() {
                errors.push(format!(
                    "spells[{index}].target.area is invalid for self target"
                ));
            }
        }

        if target.kind == SpellTargetKind::Area {
            match &target.area {
                Some(area) => {
                    if area.shape.trim().is_empty() {
                        errors.push(format!(
                            "spells[{index}].target.area.shape must be non-empty for area targets"
                        ));
                    }
                    match area.radius {
                        Some(radius) if radius > 0 => {}
                        Some(_) => errors.push(format!(
                            "spells[{index}].target.area.radius must be positive for area targets"
                        )),
                        None => errors.push(format!(
                            "spells[{index}].target.area.radius must be present for area targets"
                        )),
                    }
                }
                None if spell.catalog_entry.is_some() => {}
                None => {
                    errors.push(format!(
                        "spells[{index}].target.area.shape must be non-empty for area targets"
                    ));
                    errors.push(format!(
                        "spells[{index}].target.area.radius must be present for area targets"
                    ));
                }
            }
        }

        if spell
            .effect
            .as_ref()
            .is_some_and(|effect| requires_item_target(effect.family))
            && target.kind != SpellTargetKind::Item
        {
            errors.push(format!(
                "spells[{index}].target.kind must be item for {} spells",
                spell.effect.as_ref().unwrap().family
            ));
        }

        if let Some(effect) = &spell.effect {
            if effect.family == SpellEffectFamily::Summon
                && target.kind != SpellTargetKind::Coordinate
            {
                errors.push(format!(
                    "spells[{index}].target.kind must be coordinate for summon spells"
                ));
            }
            self.validate_bu_spell_target_family(index, effect, target, errors);
        }
    }

    fn validate_spell_portal(
        &self,
        index: usize,
        portal: &SpellPortalDef,
        errors: &mut Vec<String>,
    ) {
        let location = match &portal.target {
            TopologyTargetDef::Position { location } => location,
            TopologyTargetDef::Arrival { arrival_id } => {
                if arrival_id.trim().is_empty() {
                    errors.push(format!(
                        "spells[{index}].effect.portal.target.arrival_id must be non-empty"
                    ));
                    return;
                }
                let Some(location) = self.world_template.arrivals.get(arrival_id) else {
                    errors.push(format!(
                        "spells[{index}].effect.portal.target.arrival_id {arrival_id:?} does not exist"
                    ));
                    return;
                };
                location
            }
        };
        let terrain_map = world_template::terrain_map(&self.terrains);
        match world_template::position_status(&self.world_template, &terrain_map, location) {
            None => errors.push(format!(
                "spells[{index}].effect.portal.target references missing realm/level {}/{}",
                location.realm, location.level
            )),
            Some(world_template::WorldPositionStatus::OutOfBounds) => errors.push(format!(
                "spells[{index}].effect.portal.target is out of bounds at {}",
                location.label()
            )),
            Some(world_template::WorldPositionStatus::Blocked) => errors.push(format!(
                "spells[{index}].effect.portal.target is not traversable at {}",
                location.label()
            )),
            Some(world_template::WorldPositionStatus::Passable) => {}
        }
    }

    fn validate_spell_scry(&self, index: usize, scry: &SpellScryDef, errors: &mut Vec<String>) {
        if !SPELL_SCRY_SCOPES.contains(&scry.scope.as_str()) {
            errors.push(format!(
                "spells[{index}].effect.scry.scope must be one of {}",
                SPELL_SCRY_SCOPES.join(", ")
            ));
        }
        if scry.site.realm.trim().is_empty() || scry.site.level.trim().is_empty() {
            errors.push(format!(
                "spells[{index}].effect.scry.site realm and level must be non-empty"
            ));
            return;
        }
        let Some(level_def) = self
            .world_template
            .realms
            .get(&scry.site.realm)
            .and_then(|realm| realm.levels.get(&scry.site.level))
        else {
            errors.push(format!(
                "spells[{index}].effect.scry.site '{}' is not a level",
                scry.site.label()
            ));
            return;
        };

        match scry.scope.as_str() {
            "level" => {
                if scry.position.is_some() {
                    errors.push(format!(
                        "spells[{index}].effect.scry.position is invalid for level scope"
                    ));
                }
            }
            "coordinate" => {
                let Some(position) = scry.position else {
                    errors.push(format!(
                        "spells[{index}].effect.scry.position must be present for coordinate scope"
                    ));
                    return;
                };
                if position.x < 0
                    || position.y < 0
                    || position.x >= level_def.width
                    || position.y >= level_def.height
                {
                    errors.push(format!(
                        "spells[{index}].effect.scry.position [{},{}] out of bounds for level '{}' ({}x{})",
                        position.x, position.y, scry.site.label(), level_def.width, level_def.height
                    ));
                }
            }
            _ => {}
        }
    }

    fn validate_bu_spell_target_family(
        &self,
        index: usize,
        effect: &SpellEffectDef,
        target: &SpellTargetDef,
        errors: &mut Vec<String>,
    ) {
        match effect.family {
            SpellEffectFamily::DoorControl => {
                let action = effect
                    .door_control
                    .as_ref()
                    .map(|door| door.action.as_str());
                match action {
                    Some("open" | "close")
                        if target.kind != SpellTargetKind::Coordinate
                            && target.kind != SpellTargetKind::Door =>
                    {
                        errors.push(format!(
                            "spells[{index}].target.kind must be coordinate or door for door_control {} spells",
                            action.unwrap()
                        ));
                    }
                    Some("reveal_secret" | "hide_secret")
                        if target.kind != SpellTargetKind::None
                            && target.kind != SpellTargetKind::Coordinate =>
                    {
                        errors.push(format!(
                            "spells[{index}].target.kind must be none or coordinate for door_control {} spells",
                            action.unwrap()
                        ));
                    }
                    _ => {}
                }
            }
            SpellEffectFamily::SecretDetection => {
                if target.kind != SpellTargetKind::None
                    && target.kind != SpellTargetKind::Coordinate
                {
                    errors.push(format!(
                        "spells[{index}].target.kind must be none or coordinate for secret_detection spells"
                    ));
                }
            }
            SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::WeaponEnchant => {
                if target.kind != SpellTargetKind::Item {
                    errors.push(format!(
                        "spells[{index}].target.kind must be item for {} spells",
                        effect.family
                    ));
                }
            }
            SpellEffectFamily::Locate => {
                if target.kind != SpellTargetKind::None {
                    errors.push(format!(
                        "spells[{index}].target.kind must be none for locate spells"
                    ));
                }
            }
            SpellEffectFamily::Portal => {
                if target.kind != SpellTargetKind::Coordinate {
                    errors.push(format!(
                        "spells[{index}].target.kind must be coordinate for portal spells"
                    ));
                }
            }
            SpellEffectFamily::Scry if target.kind != SpellTargetKind::None => {
                errors.push(format!(
                    "spells[{index}].target.kind must be none for scry spells"
                ));
            }
            _ => {}
        }
    }

    fn validate_spell_acquisition(&self, index: usize, spell: &SpellDef, errors: &mut Vec<String>) {
        let Some(acquisition) = &spell.acquisition else {
            return;
        };

        if acquisition.gold_cost < 0 {
            errors.push(format!(
                "spells[{index}].acquisition.gold_cost must be >= 0"
            ));
        }
    }

    fn validate_profession_actions(&self, errors: &mut Vec<String>) {
        for (index, action) in self.profession_actions.iter().enumerate() {
            let prefix = format!("profession_actions[{index}]");
            if action.id.trim().is_empty() {
                errors.push(format!("{prefix}.id must be non-empty"));
            }
            if !PROFESSION_ACTION_KINDS.contains(&action.kind.as_str()) {
                errors.push(format!(
                    "{prefix}.kind must be one of {}",
                    PROFESSION_ACTION_KINDS.join(", ")
                ));
            }
            if action.class_ids.is_empty()
                || action
                    .class_ids
                    .iter()
                    .any(|class_id| class_id.trim().is_empty())
            {
                errors.push(format!("{prefix}.class_ids must be a non-empty list"));
            }
            action.validate_kind_fields(&prefix, errors);
        }
    }
}
