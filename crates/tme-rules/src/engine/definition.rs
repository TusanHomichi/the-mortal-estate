//! The immutable half of a running game: the selected content catalog, the
//! world template it was validated against, and the content identity that ties
//! a checkpoint to the definition that produced it.
//!
//! Nothing here mutates. The mutable half lives on [`Engine`](super::Engine).

use crate::content::{
    ArmorDef, CatalogProfileKey, ContentBoundaryPolicy, ItemEconomyDef, SeedItemValidationView,
    SeedMerchantCapabilityView, SeedPromotionCapabilityView, SeedServiceCapabilityKind,
    SeedSpellTeachingPairView, SeedWorldPositionStatus, SkillCatalogDef, SpellDef, WeaponDef,
    WorldSeedValidationContext,
};
use crate::model::{
    ActorDefinition, BankDefinition, BankId, ItemCapability, ItemPlacementKind,
    LockerVaultDefinition, LockerVaultId, NavigationDef, ProfessionActionConfig, QuestDefinition,
    QuestId, QuestStageId, RealmState, ServiceCapability, ServiceDefinition, SpellCatalogEntry,
    SummonTemplate, TerrainState, WorldPosition, WorldRules,
};

use super::ContentIdentityV1;

#[derive(Debug, Clone)]
pub struct CatalogItem {
    pub name: String,
    pub kind: String,
    pub category: Option<String>,
    pub weapon: Option<WeaponDef>,
    pub armor: Option<ArmorDef>,
    pub valid_placements: Vec<ItemPlacementKind>,
    pub capability: Option<ItemCapability>,
    pub economy: ItemEconomyDef,
}
#[derive(Debug, Clone)]
pub struct GameCatalog {
    pub(in crate::engine) boundary_policy: ContentBoundaryPolicy,
    pub(in crate::engine) profile_key: CatalogProfileKey,
    pub(in crate::engine) rules: WorldRules,
    pub(in crate::engine) consumable_heals: std::collections::HashMap<String, i32>,
    pub(in crate::engine) item_catalog: std::collections::HashMap<String, CatalogItem>,
    pub(in crate::engine) skill_catalog: Option<SkillCatalogDef>,
    pub(in crate::engine) service_definitions: Vec<ServiceDefinition>,
    pub(in crate::engine) bank_definitions: std::collections::BTreeMap<BankId, BankDefinition>,
    pub(in crate::engine) locker_vault_definitions:
        std::collections::BTreeMap<LockerVaultId, LockerVaultDefinition>,
    pub(in crate::engine) quests: std::collections::BTreeMap<QuestId, QuestDefinition>,
    pub(in crate::engine) profession_actions: Vec<ProfessionActionConfig>,
    pub(in crate::engine) spells: std::collections::HashMap<String, SpellDef>,
    pub(in crate::engine) spell_catalog: std::collections::BTreeMap<String, SpellCatalogEntry>,
    pub(in crate::engine) summon_templates: std::collections::HashMap<String, SummonTemplate>,
    pub(in crate::engine) actor_definitions: std::collections::HashMap<String, ActorDefinition>,
    pub(in crate::engine) loot_tables:
        std::collections::HashMap<String, crate::content::LootTableDef>,
    pub(in crate::engine) spawn_groups:
        std::collections::HashMap<String, crate::content::SpawnGroupDef>,
    pub(in crate::engine) lair_definitions:
        std::collections::HashMap<String, crate::content::LairDefinitionDef>,
    pub(in crate::engine) terrains: std::collections::HashMap<String, TerrainState>,
}

#[derive(Debug, Clone)]
pub struct WorldTemplate {
    pub(in crate::engine) visual_manifest_digest: String,
    pub(in crate::engine) realms: std::collections::HashMap<String, RealmState>,
    pub(in crate::engine) arrivals: std::collections::HashMap<String, WorldPosition>,
    pub(in crate::engine) navigation: std::collections::HashMap<WorldPosition, Vec<NavigationDef>>,
}

#[derive(Debug, Clone)]
pub struct GameDefinition {
    pub(in crate::engine) catalog: GameCatalog,
    pub(in crate::engine) world_template: WorldTemplate,
    pub(in crate::engine) content_identity: ContentIdentityV1,
}

impl GameDefinition {
    pub fn catalog(&self) -> &GameCatalog {
        &self.catalog
    }

    pub fn world_template(&self) -> &WorldTemplate {
        &self.world_template
    }

    pub fn content_identity(&self) -> &ContentIdentityV1 {
        &self.content_identity
    }
}
impl WorldSeedValidationContext for GameDefinition {
    fn boundary_policy(&self) -> ContentBoundaryPolicy {
        self.catalog.boundary_policy
    }

    fn profile_key(&self) -> &CatalogProfileKey {
        &self.catalog.profile_key
    }

