use std::collections::{BTreeMap, HashMap, HashSet};

use crate::content::{
    ActiveEffectDef, ActorSeedDef, CatalogProfileKey, ItemEconomyDef, ItemInstanceSeedDef,
    NpcInteractionOutcomeDef, ServiceCapabilityDef, SkillCatalogDef, SpellDef, StarterCharacterDef,
    TransactionCostDef, TransactionDef, TransactionRequirementDef, TransactionRewardDef,
    WorldSeedDef, WorldTemplateV3,
};
use crate::model::{
    ActorKind, CarriedPosition, ItemBindingState, ItemCapability, ItemPlacementKind, SkillEntry,
    WorldPosition,
};

use super::{ContentBoundaryPolicy, SelectedCatalog, ValidationError};

const TRANSACTION_CLASS_IDS: &[&str] = &[
    "fighter",
    "knight",
    "martial_artist",
    "thaumaturge",
    "thief",
    "wizard",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedWorldPositionStatus {
    Passable,
    Blocked,
    OutOfBounds,
}

#[derive(Debug, Clone, Copy)]
pub struct SeedItemValidationView<'a> {
    pub valid_placements: &'a [ItemPlacementKind],
    pub capability: Option<&'a ItemCapability>,
    pub economy: &'a ItemEconomyDef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedServiceCapabilityKind {
    SkillTraining,
    SkillCritique,
    SpellTeaching,
    ClassPromotion,
    ServiceTransaction,
    Merchant,
    ItemService,
    Restoration,
    Bank,
    Locker,
}

#[derive(Debug, Clone, Copy)]
pub struct SeedMerchantCapabilityView<'a> {
    pub id: &'a str,
    pub accepts_player_sales: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SeedPromotionCapabilityView<'a> {
    pub id: &'a str,
    pub target_class_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct SeedSpellTeachingPairView<'a> {
    pub capability_id: &'a str,
    pub class_id: &'a str,
    pub spell_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEcologyGroupView {
    pub spawn_group_id: String,
    pub member_ids: Vec<String>,
}

/// Read-only facts required to check a mutable-world seed. The runtime may
/// implement this directly over its already compiled immutable definition;
/// no source catalog/template copy is required.
pub trait WorldSeedValidationContext {
    fn boundary_policy(&self) -> ContentBoundaryPolicy;
    fn profile_key(&self) -> &CatalogProfileKey;
    fn world_position_status(&self, location: &WorldPosition) -> Option<SeedWorldPositionStatus>;
    fn progression_thresholds(&self) -> Vec<(i32, i64)>;
    fn progression_profile_exists(&self, class_id: &str) -> bool;
    fn burden_limits_per_strength(&self) -> [u64; 3];
    fn base_learning_rate(&self) -> u64;
    fn magic_resistance_denominator(&self) -> u32;
    fn actor_definition_kind(&self, definition_id: &str) -> Option<ActorKind>;
    fn actor_definition_uses_character_alignment(&self, definition_id: &str) -> Option<bool>;
    fn ecology_group(
        &self,
        source: &crate::content::EcologySiteSourceDef,
    ) -> Option<SeedEcologyGroupView>;
    fn skill_catalog(&self) -> Option<&SkillCatalogDef>;
    fn item(&self, definition_id: &str) -> Option<SeedItemValidationView<'_>>;
    fn spell(&self, spell_id: &str) -> Option<&SpellDef>;
    fn quest_exists(&self, quest_id: &str) -> bool;
    fn quest_stage_exists(&self, quest_id: &str, stage_id: &str) -> bool;
    fn service_definition_exists(&self, definition_id: &str) -> bool;
    fn service_capability_kind(
        &self,
        definition_id: &str,
        capability_id: &str,
    ) -> Option<SeedServiceCapabilityKind>;
    fn merchant_capabilities(&self, definition_id: &str) -> Vec<SeedMerchantCapabilityView<'_>>;
    fn merchant_pawn_listing_multiplier(
        &self,
        _definition_id: &str,
        _capability_id: &str,
    ) -> Option<u32> {
        None
    }
    fn promotion_capabilities(&self, _definition_id: &str) -> Vec<SeedPromotionCapabilityView<'_>> {
        Vec::new()
    }
    fn spell_teaching_pairs(&self, _definition_id: &str) -> Vec<SeedSpellTeachingPairView<'_>> {
        Vec::new()
    }
    fn service_grant_item_instance_ids(&self, _definition_id: &str) -> Vec<&str> {
        Vec::new()
    }
}

pub(super) struct SourceWorldSeedValidationContext<'a> {
    catalog: &'a SelectedCatalog,
    template: &'a WorldTemplateV3,
}

impl<'a> SourceWorldSeedValidationContext<'a> {
    pub(super) fn new(catalog: &'a SelectedCatalog, template: &'a WorldTemplateV3) -> Self {
        Self { catalog, template }
    }
}

