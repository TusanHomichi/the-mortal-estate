use super::*;

impl Engine {
    pub(super) fn npc_views_for_actor(
        &self,
        actor_index: usize,
    ) -> Result<Vec<NpcViewV1>, StepError> {
        let actor = self
            .world
            .actors
            .get(actor_index)
            .ok_or_else(|| StepError::new("NPC discovery actor disappeared"))?;
        let mut npc_indices = self
            .world
            .actors
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                candidate.kind == ActorKind::Npc
                    && candidate.is_alive()
                    && candidate.location.level == actor.location.level
                    && candidate.location.position == actor.location.position
            })
            .map(|(index, candidate)| (candidate.id.clone(), index))
            .collect::<Vec<_>>();
        npc_indices.sort_by(|left, right| left.0.cmp(&right.0));

        let mut views = Vec::with_capacity(npc_indices.len());
        for (_, npc_index) in npc_indices {
            let npc_actor = &self.world.actors[npc_index];
            let npc = npc_actor
                .npc
                .as_ref()
                .ok_or_else(|| StepError::new("validated NPC has no NPC state"))?;
            let mut interactions = Vec::with_capacity(npc.interactions.len());
            for interaction in &npc.interactions {
                let carried_requirement =
                    interaction
                        .transaction
                        .requirements
                        .iter()
                        .find_map(|requirement| match requirement {
                            crate::model::TransactionRequirement::CarriedItem {
                                item_definition_id,
                                quantity,
                            } => Some((item_definition_id.as_str(), *quantity)),
                            _ => None,
                        });
                let mut selections = match carried_requirement {
                    Some((definition_id, quantity)) => self.world.actors[actor_index]
                        .carried
                        .items
                        .iter()
                        .filter_map(|(position, instance_id)| {
                            self.world
                                .item_instances
                                .get(instance_id)
                                .filter(|instance| {
                                    instance.definition_id == definition_id
                                        && instance.quantity >= quantity
                                })
                                .map(|_| (*position, instance_id.clone()))
                        })
                        .collect::<Vec<_>>(),
                    None => Vec::new(),
                };
                selections.sort();
                let selection_ids = if carried_requirement.is_some() {
                    if selections.is_empty() {
                        vec![None]
                    } else {
                        selections
                            .into_iter()
                            .map(|(_, instance_id)| Some(instance_id))
                            .collect()
                    }
                } else {
                    vec![None]
                };
                let mut actions = Vec::with_capacity(selection_ids.len());
                for selection in selection_ids {
                    let command = PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor.id.clone(),
                        intent: PlayerIntentPayloadV1::InteractWithNpc {
                            npc_actor_id: npc_actor.id.clone(),
                            interaction_id: interaction.transaction.id.clone(),
                            item_instance_id: selection.clone(),
                        },
                    };
                    let status = self.validate_actor_command(&command)?;
                    actions.push(ActionOptionV1 {
                        id: format!(
                            "npc:{}:{}:{}",
                            npc_actor.id,
                            interaction.transaction.id,
                            selection.as_deref().unwrap_or("none")
                        ),
                        label: format!("{}: {}", npc_actor.name, interaction.transaction.label),
                        enabled: status.accepted,
                        blocked_reason: status.blocked_reason,
                        command: Some(command),
                    });
                }
                interactions.push(NpcInteractionViewV1 {
                    interaction_id: interaction.transaction.id.clone(),
                    label: interaction.transaction.label.clone(),
                    requirements: interaction
                        .transaction
                        .requirements
                        .iter()
                        .map(TransactionRequirementViewV1::from)
                        .collect(),
                    costs: interaction
                        .transaction
                        .costs
                        .iter()
                        .map(TransactionCostViewV1::from)
                        .collect(),
                    rewards: interaction
                        .transaction
                        .rewards
                        .iter()
                        .map(TransactionRewardViewV1::from)
                        .collect(),
                    outcome: interaction.outcome.clone(),
                    actions,
                });
            }
            views.push(NpcViewV1 {
                actor_id: npc_actor.id.clone(),
                name: npc_actor.name.clone(),
                following_character_id: npc.following_character_id.clone(),
                interactions,
            });
        }
        Ok(views)
    }
}
