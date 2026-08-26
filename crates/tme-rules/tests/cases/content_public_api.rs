use tme_rules::content;
use tme_rules::{
    ActorAiDef, ActorAwarenessDef, ActorDeathDef, ActorSeedDef, BurdenRulesDef, CatalogProfileDef,
    CatalogProfileKey, CatalogRegistryKey, CatalogV6, CombatBlockRulesDef, CombatDamageRulesDef,
    CombatHitRulesDef, CombatPracticeRulesDef, CombatRulesDef, CombatTuningStatusDef,
    DamageLabelDef, Engine, GameDefinition, GroundItemSeedDef, ItemDef, ItemEconomyDef,
    ItemInstanceSeedDef, ItemInstanceState, ItemKnowledgeDef, ItemKnowledgeState, LevelDef,
    MerchantInventorySeedDef, RealmDef, ResearchBoundary, SelectedCatalog, ServiceDefinitionDef,
    ServiceInstanceSeedDef, SpellDef, StepError, TransactionCostDef, TransactionDef,
    TransactionRequirementDef, TransactionRewardDef, ValidatedWorldSeed, ValidationError, World,
    WorldSeedDef, WorldTemplateV3,
};

fn assert_type<T>() {}

#[test]
fn top_level_exports_name_the_four_contract_and_runtime_seams() {
    assert_type::<CatalogV6>();
    assert_type::<CatalogProfileDef>();
    assert_type::<CatalogProfileKey>();
    assert_type::<CatalogRegistryKey>();
    assert_type::<SelectedCatalog>();
    assert_type::<WorldTemplateV3>();
    assert_type::<WorldSeedDef>();
    assert_type::<GameDefinition>();
    assert_type::<ValidatedWorldSeed>();
    assert_type::<ActorSeedDef>();
    assert_type::<GroundItemSeedDef>();
    assert_type::<ItemInstanceSeedDef>();
    assert_type::<ServiceInstanceSeedDef>();
    assert_type::<MerchantInventorySeedDef>();
    assert_type::<ServiceDefinitionDef>();
    assert_type::<ValidationError>();
}

#[test]
fn shared_definition_exports_remain_public_without_scenario_ownership() {
    assert_type::<ActorAiDef>();
    assert_type::<ActorAwarenessDef>();
    assert_type::<ActorDeathDef>();
    assert_type::<BurdenRulesDef>();
    assert_type::<CombatRulesDef>();
    assert_type::<CombatHitRulesDef>();
    assert_type::<CombatBlockRulesDef>();
    assert_type::<CombatDamageRulesDef>();
    assert_type::<CombatPracticeRulesDef>();
    assert_type::<CombatTuningStatusDef>();
    assert_type::<DamageLabelDef>();
    assert_type::<ItemDef>();
    assert_type::<ItemEconomyDef>();
    assert_type::<ItemKnowledgeDef>();
    assert_type::<ResearchBoundary>();
    assert_type::<RealmDef>();
    assert_type::<LevelDef>();
    assert_type::<SpellDef>();
    assert_type::<TransactionDef>();
    assert_type::<TransactionRequirementDef>();
    assert_type::<TransactionCostDef>();
    assert_type::<TransactionRewardDef>();
}

#[test]
fn content_module_and_domain_modules_expose_current_owner_types() {
    assert_type::<content::CatalogV6>();
    assert_type::<content::WorldTemplateV3>();
    assert_type::<content::WorldSeedDef>();
    assert_type::<content::ActorSeedDef>();
    assert_type::<content::GroundItemSeedDef>();
    assert_type::<content::ItemInstanceSeedDef>();
    assert_type::<content::ServiceDefinitionDef>();
    assert_type::<content::ServiceInstanceSeedDef>();
    assert_type::<content::MerchantInventorySeedDef>();
    assert_type::<content::actors::ActorSeedDef>();
    assert_type::<content::items::GroundItemSeedDef>();
    assert_type::<content::items::ItemInstanceSeedDef>();
    assert_type::<content::services::ServiceDefinitionDef>();
    assert_type::<content::world_seed::WorldSeedDef>();
    assert_type::<content::world_template::WorldTemplateV3>();
}

#[test]
fn runtime_domain_module_is_public() {
    assert_type::<ItemInstanceState>();
    assert_type::<ItemKnowledgeState>();
    assert_type::<tme_rules::model::ActorAiState>();
    assert_type::<tme_rules::model::SocialProfile>();
    assert_type::<tme_rules::model::SocialAlignmentSource>();
    assert_type::<tme_rules::model::SocialBehavior>();
    assert_type::<tme_rules::model::Transaction>();
    assert_type::<tme_rules::model::transactions::Transaction>();
    assert_type::<tme_rules::model::items::ItemHolderId>();
    assert_type::<tme_rules::model::items::ItemLocation>();
}

#[test]
fn engine_construction_requires_a_validated_world_seed() {
    let constructor: fn(ValidatedWorldSeed, u64) -> Result<Engine, StepError> = Engine::new;
    let _ = constructor;
}

fn destructure_the_complete_mutable_world_shape(world: World) {
    let World {
        timing: _,
        actors: _,
        social_relations: _,
        groups: _,
        group_invitations: _,
        player_follow_targets: _,
        communication_preferences: _,
        character_presence: _,
        defeat_contributions: _,
        item_instances: _,
        service_instances: _,
        merchant_inventories: _,
        banks: _,
        locker_vaults: _,
        item_offers: _,
        quest_states: _,
        ground_items: _,
        corpses: _,
        ground_gold: _,
        next_corpse_sequence: _,
        next_gold_sequence: _,
        next_summon_sequence: _,
        next_group_sequence: _,
        next_group_invite_sequence: _,
        next_membership_epoch: _,
        tile_effects: _,
        item_enchantments: _,
        portal_transitions: _,
        concealed_transitions: _,
        hidden_transition_revealed: _,
        door_states: _,
        ecology_sites: _,
        next_player_kill_sequence: _,
        linked_player_kill_karma: _,
    } = world;
}

#[test]
fn mutable_world_shape_contains_only_runtime_state() {
    let shape_guard: fn(World) = destructure_the_complete_mutable_world_shape;
    let _ = shape_guard;
}
