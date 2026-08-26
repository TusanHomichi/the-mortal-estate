use super::*;

/// Map a class to the spell lanes it can access.
pub(in crate::engine) fn class_spell_lanes(class_id: &str) -> Vec<&'static str> {
    match class_id {
        "knight" => vec!["knight_magic"],
        "thaumaturge" => vec!["thaumaturge_magic"],
        "thief" => vec!["thief_magic"],
        "wizard" => vec!["wizard_magic"],
        _ => vec![],
    }
}

impl Engine {
    pub(super) fn spell_action_descriptors(
        &self,
        player_index: usize,
    ) -> Result<Vec<SpellActionV1>, StepError> {
        let actor = &self.world.actors[player_index];
        let Some(character) = actor.character.as_ref() else {
            return Ok(Vec::new());
        };
        let mut rows = Vec::new();
        for known in &character.known_spells {
            let Some(spell) = self.definition.catalog.spells.get(&known.spell_id) else {
                continue;
            };
            let Some(casting) = spell.casting.as_ref() else {
                continue;
            };
            let warm_result = self.validate_warm_spell_command(player_index, &spell.id);
            let mut warm = match warm_result {
                Ok(_) => SpellActionStateV1 {
                    enabled: true,
                    blocked_reason: None,
                    requires_target_selection: false,
                    command: Some(PlayerCommandV1 {
                        contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                        actor_id: actor.id.clone(),
                        intent: PlayerIntentPayloadV1::WarmSpell {
                            spell_id: spell.id.clone(),
                        },
                    }),
                },
                Err(reason) => SpellActionStateV1 {
                    enabled: false,
                    blocked_reason: Some(reason),
                    requires_target_selection: false,
                    command: None,
                },
            };

            let target_kind = spell.target.as_ref().map(|target| target.kind);
            let requires_target_selection = matches!(
                casting.cast_class,
                crate::model::SpellCastClass::Character
                    | crate::model::SpellCastClass::Path
                    | crate::model::SpellCastClass::PathOrCharacter
            ) || matches!(
                target_kind,
                Some(
                    SpellTargetKind::Actor
                        | SpellTargetKind::Area
                        | SpellTargetKind::Coordinate
                        | SpellTargetKind::Direction
                        | SpellTargetKind::Door
                        | SpellTargetKind::Item
                )
            );
            let complete_target = if casting.cast_class == crate::model::SpellCastClass::SelfTarget
                || target_kind == Some(SpellTargetKind::SelfTarget)
            {
                Some(SpellTarget::SelfTarget)
            } else {
                None
            };
            let eligibility = match casting.method {
                crate::model::SpellCastingMethod::Direct => {
                    self.validate_direct_spell_eligibility(player_index, &spell.id)
                }
                crate::model::SpellCastingMethod::WarmThenCast => {
                    if actor
                        .warmed_spell
                        .as_ref()
                        .is_none_or(|warmed| warmed.spell_id != spell.id)
                    {
                        Err(ActionBlockedReasonV1::NoWarmedSpell)
                    } else {
                        self.validate_warmed_spell_eligibility(player_index)
                    }
                }
            };
            let mut cast = match eligibility {
                Err(reason) => SpellActionStateV1 {
                    enabled: false,
                    blocked_reason: Some(reason),
                    requires_target_selection: false,
                    command: None,
                },
                Ok(_) if requires_target_selection => SpellActionStateV1 {
                    enabled: true,
                    blocked_reason: None,
                    requires_target_selection: true,
                    command: None,
                },
                Ok(_) => {
                    let validation = match casting.method {
                        crate::model::SpellCastingMethod::Direct => self
                            .validate_direct_spell_command(
                                player_index,
                                &spell.id,
                                complete_target.as_ref(),
                            ),
                        crate::model::SpellCastingMethod::WarmThenCast => self
                            .validate_warmed_spell_command(player_index, complete_target.as_ref()),
                    };
                    match validation {
                        Ok(_) => SpellActionStateV1 {
                            enabled: true,
                            blocked_reason: None,
                            requires_target_selection: false,
                            command: Some(PlayerCommandV1 {
                                contract_version: crate::view::COMMAND_CONTRACT_VERSION,
                                actor_id: actor.id.clone(),
                                intent: match casting.method {
                                    crate::model::SpellCastingMethod::Direct => {
                                        PlayerIntentPayloadV1::CastSpell {
                                            spell_id: spell.id.clone(),
                                            target: complete_target.clone(),
                                            authorization:
                                                crate::model::HostilityAuthorization::Safe,
                                        }
                                    }
                                    crate::model::SpellCastingMethod::WarmThenCast => {
                                        PlayerIntentPayloadV1::CastWarmedSpell {
                                            target: complete_target.clone(),
                                            authorization:
                                                crate::model::HostilityAuthorization::Safe,
                                        }
                                    }
                                },
                            }),
                        },
                        Err(reason) => SpellActionStateV1 {
                            enabled: false,
                            blocked_reason: Some(reason),
                            requires_target_selection: false,
                            command: None,
                        },
                    }
                }
            };
            if self.suppressing_effect_for_actor(player_index).is_some() {
                for state in [&mut warm, &mut cast] {
                    state.enabled = false;
                    state.blocked_reason = Some(ActionBlockedReasonV1::SuppressedByStatus);
                    state.requires_target_selection = false;
                    state.command = None;
                }
            }
            rows.push(SpellActionV1 {
                spell_id: spell.id.clone(),
                spell_name: spell.name.clone(),
                casting_method: casting.method,
                cast_class: casting.cast_class,
                target_kind,
                mp_cost: spell.mp_cost,
                stamina_cost: spell.stamina_cost,
                social: SpellSocialViewV1 {
                    hostile_act: spell.social.hostile_act,
                    town_law: match spell.social.town_law {
                        crate::content::TownLawClassificationDef::Permitted => {
                            SpellTownLawViewV1::Permitted
                        }
                        crate::content::TownLawClassificationDef::TerrainAlignmentViolation => {
                            SpellTownLawViewV1::TerrainAlignmentViolation
                        }
                    },
                },
                warm,
                cast,
            });
        }
        rows.sort_by(|left, right| left.spell_id.cmp(&right.spell_id));
        Ok(rows)
    }
}