    fn world_position_status(&self, location: &WorldPosition) -> Option<SeedWorldPositionStatus> {
        let level = self
            .world_template
            .realms
            .get(&location.realm)?
            .levels
            .get(&location.level)?;
        let position = location.position;
        if position.x < 0
            || position.y < 0
            || position.x >= level.width
            || position.y >= level.height
        {
            return Some(SeedWorldPositionStatus::OutOfBounds);
        }
        let cell = &level.cells[position.y as usize][position.x as usize];
        let mut traversable = false;
        let mut blocked = false;
        for terrain_id in cell.iter().flatten() {
            match self.catalog.terrains.get(terrain_id) {
                Some(terrain) if terrain.unresolved || !terrain.passable => blocked = true,
                Some(_) => traversable = true,
                None => blocked = true,
            }
        }
        Some(if blocked || !traversable {
            SeedWorldPositionStatus::Blocked
        } else {
            SeedWorldPositionStatus::Passable
        })
    }

    fn progression_thresholds(&self) -> Vec<(i32, i64)> {
        self.catalog
            .rules
            .progression
            .level_thresholds
            .iter()
            .map(|threshold| (threshold.level, threshold.cumulative_experience))
            .collect()
    }

    fn progression_profile_exists(&self, class_id: &str) -> bool {
        self.catalog
            .rules
            .progression
            .growth_profiles
            .contains_key(class_id)
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

    fn actor_definition_kind(&self, definition_id: &str) -> Option<crate::model::ActorKind> {
        self.catalog
            .actor_definitions
            .get(definition_id)
            .map(|definition| definition.kind)
    }

    fn actor_definition_uses_character_alignment(&self, definition_id: &str) -> Option<bool> {
        self.catalog
            .actor_definitions
            .get(definition_id)
            .map(|definition| {
                matches!(
                    definition.social.alignment_source,
                    crate::model::SocialAlignmentSource::Character {}
                )
            })
    }

    fn ecology_group(
        &self,
        source: &crate::content::EcologySiteSourceDef,
    ) -> Option<crate::content::SeedEcologyGroupView> {
        let group = match source {
            crate::content::EcologySiteSourceDef::SpawnGroup { spawn_group_id } => {
                self.catalog.spawn_groups.get(spawn_group_id)
            }
            crate::content::EcologySiteSourceDef::Lair { lair_definition_id } => {
                let lair = self.catalog.lair_definitions.get(lair_definition_id)?;
                self.catalog.spawn_groups.get(&lair.spawn_group_id)
            }
        }?;
        Some(crate::content::SeedEcologyGroupView {
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
            .item_catalog
            .get(definition_id)
            .map(|item| SeedItemValidationView {
                valid_placements: &item.valid_placements,
                capability: item.capability.as_ref(),
                economy: &item.economy,
            })
    }

    fn spell(&self, spell_id: &str) -> Option<&SpellDef> {
        self.catalog.spells.get(spell_id)
    }

    fn quest_exists(&self, quest_id: &str) -> bool {
        self.catalog.quests.contains_key(&QuestId::new(quest_id))
    }

    fn quest_stage_exists(&self, quest_id: &str, stage_id: &str) -> bool {
        self.catalog
            .quests
            .get(&QuestId::new(quest_id))
            .is_some_and(|quest| quest.stages.contains_key(&QuestStageId::new(stage_id)))
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
            .find(|capability| service_capability_id(capability) == capability_id)
            .map(|capability| match capability {
                ServiceCapability::SkillTraining(_) => SeedServiceCapabilityKind::SkillTraining,
                ServiceCapability::SkillCritique(_) => SeedServiceCapabilityKind::SkillCritique,
                ServiceCapability::SpellTeaching(_) => SeedServiceCapabilityKind::SpellTeaching,
                ServiceCapability::ClassPromotion(_) => SeedServiceCapabilityKind::ClassPromotion,
                ServiceCapability::ServiceTransaction(_) => {
                    SeedServiceCapabilityKind::ServiceTransaction
                }
                ServiceCapability::Merchant(_) => SeedServiceCapabilityKind::Merchant,
                ServiceCapability::ItemService(_) => SeedServiceCapabilityKind::ItemService,
                ServiceCapability::Restoration(_) => SeedServiceCapabilityKind::Restoration,
                ServiceCapability::Bank(_) => SeedServiceCapabilityKind::Bank,
                ServiceCapability::Locker(_) => SeedServiceCapabilityKind::Locker,
            })
    }

    fn merchant_capabilities(&self, definition_id: &str) -> Vec<SeedMerchantCapabilityView<'_>> {
        self.catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
            .into_iter()
            .flat_map(|definition| &definition.capabilities)
            .filter_map(|capability| match capability {
                ServiceCapability::Merchant(merchant) => Some(SeedMerchantCapabilityView {
                    id: &merchant.id,
                    accepts_player_sales: merchant.player_sales.is_some(),
                }),
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
                ServiceCapability::Merchant(merchant) if merchant.id == capability_id => merchant
                    .player_sales
                    .as_ref()
                    .map(|policy| policy.pawn_listing_multiplier),
                _ => None,
            })
    }

    fn promotion_capabilities(&self, definition_id: &str) -> Vec<SeedPromotionCapabilityView<'_>> {
        self.catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
            .into_iter()
            .flat_map(|definition| &definition.capabilities)
            .filter_map(|capability| match capability {
                ServiceCapability::ClassPromotion(promotion) => promotion
                    .transaction
                    .rewards
                    .iter()
                    .find_map(|reward| match reward {
                        crate::model::TransactionReward::Class { to_class_id, .. } => {
                            Some(to_class_id)
                        }
                        _ => None,
                    })
                    .map(|target_class_id| SeedPromotionCapabilityView {
                        id: &promotion.id,
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
            let ServiceCapability::SpellTeaching(teaching) = capability else {
                continue;
            };
            let Some(ServiceCapability::SkillTraining(training)) =
                definition.capabilities.iter().find(|candidate| {
                    service_capability_id(candidate) == teaching.training_capability_id
                })
            else {
                continue;
            };
            let magic_offers = training
                .offers
                .iter()
                .filter(|offer| crate::content::is_spell_teaching_lane(&offer.track_id))
                .collect::<Vec<_>>();
            let [offer] = magic_offers.as_slice() else {
                continue;
            };
            for spell in &teaching.teachings {
                for class_id in &offer.eligible_class_ids {
                    pairs.push(SeedSpellTeachingPairView {
                        capability_id: &teaching.id,
                        class_id,
                        spell_id: &spell.spell_id,
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
        let mut item_instance_ids = Vec::new();
        for capability in &definition.capabilities {
            match capability {
                ServiceCapability::ClassPromotion(promotion) => {
                    item_instance_ids
                        .extend(transaction_grant_item_instance_ids(&promotion.transaction));
                }
                ServiceCapability::ServiceTransaction(service) => {
                    for transaction in &service.transactions {
                        item_instance_ids.extend(transaction_grant_item_instance_ids(transaction));
                    }
                }
                ServiceCapability::Restoration(restoration) => {
                    for operation in &restoration.operations {
                        item_instance_ids
                            .extend(transaction_grant_item_instance_ids(&operation.transaction));
                    }
                }
                _ => {}
            }
        }
        item_instance_ids
    }
}

fn transaction_grant_item_instance_ids(
    transaction: &crate::model::Transaction,
) -> impl Iterator<Item = &str> {
    transaction
        .rewards
        .iter()
        .filter_map(|reward| match reward {
            crate::model::TransactionReward::Item {
                item_instance_id, ..
            } => Some(item_instance_id.as_str()),
            _ => None,
        })
}

fn service_capability_id(capability: &ServiceCapability) -> &str {
    match capability {
        ServiceCapability::SkillTraining(capability) => &capability.id,
        ServiceCapability::SkillCritique(capability) => &capability.id,
        ServiceCapability::SpellTeaching(capability) => &capability.id,
        ServiceCapability::ClassPromotion(capability) => &capability.id,
        ServiceCapability::ServiceTransaction(capability) => &capability.id,
        ServiceCapability::Merchant(capability) => &capability.id,
        ServiceCapability::ItemService(capability) => &capability.id,
        ServiceCapability::Restoration(capability) => &capability.id,
        ServiceCapability::Bank(capability) => &capability.id,
        ServiceCapability::Locker(capability) => &capability.id,
    }
}

impl GameCatalog {
    pub fn rules(&self) -> &WorldRules {
        &self.rules
    }

    pub fn item(&self, id: &str) -> Option<&CatalogItem> {
        self.item_catalog.get(id)
    }

    pub fn terrain(&self, id: &str) -> Option<&TerrainState> {
        self.terrains.get(id)
    }

    pub fn spell(&self, id: &str) -> Option<&SpellDef> {
        self.spells.get(id)
    }

    pub fn quest(&self, id: &str) -> Option<&QuestDefinition> {
        self.quests.get(&QuestId::new(id))
    }

    pub fn summon_template(&self, id: &str) -> Option<&SummonTemplate> {
        self.summon_templates.get(id)
    }

    pub fn service_definitions(&self) -> &[ServiceDefinition] {
        &self.service_definitions
    }
}

impl WorldTemplate {
    pub fn visual_manifest_digest(&self) -> &str {
        &self.visual_manifest_digest
    }

    pub fn realms(&self) -> &std::collections::HashMap<String, RealmState> {
        &self.realms
    }

    pub fn arrivals(&self) -> &std::collections::HashMap<String, WorldPosition> {
        &self.arrivals
    }

    pub fn navigation(&self) -> &std::collections::HashMap<WorldPosition, Vec<NavigationDef>> {
        &self.navigation
    }
}
