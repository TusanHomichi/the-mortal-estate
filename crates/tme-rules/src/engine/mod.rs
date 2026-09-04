//! The simulation engine.
//!
//! The engine is split into three concerns:
//!
//! * [`definition`] — the immutable content definition a run is bound to.
//! * this module — the mutable [`Engine`] itself and its core accessors.
//! * [`player_intent`] — the dispatch from a player intent to the subsystem
//!   that owns its rule.
//!
//! Everything below is a rule subsystem. Each owns one area of the step loop
//! and reaches the engine state through `self`.

use std::sync::Arc;

use crate::events::{ActorSummary, Event};
use crate::model::{SpellCatalogEntry, World};
use crate::rng::DeterministicRng;
use crate::view::ActorLifeStateViewV1;

mod ai;
mod armor;
mod character_transfer;
mod combat;

mod checkpoint;
pub use checkpoint::{
    CheckpointError, ContentIdentityV1, FACET_CHECKPOINT_SCHEMA_VERSION, FacetCheckpointV5,
};
mod damage;
mod deadlines;
mod death;
mod defeat_rewards;
mod ecology;
mod effects;
mod groups;
mod inspect;
mod inventory;
mod items;
mod merchants;
mod movement;
mod navigation;
mod npc_interactions;
mod npcs;
mod path_preview;
mod physical_attacks;
mod professions;
mod progression;
mod promotion;
mod query;
mod quests;
mod resources;
mod restoration;
mod rewards;
mod services;
mod setup;
pub use setup::ValidatedWorldSeed;
pub mod skills;
mod social;
mod spell_learning;
mod spellcasting;
mod storage;
mod summons;
mod tile_effects;
mod timing;
pub use timing::RulesOutcomeV1;
mod training;
mod transactions;
mod view;
mod visibility;
mod weapons;

pub use visibility::PLAYER_OBSERVATION_RADIUS;

mod action_context;

mod definition;
pub use definition::{CatalogItem, GameCatalog, GameDefinition, WorldTemplate};
mod player_intent;
mod step_error;
pub use step_error::StepError;

#[derive(Clone)]
pub struct Engine {
    definition: Arc<GameDefinition>,
    world: World,
    rng: DeterministicRng,
    initial_events: Vec<Event>,
    pending_durable_effects: Vec<crate::model::DurableGameplayEffectV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommittedActivity {
    Active,
    Inactive,
}

impl Engine {
    pub fn definition(&self) -> &Arc<GameDefinition> {
        &self.definition
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn initial_events(&self) -> Vec<Event> {
        self.initial_events.clone()
    }

    pub fn spell_catalog_entries(&self) -> impl ExactSizeIterator<Item = &SpellCatalogEntry> {
        self.definition.catalog.spell_catalog.values()
    }

    pub fn spell_catalog_entry(&self, spell_id: &str) -> Option<&SpellCatalogEntry> {
        self.definition.catalog.spell_catalog.get(spell_id)
    }

    pub fn final_events(&self) -> Vec<Event> {
        vec![Event::FinalState {
            actors: self
                .world
                .actors
                .iter()
                .map(|actor| ActorSummary {
                    id: actor.id.clone(),
                    name: actor.name.clone(),
                    location: actor.location.clone(),
                    hp: actor.hp,
                    life_state: ActorLifeStateViewV1::from(&actor.life_state),
                    character_identity: actor.character.as_ref().map(|c| c.identity.clone()),
                })
                .collect(),
        }]
    }
}