impl WorldSeedValidationContext for SourceWorldSeedValidationContext<'_> {
    fn boundary_policy(&self) -> ContentBoundaryPolicy {
        if self.catalog.clean_content {
            ContentBoundaryPolicy::Clean
        } else {
            ContentBoundaryPolicy::InternalParity
        }
    }

    fn profile_key(&self) -> &CatalogProfileKey {
        &self.catalog.profile_key
    }

    fn world_position_status(&self, location: &WorldPosition) -> Option<SeedWorldPositionStatus> {
        let terrains = super::world_template::terrain_map(&self.catalog.terrains);
        super::world_template::position_status(self.template, &terrains, location).map(|status| {
            match status {
                super::world_template::WorldPositionStatus::Passable => {
                    SeedWorldPositionStatus::Passable
                }
                super::world_template::WorldPositionStatus::Blocked => {
                    SeedWorldPositionStatus::Blocked
                }
                super::world_template::WorldPositionStatus::OutOfBounds => {
                    SeedWorldPositionStatus::OutOfBounds
                }
            }
        })
    }

    fn progression_thresholds(&self) -> Vec<(i32, i64)> {
        self.catalog
            .rules
            .progression
            .level_thresholds
            .iter()
            .map(|row| (row.level, row.cumulative_experience))
            .collect()
    }

    fn progression_profile_exists(&self, class_id: &str) -> bool {
        self.catalog
            .rules
            .progression
            .growth_profiles
            .iter()
            .any(|profile| profile.class_id == class_id)
    }

    fn burden_limits_per_strength(&self) -> [u64; 3] {
        let burden = &self.catalog.rules.burden;
        [
            burden.lightly_loaded_max_per_strength,
            burden.moderately_loaded_max_per_strength,
            burden.heavily_loaded_max_per_strength,
        ]
    }

    fn base_learning_rate(&self) -> u64 {
        self.catalog.rules.skills.base_learning_rate
    }

    fn magic_resistance_denominator(&self) -> u32 {
        self.catalog.rules.magic.resistance.denominator
    }

    fn actor_definition_kind(&self, definition_id: &str) -> Option<ActorKind> {
        self.catalog
            .actor_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
            .map(|definition| definition.kind)
    }

    fn actor_definition_uses_character_alignment(&self, definition_id: &str) -> Option<bool> {
        self.catalog
            .actor_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
            .map(|definition| {
                matches!(
                    definition.social.alignment_source,
                    crate::content::SocialAlignmentSourceDef::Character {}
                )
            })
    }

    fn ecology_group(
        &self,
        source: &crate::content::EcologySiteSourceDef,
    ) -> Option<SeedEcologyGroupView> {
        let group = match source {
            crate::content::EcologySiteSourceDef::SpawnGroup { spawn_group_id } => self
                .catalog
                .spawn_groups
                .iter()
                .find(|group| group.id == *spawn_group_id),
            crate::content::EcologySiteSourceDef::Lair { lair_definition_id } => {
                let lair = self
                    .catalog
                    .lair_definitions
                    .iter()
                    .find(|lair| lair.id == *lair_definition_id)?;
                self.catalog
                    .spawn_groups
                    .iter()
                    .find(|group| group.id == lair.spawn_group_id)
            }
        }?;
        Some(SeedEcologyGroupView {
            spawn_group_id: group.id.clone(),
            member_ids: group
                .members
                .iter()
                .map(|member| member.member_id.clone())
                .collect(),
        })
    }

    fn skill_catalog(&self) -> Option<&SkillCatalogDef> {
        self.catalog.skill_catalog.as_ref()
    }

    fn item(&self, definition_id: &str) -> Option<SeedItemValidationView<'_>> {
        self.catalog
            .items
            .iter()
            .find(|item| item.id == definition_id)
            .map(|item| SeedItemValidationView {
                valid_placements: &item.valid_placements,
                capability: item.capability.as_ref(),
                economy: &item.economy,
            })
    }

    fn spell(&self, spell_id: &str) -> Option<&SpellDef> {
        self.catalog
            .spells
            .iter()
            .find(|spell| spell.id == spell_id)
    }

    fn quest_exists(&self, quest_id: &str) -> bool {
        self.catalog.quests.iter().any(|quest| quest.id == quest_id)
    }

    fn quest_stage_exists(&self, quest_id: &str, stage_id: &str) -> bool {
        self.catalog
            .quests
            .iter()
            .find(|quest| quest.id == quest_id)
            .is_some_and(|quest| quest.stages.iter().any(|stage| stage.id == stage_id))
    }

    fn service_definition_exists(&self, definition_id: &str) -> bool {
        self.catalog
            .service_definitions
            .iter()
            .any(|definition| definition.id == definition_id)
    }

    fn service_capability_kind(
        &self,
        definition_id: &str,
        capability_id: &str,
    ) -> Option<SeedServiceCapabilityKind> {
        let definition = self
            .catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)?;
        definition
            .capabilities
            .iter()
            .find(|capability| capability_id_for_source(capability) == capability_id)
            .map(capability_kind_for_source)
    }

    fn merchant_capabilities(&self, definition_id: &str) -> Vec<SeedMerchantCapabilityView<'_>> {
        self.catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
            .into_iter()
            .flat_map(|definition| &definition.capabilities)
            .filter_map(|capability| match capability {
                ServiceCapabilityDef::Merchant { id, player_sales } => {
                    Some(SeedMerchantCapabilityView {
                        id,
                        accepts_player_sales: player_sales.is_some(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    fn merchant_pawn_listing_multiplier(
        &self,
        definition_id: &str,
        capability_id: &str,
    ) -> Option<u32> {
        self.catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)?
            .capabilities
            .iter()
            .find_map(|capability| match capability {
                ServiceCapabilityDef::Merchant {
                    id,
                    player_sales: Some(policy),
                } if id == capability_id => Some(policy.pawn_listing_multiplier),
                _ => None,
            })
    }

    fn promotion_capabilities(&self, definition_id: &str) -> Vec<SeedPromotionCapabilityView<'_>> {
        let Some(definition) = self
            .catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
        else {
            return Vec::new();
        };
        definition
            .capabilities
            .iter()
            .filter_map(|capability| match capability {
                ServiceCapabilityDef::ClassPromotion { id, transaction } => transaction
                    .rewards
                    .iter()
                    .find_map(|reward| match reward {
                        TransactionRewardDef::Class { to_class_id, .. } => Some(to_class_id),
                        _ => None,
                    })
                    .map(|target_class_id| SeedPromotionCapabilityView {
                        id,
                        target_class_id,
                    }),
                _ => None,
            })
            .collect()
    }

    fn spell_teaching_pairs(&self, definition_id: &str) -> Vec<SeedSpellTeachingPairView<'_>> {
        let Some(definition) = self
            .catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
        else {
            return Vec::new();
        };
        let mut pairs = Vec::new();
        for capability in &definition.capabilities {
            let ServiceCapabilityDef::SpellTeaching {
                id,
                training_capability_id,
                teachings,
            } = capability
            else {
                continue;
            };
            let Some(ServiceCapabilityDef::SkillTraining { offers, .. }) = definition
                .capabilities
                .iter()
                .find(|candidate| capability_id_for_source(candidate) == training_capability_id)
            else {
                continue;
            };
            let magic_offers = offers
                .iter()
                .filter(|offer| super::is_spell_teaching_lane(&offer.track_id))
                .collect::<Vec<_>>();
            let [offer] = magic_offers.as_slice() else {
                continue;
            };
            for teaching in teachings {
                for class_id in &offer.eligible_class_ids {
                    pairs.push(SeedSpellTeachingPairView {
                        capability_id: id,
                        class_id,
                        spell_id: &teaching.spell_id,
                    });
                }
            }
        }
        pairs
    }

    fn service_grant_item_instance_ids(&self, definition_id: &str) -> Vec<&str> {
        let Some(definition) = self
            .catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
        else {
            return Vec::new();
        };
        let mut ids = Vec::new();
        for capability in &definition.capabilities {
            match capability {
                ServiceCapabilityDef::ClassPromotion { transaction, .. } => {
                    collect_transaction_item_grant_ids(transaction, &mut ids);
                }
                ServiceCapabilityDef::ServiceTransaction { transactions, .. } => {
                    for transaction in transactions {
                        collect_transaction_item_grant_ids(transaction, &mut ids);
                    }
                }
                ServiceCapabilityDef::Restoration { operations, .. } => {
                    for operation in operations {
                        collect_transaction_item_grant_ids(&operation.transaction, &mut ids);
                    }
                }
                _ => {}
            }
        }
        ids
    }
}

fn collect_transaction_item_grant_ids<'a>(transaction: &'a TransactionDef, ids: &mut Vec<&'a str>) {
    ids.extend(
        transaction
            .rewards
            .iter()
            .filter_map(|reward| match reward {
                TransactionRewardDef::Item {
                    item_instance_id, ..
                } => Some(item_instance_id.as_str()),
                _ => None,
            }),
    );
}

fn capability_id_for_source(capability: &ServiceCapabilityDef) -> &str {
    match capability {
        ServiceCapabilityDef::SkillTraining { id, .. }
        | ServiceCapabilityDef::SkillCritique { id }
        | ServiceCapabilityDef::SpellTeaching { id, .. }
        | ServiceCapabilityDef::ClassPromotion { id, .. }
        | ServiceCapabilityDef::ServiceTransaction { id, .. }
        | ServiceCapabilityDef::Merchant { id, .. }
        | ServiceCapabilityDef::ItemService { id, .. }
        | ServiceCapabilityDef::Restoration { id, .. }
        | ServiceCapabilityDef::Bank { id, .. }
        | ServiceCapabilityDef::Locker { id, .. } => id,
    }
}

fn capability_kind_for_source(capability: &ServiceCapabilityDef) -> SeedServiceCapabilityKind {
    match capability {
        ServiceCapabilityDef::SkillTraining { .. } => SeedServiceCapabilityKind::SkillTraining,
        ServiceCapabilityDef::SkillCritique { .. } => SeedServiceCapabilityKind::SkillCritique,
        ServiceCapabilityDef::SpellTeaching { .. } => SeedServiceCapabilityKind::SpellTeaching,
        ServiceCapabilityDef::ClassPromotion { .. } => SeedServiceCapabilityKind::ClassPromotion,
        ServiceCapabilityDef::ServiceTransaction { .. } => {
            SeedServiceCapabilityKind::ServiceTransaction
        }
        ServiceCapabilityDef::Merchant { .. } => SeedServiceCapabilityKind::Merchant,
        ServiceCapabilityDef::ItemService { .. } => SeedServiceCapabilityKind::ItemService,
        ServiceCapabilityDef::Restoration { .. } => SeedServiceCapabilityKind::Restoration,
        ServiceCapabilityDef::Bank { .. } => SeedServiceCapabilityKind::Bank,
        ServiceCapabilityDef::Locker { .. } => SeedServiceCapabilityKind::Locker,
    }
}

pub(super) fn validate_world_seed(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
) -> Result<(), ValidationError> {
    let mut errors = Vec::new();
    validate_actors(seed, context, &mut errors);
    validate_ground_items(seed, context, &mut errors);
    validate_services(seed, context, &mut errors);
    validate_item_instances(seed, context, &mut errors);
    validate_npcs(seed, context, &mut errors);
    validate_ecology_sites(seed, context, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::new(errors))
    }
}

