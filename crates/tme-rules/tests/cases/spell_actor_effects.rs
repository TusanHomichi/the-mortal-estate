use crate::spell_effect_support::*;
use crate::spell_support::*;
use tme_rules::*;

#[path = "spell_actor_effects/healing_spell_restores_hp_to_max_and_spends_resources_without_stub.rs"]
mod healing_spell_restores_hp_to_max_and_spends_resources_without_stub;

#[path = "spell_actor_effects/poison_spell_applies_delayed_tick_damage_and_syncs_character_hp.rs"]
mod poison_spell_applies_delayed_tick_damage_and_syncs_character_hp;

#[path = "spell_actor_effects/lethal_fire_spell_passes_fire_credit_and_suppresses_corpse_creation.rs"]
mod lethal_fire_spell_passes_fire_credit_and_suppresses_corpse_creation;
