//! Locate and scry hints, and the observation they may read.

use crate::content::{SpellLocateDef, SpellScryDef};
use crate::model::WorldPosition;

use crate::engine::Engine;

impl Engine {
    pub(super) fn resolve_locate_hint(
        &self,
        player_index: usize,
        locate: &SpellLocateDef,
    ) -> (
        Option<crate::model::WorldSite>,
        Option<WorldPosition>,
        String,
    ) {
        match locate.subject.as_str() {
            "actor" => {
                let Some(actor) = self
                    .world
                    .actors
                    .iter()
                    .find(|actor| actor.is_alive() && actor.id.as_str() == locate.id)
                else {
                    return (
                        None,
                        None,
                        format!("actor {} is hidden or not found", locate.id),
                    );
                };
                if !self.actor_observed_by_player(player_index, actor) {
                    return (
                        None,
                        None,
                        format!("actor {} is hidden or unobserved", locate.id),
                    );
                }
                (
                    Some(actor.location.site()),
                    Some(actor.location.clone()),
                    format!(
                        "actor {} located in {} at {},{}",
                        locate.id,
                        actor.location.level,
                        actor.location.position.x,
                        actor.location.position.y
                    ),
                )
            }
            "item" => {
                if let Some(item) = self.ground_items().iter().find(|item| {
                    self.item_instance(&item.item_instance_id)
                        .is_ok_and(|instance| instance.definition_id == locate.id)
                }) {
                    if !self.world_position_observed_by_player(player_index, &item.location.clone())
                    {
                        return (
                            None,
                            None,
                            format!("item {} is hidden or unobserved", locate.id),
                        );
                    }
                    return (
                        Some(item.location.site()),
                        Some(item.location.clone()),
                        format!(
                            "item {} located in {} at {},{}",
                            locate.id,
                            item.location.level,
                            item.location.position.x,
                            item.location.position.y
                        ),
                    );
                }
                if let Some(actor) = self
                    .world
                    .actors
                    .iter()
                    .enumerate()
                    .find(|(actor_index, actor)| {
                        actor.is_alive()
                            && self.carried_item_ids(*actor_index).is_ok_and(|carried| {
                                carried.iter().any(|instance_id| {
                                    self.item_instance(instance_id)
                                        .is_ok_and(|instance| instance.definition_id == locate.id)
                                })
                            })
                    })
                    .map(|(_, actor)| actor)
                {
                    if !self.actor_observed_by_player(player_index, actor) {
                        return (
                            None,
                            None,
                            format!("item {} is hidden or unobserved", locate.id),
                        );
                    }
                    return (
                        Some(actor.location.site()),
                        Some(actor.location.clone()),
                        format!(
                            "item {} located in {} at {},{}",
                            locate.id,
                            actor.location.level,
                            actor.location.position.x,
                            actor.location.position.y
                        ),
                    );
                }
                (
                    None,
                    None,
                    format!("item {} is hidden or not found", locate.id),
                )
            }
            "level" => {
                let player_realm = &self.world.actors[player_index].location.realm;
                let site = crate::model::WorldSite::new(player_realm, &locate.id);
                if self
                    .definition
                    .world_template
                    .realms
                    .get(&site.realm)
                    .and_then(|realm| realm.levels.get(&site.level))
                    .is_none()
                {
                    return (
                        None,
                        None,
                        format!("level {} is hidden or not found", locate.id),
                    );
                }
                if !self.level_known_for_locate(player_index, &site) {
                    return (
                        None,
                        None,
                        format!("level {} is hidden or unobserved", locate.id),
                    );
                }
                (Some(site), None, format!("level {} located", locate.id))
            }
            _ => (None, None, "locate subject is unsupported".to_string()),
        }
    }

    pub(super) fn resolve_scry_hint(
        &self,
        player_index: usize,
        spell_id: &str,
        scry: &SpellScryDef,
    ) -> (
        Option<crate::model::WorldSite>,
        Option<WorldPosition>,
        String,
    ) {
        match scry.scope.as_str() {
            "level" => {
                if self.level_known_for_locate(player_index, &scry.site) {
                    (
                        Some(scry.site.clone()),
                        None,
                        format!("scry {spell_id} located level {}", scry.site.label()),
                    )
                } else {
                    (
                        None,
                        None,
                        format!("scry {spell_id} is hidden or unobserved"),
                    )
                }
            }
            "coordinate" => {
                let Some(position) = scry.position else {
                    return (
                        None,
                        None,
                        format!("scry {spell_id} is hidden or unobserved"),
                    );
                };
                let location = WorldPosition::new(&scry.site.realm, &scry.site.level, position);
                if self.world_position_observed_by_player(player_index, &location) {
                    (
                        Some(scry.site.clone()),
                        Some(location),
                        format!(
                            "scry {spell_id} located in {} at {},{}",
                            scry.site.level, position.x, position.y
                        ),
                    )
                } else {
                    (
                        None,
                        None,
                        format!("scry {spell_id} is hidden or unobserved"),
                    )
                }
            }
            _ => (
                None,
                None,
                format!("scry {spell_id} is hidden or unobserved"),
            ),
        }
    }

    fn actor_observed_by_player(
        &self,
        player_index: usize,
        actor: &crate::model::ActorState,
    ) -> bool {
        self.world_position_observed_by_player(player_index, &actor.location.clone())
    }

    fn world_position_observed_by_player(
        &self,
        player_index: usize,
        position: &WorldPosition,
    ) -> bool {
        let player = &self.world.actors[player_index];
        if !player.location.same_site(position) {
            return false;
        }
        self.visible_tiles_for_actor_id(&player.id)
            .is_ok_and(|visible| visible.contains(position))
    }

    fn level_known_for_locate(&self, player_index: usize, site: &crate::model::WorldSite) -> bool {
        let player = &self.world.actors[player_index];
        player.location.site() == *site
            || self
                .automatic_navigation_edges_from(&player.location.site())
                .iter()
                .any(|(_, transition)| transition.target.site() == *site)
    }
}