fn validate_actors(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let mut actor_ids = HashMap::new();
    let mut character_ids = HashMap::new();
    let mut player_count = 0;
    for (index, actor) in seed.actors.iter().enumerate() {
        let prefix = format!("actors[{index}]");
        if actor.id.as_str().trim().is_empty() {
            errors.push(format!("{prefix}.id must be non-empty"));
        } else if let Some(previous) = actor_ids.insert(actor.id.as_str(), index) {
            errors.push(format!("{prefix}.id duplicates actors[{previous}].id"));
        }
        let Some(actor_kind) = context.actor_definition_kind(&actor.actor_definition_id) else {
            errors.push(format!(
                "{prefix}.actor_definition_id references unknown or unselected actor definition {:?}",
                actor.actor_definition_id
            ));
            continue;
        };
        let has_character = actor.character.is_some() || actor.starter_character.is_some();
        let uses_character_alignment = context
            .actor_definition_uses_character_alignment(&actor.actor_definition_id)
            .unwrap_or(false);
        if uses_character_alignment && !has_character {
            errors.push(format!(
                "{prefix} actor definition alignment_source character requires a character-backed actor"
            ));
        }
        if has_character && !uses_character_alignment {
            errors.push(format!(
                "{prefix} actor definition alignment_source must be character for a character-backed actor"
            ));
        }
        if actor.character.is_some() && actor.starter_character.is_some() {
            errors.push(format!(
                "{prefix} must not contain both character and starter_character"
            ));
        }
        match (&actor.character_id, has_character) {
            (None, true) => errors.push(format!(
                "{prefix}.character_id is required when a character role is present"
            )),
            (Some(_), false) => errors.push(format!(
                "{prefix}.character_id is only valid when character is present"
            )),
            _ => {}
        }
        if let Some(character_id) = &actor.character_id {
            let id = character_id.as_str();
            if id.trim().is_empty() {
                errors.push(format!("{prefix}.character_id must be non-empty"));
            } else if id == actor.id.as_str() {
                errors.push(format!(
                    "{prefix}.character_id must differ from transient actor id"
                ));
            } else if let Some(previous) = character_ids.insert(id, index) {
                errors.push(format!(
                    "{prefix}.character_id duplicates actors[{previous}].character_id"
                ));
            }
        }
        if actor_kind == ActorKind::Player {
            player_count += 1;
        }
        match (actor_kind, actor.npc.as_ref()) {
            (ActorKind::Npc, None) => errors.push(format!("{prefix}.npc is required for NPCs")),
            (ActorKind::Npc, Some(_)) => {}
            (_, Some(_)) => errors.push(format!("{prefix}.npc is only valid for NPCs")),
            _ => {}
        }
        validate_world_position(
            context,
            &actor.location,
            &format!("{prefix}.location"),
            errors,
        );
        validate_character(actor, actor_kind, context, index, errors);
        validate_starter(actor, actor_kind, seed, context, index, errors);
        validate_active_effects(
            &actor.active_effects,
            context,
            &format!("{prefix}.active_effects"),
            errors,
        );
        validate_burden_strength(actor, context, &prefix, errors);
    }
    if player_count == 0 {
        errors.push("actors must contain at least one player".to_string());
    }
}

fn validate_ecology_sites(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let mut site_ids = HashSet::new();
    let actor_ids = seed
        .actors
        .iter()
        .map(|actor| actor.id.as_str())
        .collect::<HashSet<_>>();
    for (index, site) in seed.ecology_sites.iter().enumerate() {
        let prefix = format!("ecology_sites[{index}]");
        if site.id.trim().is_empty() {
            errors.push(format!("{prefix}.id must be non-empty"));
        } else if !site_ids.insert(site.id.as_str()) {
            errors.push(format!("{prefix}.id must be unique"));
        }
        let Some(group) = context.ecology_group(&site.source) else {
            errors.push(format!(
                "{prefix}.source references unknown or unselected ecology definition"
            ));
            continue;
        };
        let expected = group
            .member_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let actual = site
            .member_locations
            .keys()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if actual != expected {
            errors.push(format!(
                "{prefix}.member_locations keys must exactly equal spawn-group members"
            ));
        }
        for (member_id, location) in &site.member_locations {
            validate_world_position(
                context,
                location,
                &format!("{prefix}.member_locations[{member_id:?}]"),
                errors,
            );
            let generated_id = format!("ecology:{}:{}:0", site.id, member_id);
            if actor_ids.contains(generated_id.as_str()) {
                errors.push(format!(
                    "{prefix} generation-zero actor ID collides with an explicit actor"
                ));
            }
        }
    }
}

fn validate_burden_strength(
    actor: &ActorSeedDef,
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let Some(attributes) = actor.effective_attributes() else {
        return;
    };
    let strength = u64::try_from(attributes.strength).unwrap_or(0);
    for (name, per_strength) in [
        (
            "lightly_loaded_max_per_strength",
            context.burden_limits_per_strength()[0],
        ),
        (
            "moderately_loaded_max_per_strength",
            context.burden_limits_per_strength()[1],
        ),
        (
            "heavily_loaded_max_per_strength",
            context.burden_limits_per_strength()[2],
        ),
    ] {
        if per_strength.checked_mul(strength).is_none() {
            errors.push(format!(
                "rules.burden.{name} * {prefix} effective character strength must not overflow"
            ));
        }
    }
}

fn validate_world_position(
    context: &impl WorldSeedValidationContext,
    location: &WorldPosition,
    label: &str,
    errors: &mut Vec<String>,
) {
    match context.world_position_status(location) {
        None => errors.push(format!(
            "{label} realm/level does not exist in the selected world template"
        )),
        Some(SeedWorldPositionStatus::OutOfBounds) => {
            errors.push(format!("{label} is out of bounds at {}", location.label()))
        }
        Some(SeedWorldPositionStatus::Blocked) => errors.push(format!(
            "{label} is not traversable at {}",
            location.label()
        )),
        Some(SeedWorldPositionStatus::Passable) => {}
    }
}

