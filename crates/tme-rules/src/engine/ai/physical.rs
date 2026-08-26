use crate::events::{AutomaticActorDecisionV1, Event};
use crate::model::PhysicalAttackMode;

use super::super::physical_attacks::PhysicalAttackAuthority;
use super::super::{Engine, StepError};

impl Engine {
    pub(super) fn try_automatic_physical_action(
        &mut self,
        actor_index: usize,
        target_index: usize,
        events: &mut Vec<Event>,
    ) -> Result<bool, StepError> {
        let modes = self.world.actors[actor_index]
            .ai
            .as_ref()
            .expect("automatic actor AI was checked")
            .physical_attack_modes
            .clone();
        for mode in modes {
            match self.automatic_physical_attack_plan(actor_index, target_index, mode) {
                Ok(_) => {
                    let target = &self.world.actors[target_index];
                    self.emit_automatic_decision(
                        actor_index,
                        AutomaticActorDecisionV1::PhysicalAttack {
                            target_id: target.id.clone(),
                            target: target.name.clone(),
                            mode,
                        },
                        events,
                    );
                    self.attack_if_ready(
                        actor_index,
                        target_index,
                        mode,
                        PhysicalAttackAuthority::Automatic,
                        events,
                    )?;
                    return Ok(true);
                }
                Err(error)
                    if mode == PhysicalAttackMode::Shoot
                        && error.message() == "bow is not nocked"
                        && self
                            .automatic_physical_attack_opportunity_plan(
                                actor_index,
                                target_index,
                                mode,
                            )
                            .is_ok() =>
                {
                    let selection = self.physical_weapon_selection(actor_index)?;
                    let item_instance_id = selection
                        .item_instance_id
                        .ok_or_else(|| StepError::new("automatic nock has no selected bow"))?;
                    let item_definition_id = selection
                        .item_definition_id
                        .ok_or_else(|| StepError::new("automatic nock has no bow definition"))?;
                    let item = self
                        .definition
                        .catalog
                        .item_catalog
                        .get(&item_definition_id)
                        .map(|item| item.name.clone())
                        .ok_or_else(|| {
                            StepError::new("automatic nock bow definition is missing")
                        })?;
                    self.emit_automatic_decision(
                        actor_index,
                        AutomaticActorDecisionV1::Nock {
                            item_instance_id,
                            item_definition_id,
                            item,
                        },
                        events,
                    );
                    self.apply_actor_nock(actor_index, events)?;
                    return Ok(true);
                }
                Err(_) => {}
            }
        }
        Ok(false)
    }
}
