#![recursion_limit = "256"]

#[path = "spell_effect_support/mod.rs"]
mod spell_effect_support;
#[path = "spell_support/mod.rs"]
mod spell_support;
#[path = "support/mod.rs"]
mod support;

#[path = "cases/active_effects.rs"]
mod active_effects;
#[path = "cases/item_capability.rs"]
mod item_capability;
#[path = "cases/magic_mp_recovery.rs"]
mod magic_mp_recovery;
#[path = "cases/magic_rewards.rs"]
mod magic_rewards;
#[path = "cases/spell_actor_effects.rs"]
mod spell_actor_effects;
#[path = "cases/spell_catalog.rs"]
mod spell_catalog;
#[path = "cases/spell_doors.rs"]
mod spell_doors;
#[path = "cases/spell_effect_families.rs"]
mod spell_effect_families;
#[path = "cases/spell_items.rs"]
mod spell_items;
#[path = "cases/spell_learning.rs"]
mod spell_learning;
#[path = "cases/spell_lifecycle.rs"]
mod spell_lifecycle;
#[path = "cases/spell_summons.rs"]
mod spell_summons;
#[path = "cases/spell_targeting.rs"]
mod spell_targeting;
#[path = "cases/spell_terrain.rs"]
mod spell_terrain;
#[path = "cases/spell_world_utility.rs"]
mod spell_world_utility;