fn validate_character(
    actor: &ActorSeedDef,
    actor_kind: ActorKind,
    context: &impl WorldSeedValidationContext,
    index: usize,
    errors: &mut Vec<String>,
) {
    let Some(character) = &actor.character else {
        return;
    };
    let prefix = format!("actors[{index}].character");
    if actor_kind != ActorKind::Player {
        errors.push(format!("{prefix} is only valid for players"));
        return;
    }
    validate_skill_ledger(
        &character.skill_ledger,
        &character.identity.current_class_id,
        context,
        &format!("{prefix}.skill_ledger"),
        errors,
    );
    validate_attributes_and_resources(character, &prefix, errors);
    validate_progression(
        character.progression.level,
        character.progression.experience,
        &character.identity.current_class_id,
        context,
        &prefix,
        errors,
    );
}

fn validate_attributes_and_resources(
    character: &crate::model::CharacterSheetV1,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    for (name, value) in [
        ("strength", character.attributes.strength),
        ("dexterity", character.attributes.dexterity),
        ("constitution", character.attributes.constitution),
        ("intelligence", character.attributes.intelligence),
        ("wisdom", character.attributes.wisdom),
        ("charisma", character.attributes.charisma),
    ] {
        if !(3..=18).contains(&value) {
            errors.push(format!(
                "{prefix}.attributes.{name} must be between 3 and 18, got {value}"
            ));
        }
    }
    let resources = &character.resources;
    for (name, value) in [
        ("hp", resources.hp),
        ("max_hp", resources.max_hp),
        ("peak_hp", resources.peak_hp),
        ("mp", resources.mp),
        ("max_mp", resources.max_mp),
        ("stamina", resources.stamina),
        ("max_stamina", resources.max_stamina),
    ] {
        if value < 0 {
            errors.push(format!("{prefix}.resources.{name} must be non-negative"));
        }
    }
    if resources.hp <= 0 || resources.max_hp <= 0 {
        errors.push(format!(
            "{prefix}.resources.hp and max_hp must be positive for a living character"
        ));
    }
    if resources.hp > resources.max_hp {
        errors.push(format!("{prefix}.resources.hp must not exceed max_hp"));
    }
    if resources.max_hp > resources.peak_hp {
        errors.push(format!("{prefix}.resources.max_hp must not exceed peak_hp"));
    }
    if resources.mp > resources.max_mp {
        errors.push(format!("{prefix}.resources.mp must not exceed max_mp"));
    }
    if resources.max_stamina <= 0 {
        errors.push(format!(
            "{prefix}.resources.max_stamina must be positive for a living character"
        ));
    }
    if resources.stamina > resources.max_stamina {
        errors.push(format!(
            "{prefix}.resources.stamina must not exceed max_stamina"
        ));
    }
    if character.physical_attribute_adds.strength_adds < 0
        || character.physical_attribute_adds.dexterity_adds < 0
    {
        errors.push(format!(
            "{prefix}.physical_attribute_adds values must be non-negative"
        ));
    }
    for (index, entry) in character.promotion_history.iter().enumerate() {
        if entry.level < 1 {
            errors.push(format!(
                "{prefix}.promotion_history[{index}].level must be >= 1"
            ));
        }
    }
}

fn validate_progression(
    level: i32,
    experience: i64,
    class_id: &str,
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let thresholds = context.progression_thresholds();
    if let (Some(first), Some(last)) = (thresholds.first(), thresholds.last()) {
        if level < first.0 || level > last.0 {
            errors.push(format!(
                "{prefix}.progression.level must be within authored threshold range"
            ));
        } else if thresholds
            .iter()
            .rev()
            .find(|(_, threshold)| experience >= *threshold)
            .is_none_or(|(earned, _)| level > *earned)
        {
            errors.push(format!(
                "{prefix}.progression.level must not exceed the XP-earned level"
            ));
        }
    }
    if experience < 0 {
        errors.push(format!("{prefix}.progression.experience must be >= 0"));
    }
    if !context.progression_profile_exists(class_id) {
        errors.push(format!(
            "rules.progression.growth_profiles must contain class_id {class_id:?}"
        ));
    }
}

fn validate_skill_ledger(
    entries: &[SkillEntry],
    class_id: &str,
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    let mut tracks = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let label = format!("{prefix}[{index}]");
        if entry.track_id.trim().is_empty() {
            errors.push(format!("{label}.track_id must be non-empty"));
        } else if !tracks.insert(entry.track_id.as_str()) {
            errors.push(format!("{label}.track_id must be unique"));
        }
        if !entry.is_valid_position() {
            errors.push(format!(
                "{label} must use level 0/critique 0 or level 1..=19/critique 0..=10"
            ));
        }
        if !entry.has_valid_learning_rate() {
            errors.push(format!("{label}.learning_rate must be positive"));
        } else if entry.learning_rate < context.base_learning_rate() {
            errors.push(format!(
                "{label}.learning_rate must be at least rules.skills.base_learning_rate"
            ));
        }
        let expected_magic = match class_id {
            "wizard" => Some("wizard_magic"),
            "thaumaturge" => Some("thaumaturge_magic"),
            "thief" => Some("thief_magic"),
            _ => None,
        };
        if matches!(
            entry.track_id.as_str(),
            "wizard_magic" | "thaumaturge_magic" | "thief_magic" | "knight_magic"
        ) && expected_magic != Some(entry.track_id.as_str())
        {
            errors.push(format!(
                "{label}.track_id is not a magic skill track for class {class_id:?}"
            ));
        }
        if let Some(catalog) = context.skill_catalog() {
            if catalog.track(&entry.track_id).is_none() {
                errors.push(format!(
                    "{label}.track_id references unknown skill catalog track {:?}",
                    entry.track_id
                ));
            } else if !catalog.track_is_eligible_for_class(&entry.track_id, class_id) {
                errors.push(format!(
                    "{label}.track_id is not eligible for class {class_id:?}"
                ));
            }
        }
    }
}

fn validate_starter(
    actor: &ActorSeedDef,
    actor_kind: ActorKind,
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    actor_index: usize,
    errors: &mut Vec<String>,
) {
    let Some(starter) = &actor.starter_character else {
        return;
    };
    let prefix = format!("actors[{actor_index}].starter_character");
    if context.boundary_policy() == ContentBoundaryPolicy::Clean {
        errors.push(format!(
            "{prefix} is only valid in an internal_parity_fixture"
        ));
    }
    if actor_kind != ActorKind::Player {
        errors.push(format!("{prefix} is only valid for players"));
        return;
    }
    starter.validate_intrinsic(&prefix, errors);
    validate_skill_ledger(
        &starter.initial_skills,
        starter.current_class_id(),
        context,
        &format!("{prefix}.initial_skills"),
        errors,
    );
    for (index, known) in starter.initial_known_spells.iter().enumerate() {
        let label = format!("{prefix}.initial_known_spells[{index}]");
        match context.spell(&known.spell_id) {
            Some(spell) if spell.lane.as_deref() == Some(known.lane.as_str()) => {}
            Some(_) => errors.push(format!("{label}.lane must match the referenced spell lane")),
            None => errors.push(format!(
                "{label}.spell_id references unknown spell {:?}",
                known.spell_id
            )),
        }
    }
    validate_progression(
        starter.progression.level,
        starter.progression.experience,
        starter.current_class_id(),
        context,
        &prefix,
        errors,
    );
    validate_starter_loadout(actor, starter, seed, actor_index, errors);
}

