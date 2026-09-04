//! Deadline selection is derived from the owning states, never a second timer ledger.
use super::Engine;
use crate::model::{ActorKind, GROUP_DISCONNECT_GRACE_UNITS, LogicalTime, WarmedSpellStatus};

impl Engine {
    pub fn next_deadline(&self) -> Option<LogicalTime> {
        let now = self.world.timing.now;
        if self.world.actors.iter().any(|actor| {
            actor.is_alive() && actor.kind != ActorKind::Player && actor.timing.ready_at <= now
        }) {
            return Some(now);
        }
        let mut next: Option<LogicalTime> = None;
        let mut consider = |at: LogicalTime| {
            if at > now {
                next = Some(next.map_or(at, |old| old.min(at)));
            }
        };
        let recovery = self
            .definition
            .catalog
            .rules
            .resources
            .recovery_interval_units;
        for actor in &self.world.actors {
            // A defeated summon still owns its expiry and must be cleaned up.
            if let Some(summon) = &actor.summoned {
                consider(summon.last_ticked_at.saturating_add_rounds(1));
            }
            if !actor.is_alive() {
                continue;
            }
            consider(actor.timing.ready_at);
            if actor.kind == ActorKind::Player && actor.character.is_some() {
                consider(
                    actor
                        .resource_activity
                        .last_recovered_at
                        .saturating_add_rounds(recovery),
                );
            }
            for effect in &actor.active_effects {
                consider(effect.last_ticked_at.saturating_add_rounds(
                    if effect.start_delay_rounds > 0 {
                        1
                    } else {
                        effect.tick_interval_rounds
                    },
                ));
            }
            if let Some(effect) = &actor.balm_effect {
                consider(effect.last_tick_at.saturating_add_rounds(1));
            }
            if let Some(spell) = &actor.warmed_spell
                && spell.status == WarmedSpellStatus::Warming
            {
                consider(spell.ready_at);
            }
        }
        for effect in &self.world.tile_effects {
            consider(
                effect
                    .last_ticked_at
                    .saturating_add_rounds(effect.tick_interval_rounds),
            );
        }
        for effect in &self.world.item_enchantments {
            consider(effect.last_ticked_at.saturating_add_rounds(1));
        }
        for effect in &self.world.portal_transitions {
            consider(effect.last_ticked_at.saturating_add_rounds(1));
        }
        for effect in &self.world.concealed_transitions {
            consider(effect.last_ticked_at.saturating_add_rounds(1));
        }
        for site in self.world.ecology_sites.values() {
            if let Some(at) = site.full_clear_due_at {
                consider(at);
            }
            for slot in site.member_slots.values() {
                if let Some(at) = slot.due_at {
                    consider(at);
                }
            }
        }
        for invitation in self.world.group_invitations.values() {
            consider(invitation.expires_at);
        }
        for presence in self.world.character_presence.values() {
            if let Some(at) = presence.absent_since {
                consider(at.saturating_add_rounds(GROUP_DISCONNECT_GRACE_UNITS as u32));
            }
        }
        next
    }
}
