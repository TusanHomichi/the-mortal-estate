mod actors;
mod items;
mod npcs;
mod services;
use actors::*;
use items::*;
use npcs::*;
use services::*;

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