fn validate_starter_loadout(
    actor: &ActorSeedDef,
    starter: &StarterCharacterDef,
    seed: &WorldSeedDef,
    actor_index: usize,
    errors: &mut Vec<String>,
) {
    let prefix = format!("actors[{actor_index}].starter_character.loadout");
    if actor.carried.gold != starter.loadout.gold {
        errors.push(format!(
            "actors[{actor_index}].carried.gold must equal {prefix}.gold"
        ));
    }
    let positions = actor
        .carried
        .items
        .iter()
        .map(|item| (item.item_instance_id.as_str(), item.position))
        .collect::<HashMap<_, _>>();
    let expected = starter
        .expected_carried_instance_ids()
        .into_iter()
        .collect::<HashSet<_>>();
    if positions.keys().copied().collect::<HashSet<_>>() != expected {
        errors.push(format!(
            "actors[{actor_index}].carried.items must equal the starter resolved loadout"
        ));
    }
    let mut check = |instance_id: &str,
                     definition_id: &str,
                     position_ok: fn(CarriedPosition) -> bool,
                     label: String| {
        let Some(instance) = seed.item_instances.get(instance_id) else {
            errors.push(format!(
                "{label}.item_instance_id references unknown item instance {instance_id:?}"
            ));
            return;
        };
        if instance.definition_id != definition_id {
            errors.push(format!(
                "{label}.item_definition_id does not match item instance definition"
            ));
        }
        match positions.get(instance_id).copied() {
            Some(position) if position_ok(position) => {}
            Some(position) => errors.push(format!(
                "{label}.item_instance_id has invalid carried position {:?}",
                position.label()
            )),
            None => errors.push(format!(
                "{label}.item_instance_id is not present in the actor carried layout"
            )),
        }
    };
    check(
        &starter.loadout.right_hand.item_instance_id,
        &starter.loadout.right_hand.item_definition_id,
        |position| position == CarriedPosition::RightHand,
        format!("{prefix}.right_hand"),
    );
    for (index, row) in starter.loadout.ordered_belt.iter().enumerate() {
        check(
            &row.item_instance_id,
            &row.item_definition_id,
            CarriedPosition::is_belt,
            format!("{prefix}.ordered_belt[{index}]"),
        );
    }
    check(
        &starter.loadout.inner_armor.item_instance_id,
        &starter.loadout.inner_armor.item_definition_id,
        |position| position == CarriedPosition::InnerArmor,
        format!("{prefix}.inner_armor"),
    );
    if let Some(book) = &starter.loadout.spell_book {
        check(
            &book.item_instance_id,
            &book.item_definition_id,
            CarriedPosition::is_sack_item,
            format!("{prefix}.spell_book"),
        );
    }
}

fn validate_active_effects(
    effects: &[ActiveEffectDef],
    context: &impl WorldSeedValidationContext,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    const SOURCE_KINDS: &[&str] = &["actor", "fixture", "item", "spell"];
    const STACKING: &[&str] = &["replace_same_kind", "stack_instance", "refresh_duration"];
    let mut instances = HashMap::new();
    for (index, effect) in effects.iter().enumerate() {
        let label = format!("{prefix}[{index}]");
        if effect.instance_id.trim().is_empty() {
            errors.push(format!("{label}.instance_id must be non-empty"));
        } else if let Some(previous) = instances.insert(effect.instance_id.as_str(), index) {
            errors.push(format!(
                "{label}.instance_id duplicates {prefix}[{previous}].instance_id"
            ));
        }
        if effect.effect_id.trim().is_empty() {
            errors.push(format!("{label}.effect_id must be non-empty"));
        }
        if !SOURCE_KINDS.contains(&effect.source.kind.as_str()) {
            errors.push(format!("{label}.source.kind is invalid"));
        }
        if effect.source.id.trim().is_empty() {
            errors.push(format!("{label}.source.id must be non-empty"));
        }
        if effect.kind.trim().is_empty() {
            errors.push(format!("{label}.kind must be non-empty"));
        }
        if effect.tags.iter().any(|tag| tag.trim().is_empty()) {
            errors.push(format!("{label}.tags must contain non-empty strings"));
        }
        let mut resistance_tags = HashSet::new();
        for (boost_index, boost) in effect.resistance_boosts.iter().enumerate() {
            if boost.tag.trim().is_empty() {
                errors.push(format!(
                    "{label}.resistance_boosts[{boost_index}].tag must be non-empty"
                ));
            }
            if boost.bonus_twentieths == 0
                || boost.bonus_twentieths > context.magic_resistance_denominator()
            {
                errors.push(format!(
                    "{label}.resistance_boosts[{boost_index}].bonus_twentieths must be in range"
                ));
            }
            if !resistance_tags.insert(boost.tag.as_str()) {
                errors.push(format!("{label}.resistance_boosts tags must be unique"));
            }
        }
        if effect.potency < 0 {
            errors.push(format!("{label}.potency must be non-negative"));
        }
        if effect.remaining_rounds.is_some_and(|rounds| rounds <= 0) {
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
            errors.push(format!("{label}.stacking is invalid"));
        }
        if effect.start_delay_rounds < 0 {
            errors.push(format!("{label}.start_delay_rounds must be non-negative"));
        }
        if effect.tick_interval_rounds <= 0 {
            errors.push(format!("{label}.tick_interval_rounds must be positive"));
        }
    }
}

fn validate_ground_items(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    for (index, ground) in seed.ground_items.iter().enumerate() {
        validate_world_position(
            context,
            &ground.location,
            &format!("ground_items[{index}].location"),
            errors,
        );
    }
}

