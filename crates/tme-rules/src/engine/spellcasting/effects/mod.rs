//! Spell effect execution, split by responsibility.
//!
//! `execute` is the single dispatch from a committed cast to an effect
//! family. `contact` decides who a hostile cast reaches; each family module
//! owns its own resolution, and `direct_damage` owns spatial target
//! selection for damage. This module used to be one file of nearly two
//! thousand lines, which is past the point where a reader can hold it.

mod contact;
mod direct_damage;
mod effect_support;
mod execute;
mod healing;
mod locate;
mod portal_utility;
mod protected_families;
mod recovery;
mod terrain_families;
