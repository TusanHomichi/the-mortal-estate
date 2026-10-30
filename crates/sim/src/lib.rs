//! The Mortal Estate's rules and the world they run: plain Rust with no
//! engine in it, deterministic, stepped at a fixed rate. The server runs it
//! with authority; a client runs its walking to predict its own movement.

pub mod body;
pub mod catalogue;
pub mod creature;
pub mod geom;
pub mod rng;
pub mod world;

pub use catalogue::{Catalogue, Id, Side};
pub use geom::{Dir, Vec2};
pub use world::{BodyId, CreatureId, Event, Input, Intent, PersonId, Refused, World};

/// A count of steps.
pub type Steps = u32;

/// How often the world steps, a second.
pub const STEPS_PER_SECOND: u32 = 20;

/// How long a step is, in milliseconds.
pub const STEP_MS: u32 = 1000 / STEPS_PER_SECOND;