fn validate_services(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let mut instance_ids = HashMap::new();
    let mut inventories = HashMap::new();
    let mut promotion_keys = HashMap::new();
    let mut service_grant_ids = HashMap::new();
    let mut spell_teaching_keys = HashMap::new();
    for (index, inventory) in seed.merchant_inventories.iter().enumerate() {
        let key = (
            inventory.service_instance_id.as_str(),
            inventory.capability_id.as_str(),
        );
        if let Some(previous) = inventories.insert(key, index) {
            errors.push(format!(
                "merchant_inventories[{index}] duplicates merchant_inventories[{previous}] service/capability"
            ));
        }
    }
    for (index, instance) in seed.service_instances.iter().enumerate() {
        let label = format!("service_instances[{index}]");
        if instance.id.trim().is_empty() {
            errors.push(format!("{label}.id must be non-empty"));
        } else if let Some(previous) = instance_ids.insert(instance.id.as_str(), index) {
            errors.push(format!(
                "{label}.id duplicates service_instances[{previous}].id"
            ));
        }
        if !context.service_definition_exists(&instance.service_definition_id) {
            errors.push(format!(
                "{label}.service_definition_id references unknown selected service definition {:?}",
                instance.service_definition_id
            ));
        }
        validate_world_position(
            context,
            &instance.location,
            &format!("{label}.location"),
            errors,
        );
        for promotion in context.promotion_capabilities(&instance.service_definition_id) {
            let key = (
                instance.location.clone(),
                promotion.target_class_id.to_string(),
            );
            if let Some((previous_instance, previous_capability)) =
                promotion_keys.insert(key, (index, promotion.id.to_string()))
            {
                errors.push(format!(
                    "{label} promotion capability {:?} duplicates service_instances[{previous_instance}] promotion capability {previous_capability:?} room/position/target",
                    promotion.id
                ));
            }
        }
        for teaching in context.spell_teaching_pairs(&instance.service_definition_id) {
            let key = (
                instance.location.clone(),
                teaching.class_id.to_string(),
                teaching.spell_id.to_string(),
            );
            if let Some((previous_instance, previous_capability)) =
                spell_teaching_keys.insert(key, (index, teaching.capability_id.to_string()))
            {
                errors.push(format!(
                    "{label} spell teaching capability {:?} duplicates service_instances[{previous_instance}] spell teaching capability {previous_capability:?} room/position/class/spell",
                    teaching.capability_id
                ));
            }
        }
        for item_instance_id in
            context.service_grant_item_instance_ids(&instance.service_definition_id)
        {
            if seed.item_instances.contains_key(item_instance_id) {
                errors.push(format!(
                    "{label} service definition item grant {item_instance_id:?} must not already be registered"
                ));
            }
            if let Some(previous_instance) =
                service_grant_ids.insert(item_instance_id.to_string(), index)
            {
                errors.push(format!(
                    "{label} service definition item grant {item_instance_id:?} duplicates service_instances[{previous_instance}] service definition item grant"
                ));
            }
        }
        for merchant in context.merchant_capabilities(&instance.service_definition_id) {
            if !inventories.contains_key(&(instance.id.as_str(), merchant.id)) {
                errors.push(format!(
                    "{label} merchant capability {:?} requires exactly one merchant inventory seed",
                    merchant.id
                ));
            }
            if let Some(multiplier) = context
                .merchant_pawn_listing_multiplier(&instance.service_definition_id, merchant.id)
            {
                for (item_instance_id, item_instance) in &seed.item_instances {
                    let Some(item) = context.item(&item_instance.definition_id) else {
                        continue;
                    };
                    let Some(unit_value) = item.economy.unit_value_gold else {
                        continue;
                    };
                    let total = unit_value
                        .checked_mul(u64::from(item_instance.quantity))
                        .and_then(|value| value.checked_mul(u64::from(multiplier)));
                    if total.is_none_or(|value| value > i64::MAX as u64) {
                        errors.push(format!(
                            "{label} merchant capability {:?} player_sales cannot price item instance {item_instance_id:?} within signed carried gold",
                            merchant.id
                        ));
                    }
                }
            }
        }
    }
    for (index, inventory) in seed.merchant_inventories.iter().enumerate() {
        let label = format!("merchant_inventories[{index}]");
        let Some(instance) = seed
            .service_instances
            .iter()
            .find(|instance| instance.id == inventory.service_instance_id)
        else {
            errors.push(format!(
                "{label}.service_instance_id references unknown service instance {:?}",
                inventory.service_instance_id
            ));
            continue;
        };
        if context
            .service_capability_kind(&instance.service_definition_id, &inventory.capability_id)
            != Some(SeedServiceCapabilityKind::Merchant)
        {
            errors.push(format!(
                "{label}.capability_id must reference a merchant capability"
            ));
            continue;
        }
        let accepts_sales = context
            .merchant_capabilities(&instance.service_definition_id)
            .into_iter()
            .find(|merchant| merchant.id == inventory.capability_id)
            .is_some_and(|merchant| merchant.accepts_player_sales);
        if inventory.stock.is_empty() && !accepts_sales {
            errors.push(format!(
                "{label}.stock must be non-empty when player_sales is null"
            ));
        }
        let mut stock_ids = HashSet::new();
        for (stock_index, stock) in inventory.stock.iter().enumerate() {
            let stock_label = format!("{label}.stock[{stock_index}]");
            if stock.item_instance_id.trim().is_empty() {
                errors.push(format!("{stock_label}.item_instance_id must be non-empty"));
            } else if !stock_ids.insert(stock.item_instance_id.as_str()) {
                errors.push(format!(
                    "{stock_label}.item_instance_id must be unique within the inventory"
                ));
            }
            if stock.price_gold <= 0 {
                errors.push(format!("{stock_label}.price_gold must be positive"));
            }
            if let Some(item_instance) = seed.item_instances.get(&stock.item_instance_id) {
                if !matches!(item_instance.binding, ItemBindingState::Unrestricted) {
                    errors.push(format!(
                        "{stock_label}.item_instance_id must reference an unrestricted item"
                    ));
                }
                if context
                    .item(&item_instance.definition_id)
                    .is_some_and(|item| !item.valid_placements.contains(&ItemPlacementKind::Sack))
                {
                    errors.push(format!(
                        "{stock_label}.item_instance_id definition must permit sack placement"
                    ));
                }
            } else {
                errors.push(format!(
                    "{stock_label}.item_instance_id references unknown item instance {:?}",
                    stock.item_instance_id
                ));
            }
        }
    }
}

fn validate_item_instances(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let character_ids = seed
        .actors
        .iter()
        .filter_map(|actor| actor.character_id.as_ref())
        .collect::<HashSet<_>>();
    for (instance_id, instance) in &seed.item_instances {
        let label = format!("item_instances[{instance_id:?}]");
        if instance_id.trim().is_empty() {
            errors.push("item_instances keys must be non-empty".to_string());
        }
        if instance_id.starts_with("summon:") {
            errors.push(format!(
                "item instance {instance_id:?} must not use reserved prefix \"summon:\""
            ));
        }
        if instance.quantity == 0 {
            errors.push(format!("{label}.quantity must be positive"));
        }
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
        let Some(item) = context.item(&instance.definition_id) else {
            errors.push(format!(
                "{label}.definition_id references unknown item definition {:?}",
                instance.definition_id
            ));
            continue;
        };
        if item
            .capability
            .and_then(|capability| capability.spell_book_for.as_ref())
            .is_some()
        {
            if instance.quantity != 1 {
                errors.push(format!("{label}.quantity must be 1 for a Spell Book"));
            }
            match &instance.binding {
                ItemBindingState::Bound { character_id }
                    if character_ids.contains(character_id) => {}
                ItemBindingState::Bound { .. } => errors.push(format!(
                    "{label}.binding.character_id references no scenario character"
                )),
                _ => errors.push(format!("{label}.binding must be bound for a Spell Book")),
            }
        }
        if item
            .economy
            .unit_value_gold
            .is_some_and(|unit| unit.checked_mul(u64::from(instance.quantity)).is_none())
        {
            errors.push(format!(
                "{label}.quantity * unit_value_gold must not overflow"
            ));
        }
        if item
            .economy
            .unit_burden
            .checked_mul(u64::from(instance.quantity))
            .is_none()
        {
            errors.push(format!("{label}.quantity * unit_burden must not overflow"));
        }
    }

    let mut owners = HashMap::<&str, String>::new();
    for (actor_index, actor) in seed.actors.iter().enumerate() {
        if actor.carried.gold.left_hand < 0
            || actor.carried.gold.right_hand < 0
            || actor.carried.gold.sack < 0
        {
            errors.push(format!(
                "actors[{actor_index}].carried.gold values must be non-negative"
            ));
        }
        if actor.carried.gold.checked_total().is_none() {
            errors.push(format!(
                "actors[{actor_index}].carried.gold total must fit a signed 64-bit integer"
            ));
        }
        let mut positions = HashMap::new();
        for (item_index, positioned) in actor.carried.items.iter().enumerate() {
            let label =
                format!("actors[{actor_index}].carried.items[{item_index}].item_instance_id");
            record_owner(
                &seed.item_instances,
                &mut owners,
                &positioned.item_instance_id,
                &label,
                errors,
            );
            if let Some(previous) = positions.insert(positioned.position, item_index) {
                errors.push(format!(
                    "actors[{actor_index}].carried.items[{item_index}].position duplicates actors[{actor_index}].carried.items[{previous}].position"
                ));
            }
            if let Some(instance) = seed.item_instances.get(&positioned.item_instance_id)
                && let Some(item) = context.item(&instance.definition_id)
                && !item
                    .valid_placements
                    .contains(&positioned.position.placement_kind())
            {
                errors.push(format!(
                    "{label} cannot occupy carried position {:?}",
                    positioned.position.label()
                ));
            }
            if let Some(instance) = seed.item_instances.get(&positioned.item_instance_id)
                && !positioned.position.is_sack_item()
                && instance.quantity != 1
            {
                errors.push(format!("{label} must have quantity 1 outside the sack"));
            }
        }
        for (position, amount) in [
            (CarriedPosition::LeftHand, actor.carried.gold.left_hand),
            (CarriedPosition::RightHand, actor.carried.gold.right_hand),
        ] {
            if amount > 0 && positions.contains_key(&position) {
                errors.push(format!(
                    "actors[{actor_index}].carried cannot place an item and gold in {}",
                    position.label()
                ));
            }
        }
    }
    for (index, ground) in seed.ground_items.iter().enumerate() {
        record_owner(
            &seed.item_instances,
            &mut owners,
            &ground.item_instance_id,
            &format!("ground_items[{index}].item_instance_id"),
            errors,
        );
    }
    for (inventory_index, inventory) in seed.merchant_inventories.iter().enumerate() {
        for (stock_index, stock) in inventory.stock.iter().enumerate() {
            record_owner(
                &seed.item_instances,
                &mut owners,
                &stock.item_instance_id,
                &format!(
                    "merchant_inventories[{inventory_index}].stock[{stock_index}].item_instance_id"
                ),
                errors,
            );
        }
    }
    for instance_id in seed.item_instances.keys() {
        if !owners.contains_key(instance_id.as_str()) {
            errors.push(format!(
                "item instance {instance_id:?} has no owner or location"
            ));
        }
    }
}

