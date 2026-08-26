//! Typed restoration-service planning with delegated domain mutation.

use crate::events::{Event, TransactionRewardReceiptV1};
use crate::model::{
    ActorKind, CorpseId, ResourceKind, RestorationOutcome, RestorationStatusKind,
    ResurrectionMethod, ResurrectionRequest,
};
use crate::view::ActionBlockedReasonV1;

use super::transactions::{
    PlannedReward, TransactionPlan, TransactionPlanError, TransactionSource,
};
use super::{Engine, StepError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RestorationRewardPlan {
    RestoreResource {
        target_actor_index: usize,
        resource: ResourceKind,
    },
    CureStatus {
        target_actor_index: usize,
        status: RestorationStatusKind,
        operation_id: String,
    },
    PriestResurrection {
        request: ResurrectionRequest,
        target_actor_id: crate::model::ActorId,
        corpse_id: CorpseId,
    },
}

pub(super) struct RestorationServiceRequest {
    pub(super) service_id: String,
    pub(super) capability_id: String,
    pub(super) operation_id: String,
    pub(super) item_instance_id: Option<String>,
    pub(super) corpse_id: Option<CorpseId>,
}

impl Engine {
    pub(super) fn restoration_transaction_plan(
        &self,
        actor_index: usize,
        service_id: &str,
        capability_id: &str,
        operation_id: &str,
        selected_item_instance_id: Option<&str>,
        corpse_id: Option<&CorpseId>,
    ) -> Result<TransactionPlan, TransactionPlanError> {
        let actor = self.world.actors.get(actor_index).ok_or_else(|| {
            TransactionPlanError::new(ActionBlockedReasonV1::NoSuchTarget, "unknown actor")
        })?;
        if !actor.is_alive() {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ActorNotLiving,
                "restoration payer must be living",
            ));
        }
        let service = self.service_by_id(service_id).ok_or_else(|| {
            TransactionPlanError::new(
                ActionBlockedReasonV1::NoService,
                format!("service {service_id:?} was not found"),
            )
        })?;
        if service.position().level != actor.location.level
            || service.position().position != actor.location.position
        {
            return Err(TransactionPlanError::new(
                ActionBlockedReasonV1::ServiceNotHere,
                format!("service {service_id:?} is not at the actor coordinate"),
            ));
        }
        let capability = self
            .restoration_capability(service, capability_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::NoService,
                    format!(
                        "service {service_id:?} has no restoration capability {capability_id:?}"
                    ),
                )
            })?;
        let operation = capability
            .operations
            .iter()
            .find(|operation| operation.transaction.id == operation_id)
            .ok_or_else(|| {
                TransactionPlanError::new(
                    ActionBlockedReasonV1::UnsupportedRestoration,
                    format!("restoration operation {operation_id:?} was not found"),
                )
            })?;

        let reward = match &operation.outcome {
            RestorationOutcome::RestoreResource { resource } => {
                if corpse_id.is_some() {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::UnexpectedTransactionInput,
                        "resource restoration does not accept a corpse selection",
                    ));
                }
                let (current, maximum) = match resource {
                    ResourceKind::Hp => (actor.hp, actor.max_hp()),
                    ResourceKind::Mp => {
                        let maximum = actor
                            .character
                            .as_ref()
                            .map(|character| character.resources.max_mp)
                            .ok_or_else(|| {
                                TransactionPlanError::new(
                                    ActionBlockedReasonV1::UnsupportedRestoration,
                                    "MP restoration requires character resources",
                                )
                            })?;
                        (actor.mp, maximum)
                    }
                    ResourceKind::Stamina => (actor.stamina, actor.max_stamina()),
                };
                if current >= maximum {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::NoRestorationNeeded,
                        format!("{} is already full", format!("{resource:?}").to_lowercase()),
                    ));
                }
                RestorationRewardPlan::RestoreResource {
                    target_actor_index: actor_index,
                    resource: *resource,
                }
            }
            RestorationOutcome::CureStatus { status } => {
                if corpse_id.is_some() {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::UnexpectedTransactionInput,
                        "status cure does not accept a corpse selection",
                    ));
                }
                if !self.actor_has_effect_matching_tag(actor_index, status.effect_tag()) {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::NoRestorationNeeded,
                        format!("actor has no {} effect", status.label()),
                    ));
                }
                RestorationRewardPlan::CureStatus {
                    target_actor_index: actor_index,
                    status: *status,
                    operation_id: operation_id.to_string(),
                }
            }
            RestorationOutcome::PriestResurrection => {
                if selected_item_instance_id.is_some() {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::UnexpectedTransactionInput,
                        "Priest resurrection does not accept an item selection",
                    ));
                }
                let corpse_id = corpse_id.ok_or_else(|| {
                    TransactionPlanError::new(
                        ActionBlockedReasonV1::NoSuchCorpse,
                        "Priest resurrection requires an exact corpse selection",
                    )
                })?;
                let corpse = self.world.corpses.get(corpse_id).ok_or_else(|| {
                    TransactionPlanError::new(
                        ActionBlockedReasonV1::NoSuchCorpse,
                        "selected resurrection corpse does not exist",
                    )
                })?;
                if corpse.location.level != service.position().level
                    || corpse.location.position != service.position().position
                {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::CorpseNotHere,
                        "selected resurrection corpse is not at the Priest service",
                    ));
                }
                if corpse.origin_kind != ActorKind::Player {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::UnsupportedRestoration,
                        "Priest resurrection requires a player corpse",
                    ));
                }
                let target_index = self
                    .world
                    .actors
                    .iter()
                    .position(|candidate| candidate.id == corpse.origin_actor_id)
                    .ok_or_else(|| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::NoSuchTarget,
                            "resurrection corpse origin actor is missing",
                        )
                    })?;
                let target = &self.world.actors[target_index];
                let max_hp = target.max_hp();
                let max_stamina = target.max_stamina();
                if max_hp <= 1 || max_stamina <= 0 {
                    return Err(TransactionPlanError::new(
                        ActionBlockedReasonV1::UnsupportedRestoration,
                        "Priest provisional resource rule cannot satisfy resurrection bounds",
                    ));
                }
                let request = ResurrectionRequest {
                    actor_id: target.id.clone(),
                    corpse_id: Some(corpse_id.clone()),
                    method: ResurrectionMethod::Priest,
                    destination: service.position().clone(),
                    current_hp: max_hp - 1,
                    current_stamina: max_stamina - 1,
                };
                self.validate_resurrection_request(&request)
                    .map_err(|error| {
                        TransactionPlanError::new(
                            ActionBlockedReasonV1::UnsupportedRestoration,
                            error.message(),
                        )
                    })?;
                RestorationRewardPlan::PriestResurrection {
                    request,
                    target_actor_id: target.id.clone(),
                    corpse_id: corpse_id.clone(),
                }
            }
        };

        let source = TransactionSource::RestorationService {
            service_id: service_id.to_string(),
            capability_id: capability_id.to_string(),
            operation_id: operation_id.to_string(),
            corpse_id: corpse_id.cloned(),
        };
        self.plan_transaction(
            actor_index,
            source,
            &operation.transaction,
            selected_item_instance_id,
            vec![PlannedReward::Restoration(reward)],
        )
    }

    pub(super) fn apply_restoration_reward(
        &mut self,
        plan: RestorationRewardPlan,
    ) -> Result<(TransactionRewardReceiptV1, Vec<Event>), StepError> {
        match plan {
            RestorationRewardPlan::RestoreResource {
                target_actor_index,
                resource,
            } => {
                let actor_id = self.world.actors[target_actor_index].id.clone();
                let actor = self.world.actors[target_actor_index].name.clone();
                let maximum = match resource {
                    ResourceKind::Hp => self.world.actors[target_actor_index].max_hp(),
                    ResourceKind::Mp => {
                        self.world.actors[target_actor_index]
                            .character
                            .as_ref()
                            .ok_or_else(|| {
                                StepError::new("MP restoration target has no character")
                            })?
                            .resources
                            .max_mp
                    }
                    ResourceKind::Stamina => self.world.actors[target_actor_index].max_stamina(),
                };
                let delta = match resource {
                    ResourceKind::Hp => self.set_hp(target_actor_index, maximum)?,
                    ResourceKind::Mp => self.set_mp(target_actor_index, maximum)?,
                    ResourceKind::Stamina => self.set_stamina(target_actor_index, maximum)?,
                };
                if delta.actual <= 0 {
                    return Err(StepError::new(
                        "captured restoration no longer increases the resource",
                    ));
                }
                let event = Event::ResourceRestored {
                    actor_id: actor_id.clone(),
                    actor,
                    resource,
                    before: delta.before,
                    after: delta.current,
                    maximum: delta.maximum,
                };
                Ok((
                    TransactionRewardReceiptV1::ResourceRestored {
                        target_actor_id: actor_id,
                        resource,
                        before: delta.before,
                        after: delta.current,
                        maximum: delta.maximum,
                    },
                    vec![event],
                ))
            }
            RestorationRewardPlan::CureStatus {
                target_actor_index,
                status,
                operation_id,
            } => {
                let target_actor_id = self.world.actors[target_actor_index].id.clone();
                let mut events = Vec::new();
                let removed = self.remove_active_effects_matching_tag_from_actor(
                    target_actor_index,
                    status.effect_tag(),
                    &format!("restoration_service:{operation_id}"),
                    &mut events,
                );
                if removed == 0 {
                    return Err(StepError::new(
                        "captured restoration status is no longer present",
                    ));
                }
                let removed_count = u32::try_from(removed)
                    .map_err(|_| StepError::new("removed effect count exceeds u32"))?;
                Ok((
                    TransactionRewardReceiptV1::StatusCured {
                        target_actor_id,
                        status,
                        removed_count,
                    },
                    events,
                ))
            }
            RestorationRewardPlan::PriestResurrection {
                request,
                target_actor_id,
                corpse_id,
            } => {
                let current_hp = request.current_hp;
                let current_stamina = request.current_stamina;
                let mut events = self.apply_resurrection_request(request)?;
                self.schedule_resurrected_actor(&target_actor_id, &mut events)?;
                Ok((
                    TransactionRewardReceiptV1::PriestResurrection {
                        target_actor_id,
                        corpse_id,
                        method: ResurrectionMethod::Priest,
                        current_hp,
                        current_stamina,
                    },
                    events,
                ))
            }
        }
    }

    pub(super) fn apply_player_restoration_service(
        &mut self,
        actor_index: usize,
        request: RestorationServiceRequest,
        events: &mut Vec<Event>,
    ) -> Result<(), StepError> {
        let plan = self.restoration_transaction_plan(
            actor_index,
            &request.service_id,
            &request.capability_id,
            &request.operation_id,
            request.item_instance_id.as_deref(),
            request.corpse_id.as_ref(),
        )?;
        let actor_id = plan.actor_id.clone();
        let actor_name = plan.actor_name.clone();
        let mut receipt = self.commit_transaction(actor_index, plan)?;
        events.append(&mut receipt.delegated_events);
        events.push(receipt.committed_event(actor_id, actor_name));
        Ok(())
    }
}