fn record_owner<'a>(
    registry: &'a BTreeMap<String, ItemInstanceSeedDef>,
    owners: &mut HashMap<&'a str, String>,
    instance_id: &'a str,
    label: &str,
    errors: &mut Vec<String>,
) {
    if instance_id.trim().is_empty() {
        errors.push(format!("{label} must be non-empty"));
    } else if !registry.contains_key(instance_id) {
        errors.push(format!(
            "{label} references unknown item instance {instance_id:?}"
        ));
    } else if let Some(previous) = owners.insert(instance_id, label.to_string()) {
        errors.push(format!(
            "item instance {instance_id:?} is referenced more than once ({previous} and {label})"
        ));
    }
}

fn validate_npcs(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    for (actor_index, actor) in seed.actors.iter().enumerate() {
        let Some(npc) = &actor.npc else {
            continue;
        };
        let prefix = format!("actors[{actor_index}].npc");
        if npc.follow_cadence_units == 0 {
            errors.push(format!("{prefix}.follow_cadence_units must be positive"));
        }
        if npc.interactions.is_empty() {
            errors.push(format!("{prefix}.interactions must be non-empty"));
        }
        let mut interaction_ids = HashMap::new();
        for (index, interaction) in npc.interactions.iter().enumerate() {
            let label = format!("{prefix}.interactions[{index}]");
            if let Some(previous) =
                interaction_ids.insert(interaction.transaction.id.as_str(), index)
            {
                errors.push(format!(
                    "{label}.transaction.id duplicates {prefix}.interactions[{previous}].transaction.id"
                ));
            }
            if interaction.response.trim().is_empty() {
                errors.push(format!("{label}.response must be non-empty"));
            }
            validate_transaction(
                &interaction.transaction,
                seed,
                context,
                &format!("{label}.transaction"),
                errors,
            );
            let accompanies = |npc_actor_id: &crate::model::ActorId| {
                interaction.transaction.requirements.iter().any(|requirement| {
                    matches!(requirement, TransactionRequirementDef::NpcAccompanying { npc_actor_id: required } if required == npc_actor_id)
                })
            };
            match &interaction.outcome {
                NpcInteractionOutcomeDef::Speak | NpcInteractionOutcomeDef::BeginFollow => {}
                NpcInteractionOutcomeDef::EndFollow | NpcInteractionOutcomeDef::Climb { .. } => {
                    if !accompanies(&actor.id) {
                        errors.push(format!(
                            "{label}.outcome requires npc_accompanying for the provider"
                        ));
                    }
                }
                NpcInteractionOutcomeDef::CompleteEscort { npc_actor_id } => {
                    if npc_actor_id == &actor.id {
                        errors.push(format!(
                            "{label}.outcome.npc_actor_id must differ from the provider"
                        ));
                    }
                    if !seed.actors.iter().any(|candidate| {
                        candidate.id == *npc_actor_id
                            && context.actor_definition_kind(&candidate.actor_definition_id)
                                == Some(ActorKind::Npc)
                    }) {
                        errors.push(format!(
                            "{label}.outcome.npc_actor_id references unknown NPC {npc_actor_id:?}"
                        ));
                    }
                    if !accompanies(npc_actor_id) {
                        errors.push(format!(
                            "{label}.outcome requires a matching npc_accompanying gate"
                        ));
                    }
                }
            }
        }
    }
}

fn validate_transaction(
    transaction: &TransactionDef,
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    label: &str,
    errors: &mut Vec<String>,
) {
    if transaction.id.trim().is_empty() {
        errors.push(format!("{label}.id must be non-empty"));
    }
    if transaction.label.trim().is_empty() {
        errors.push(format!("{label}.label must be non-empty"));
    }
    let mut requirement_keys = HashSet::new();
    let mut carried_item_requirement = None;
    let mut minimum_gold = None;
    let mut quest_gates = HashMap::<&str, Option<&str>>::new();
    for (index, requirement) in transaction.requirements.iter().enumerate() {
        let row = format!("{label}.requirements[{index}]");
        let key = match requirement {
            TransactionRequirementDef::CurrentClass { class_id } => {
                if class_id.trim().is_empty() {
                    errors.push(format!("{row}.class_id must be non-empty"));
                }
                if !TRANSACTION_CLASS_IDS.contains(&class_id.as_str()) {
                    errors.push(format!(
                        "{row}.class_id references unknown class {class_id:?}"
                    ));
                }
                "current_class".to_string()
            }
            TransactionRequirementDef::MinimumLevel { level } => {
                if *level <= 0 {
                    errors.push(format!("{row}.level must be positive"));
                }
                "minimum_level".to_string()
            }
            TransactionRequirementDef::ExactKarma { .. } => "exact_karma".to_string(),
            TransactionRequirementDef::ExactAlignment { .. } => "exact_alignment".to_string(),
            TransactionRequirementDef::MinimumSkillLevel { track_id, level } => {
                if track_id.trim().is_empty() {
                    errors.push(format!("{row}.track_id must be non-empty"));
                }
                if *level == 0 || *level > crate::model::MAX_SKILL_LEVEL {
                    errors.push(format!("{row}.level must be between 1 and 19"));
                }
                if context
                    .skill_catalog()
                    .and_then(|catalog| catalog.track(track_id))
                    .is_none()
                {
                    errors.push(format!(
                        "{row}.track_id references unknown skill track {track_id:?}"
                    ));
                }
                format!("minimum_skill_level:{track_id}")
            }
            TransactionRequirementDef::MinimumCarriedGold { amount } => {
                if *amount <= 0 {
                    errors.push(format!("{row}.amount must be positive"));
                }
                minimum_gold = Some(*amount);
                "minimum_carried_gold".to_string()
            }
            TransactionRequirementDef::CarriedItem {
                item_definition_id,
                quantity,
            } => {
                if context.item(item_definition_id).is_none() {
                    errors.push(format!(
                        "{row}.item_definition_id references unknown item definition {item_definition_id:?}"
                    ));
                }
                if *quantity == 0 {
                    errors.push(format!("{row}.quantity must be positive"));
                }
                if carried_item_requirement.is_some() {
                    errors.push(format!(
                        "{label} may contain at most one carried_item requirement"
                    ));
                }
                carried_item_requirement = Some((item_definition_id.as_str(), *quantity));
                format!("carried_item:{item_definition_id}")
            }
            TransactionRequirementDef::CarriedPositionEmpty { position } => {
                format!("carried_position_empty:{}", position.label())
            }
            TransactionRequirementDef::SpellUnknown { spell_id } => {
                if context.spell(spell_id).is_none() {
                    errors.push(format!(
                        "{row}.spell_id references unknown spell {spell_id:?}"
                    ));
                }
                format!("spell_unknown:{spell_id}")
            }
            TransactionRequirementDef::QuestUnstarted { quest_id } => {
                if !context.quest_exists(quest_id) {
                    errors.push(format!(
                        "{row}.quest_id references unknown quest {quest_id:?}"
                    ));
                }
                if quest_gates.insert(quest_id, None).is_some() {
                    errors.push(format!(
                        "{label} may contain only one quest gate for {quest_id:?}"
                    ));
                }
                format!("quest_unstarted:{quest_id}")
            }
            TransactionRequirementDef::QuestAtStage { quest_id, stage_id } => {
                if !context.quest_stage_exists(quest_id, stage_id) {
                    errors.push(format!(
                        "{row} references unknown quest/stage {quest_id:?}/{stage_id:?}"
                    ));
                }
                if quest_gates.insert(quest_id, Some(stage_id)).is_some() {
                    errors.push(format!(
                        "{label} may contain only one quest gate for {quest_id:?}"
                    ));
                }
                format!("quest_at_stage:{quest_id}")
            }
            TransactionRequirementDef::NpcAccompanying { npc_actor_id } => {
                if !seed.actors.iter().any(|actor| {
                    actor.id == *npc_actor_id
                        && context.actor_definition_kind(&actor.actor_definition_id)
                            == Some(ActorKind::Npc)
                }) {
                    errors.push(format!(
                        "{row}.npc_actor_id references unknown NPC {npc_actor_id:?}"
                    ));
                }
                format!("npc_accompanying:{npc_actor_id}")
            }
        };
        if !requirement_keys.insert(key) {
            errors.push(format!("{row} duplicates a requirement kind/target"));
        }
    }
    let mut selected_item_cost = None;
    let mut carried_gold_costs = 0;
    for (index, cost) in transaction.costs.iter().enumerate() {
        let row = format!("{label}.costs[{index}]");
        match cost {
            TransactionCostDef::CarriedGold { amount } => {
                carried_gold_costs += 1;
                if *amount <= 0 {
                    errors.push(format!("{row}.amount must be positive"));
                }
                if minimum_gold.is_some_and(|minimum| *amount > minimum) {
                    errors.push(format!(
                        "{row}.amount must not exceed the minimum_carried_gold requirement"
                    ));
                }
            }
            TransactionCostDef::SelectedCarriedItem { quantity } => {
                if *quantity == 0 {
                    errors.push(format!("{row}.quantity must be positive"));
                }
                if selected_item_cost.is_some() {
                    errors.push(format!(
                        "{label} may contain at most one selected_carried_item cost"
                    ));
                }
                selected_item_cost = Some(*quantity);
            }
        }
    }
    if carried_gold_costs > 1 {
        errors.push(format!("{label} may contain at most one carried_gold cost"));
    }
    match (carried_item_requirement, selected_item_cost) {
        (None, Some(_)) => errors.push(format!(
            "{label} selected_carried_item cost requires a carried_item requirement"
        )),
        (Some((_, required)), Some(cost)) if cost > required => errors.push(format!(
            "{label} selected_carried_item cost must not exceed its carried_item requirement"
        )),
        _ => {}
    }
    let mut reward_ids = HashSet::new();
    let mut quest_reward_ids = HashSet::new();
    for (index, reward) in transaction.rewards.iter().enumerate() {
        let row = format!("{label}.rewards[{index}]");
        match reward {
            TransactionRewardDef::Item {
                item_instance_id,
                item_definition_id,
                position,
            } => {
                if item_instance_id.trim().is_empty() {
                    errors.push(format!("{row}.item_instance_id must be non-empty"));
                }
                if seed.item_instances.contains_key(item_instance_id) {
                    errors.push(format!(
                        "{row}.item_instance_id collides with an initial item instance"
                    ));
                }
                if !reward_ids.insert(format!("item:{item_instance_id}")) {
                    errors.push(format!("{row}.item_instance_id must be unique"));
                }
                match context.item(item_definition_id) {
                    None => errors.push(format!(
                        "{row}.item_definition_id references unknown item definition {item_definition_id:?}"
                    )),
                    Some(item)
                        if !item
                            .valid_placements
                            .contains(&position.placement_kind()) =>
                    {
                        errors.push(format!(
                            "{row}.position is invalid for item definition {item_definition_id:?}"
                        ));
                    }
                    Some(_) => {}
                }
            }
            TransactionRewardDef::Class {
                to_class_id,
                to_class_display,
            } => {
                if to_class_id.trim().is_empty() || to_class_display.trim().is_empty() {
                    errors.push(format!("{row} class identifiers must be non-empty"));
                }
                if !context.progression_profile_exists(to_class_id) {
                    errors.push(format!(
                        "{row}.to_class_id has no progression growth profile"
                    ));
                }
                errors.push(format!(
                    "{row}.kind class is legal only for class_promotion"
                ));
            }
            TransactionRewardDef::Spell { spell_id } => {
                if context.spell(spell_id).is_none() {
                    errors.push(format!(
                        "{row}.spell_id references unknown spell {spell_id:?}"
                    ));
                }
                if !reward_ids.insert(format!("spell:{spell_id}")) {
                    errors.push(format!("{row}.spell_id must be unique"));
                }
                errors.push(format!(
                    "{row}.kind spell is legal only for class_promotion"
                ));
            }
            TransactionRewardDef::Experience { amount } if *amount <= 0 => {
                errors.push(format!("{row}.amount must be positive"));
            }
            TransactionRewardDef::QuestStage { quest_id, stage_id } => {
                if !quest_reward_ids.insert(quest_id.as_str()) {
                    errors.push(format!(
                        "{label} may change quest {quest_id:?} at most once"
                    ));
                }
                if !context.quest_stage_exists(quest_id, stage_id) {
                    errors.push(format!(
                        "{row} references unknown quest/stage {quest_id:?}/{stage_id:?}"
                    ));
                }
                match quest_gates.get(quest_id.as_str()) {
                    Some(Some(required_stage)) if *required_stage == stage_id => {
                        errors.push(format!("{row}.stage_id must advance beyond its quest gate"))
                    }
                    Some(_) => {}
                    None => errors.push(format!(
                        "{row} requires exactly one quest gate for {quest_id:?}"
                    )),
                }
            }
            TransactionRewardDef::Experience { .. } => {}
        }
    }
}
