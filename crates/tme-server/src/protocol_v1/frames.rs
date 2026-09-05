use super::*;

pub(super) fn observer_intent(
    value: &rules::PlayerIntentPayloadV1,
) -> Result<wire::Intent, wire::ProtocolError> {
    Ok(match value {
        rules::PlayerIntentPayloadV1::MovePath { path } => wire::Intent::MovePath {
            path: path.iter().copied().map(direction).collect(),
        },
        rules::PlayerIntentPayloadV1::Traverse { kind } => wire::Intent::Traverse {
            traversal: explicit_traversal(*kind),
        },
        rules::PlayerIntentPayloadV1::Open { direction: value } => wire::Intent::Open {
            direction: direction(*value),
        },
        rules::PlayerIntentPayloadV1::Close { direction: value } => wire::Intent::Close {
            direction: direction(*value),
        },
        rules::PlayerIntentPayloadV1::Inspect => wire::Intent::Inspect,
        rules::PlayerIntentPayloadV1::Hide => wire::Intent::Hide,
        rules::PlayerIntentPayloadV1::ShowSack => wire::Intent::ShowSack,
        rules::PlayerIntentPayloadV1::Wait => wire::Intent::Wait,
        rules::PlayerIntentPayloadV1::Rest => wire::Intent::Rest,
        rules::PlayerIntentPayloadV1::PhysicalAttack {
            mode,
            target_actor_id,
            authorization: value,
        } => wire::Intent::PhysicalAttack {
            mode: physical_mode(*mode),
            target_actor_id: actor_id(target_actor_id)?,
            authorization: authorization(*value),
        },
        rules::PlayerIntentPayloadV1::Nock => wire::Intent::Nock,
        rules::PlayerIntentPayloadV1::UnloadBow => wire::Intent::UnloadBow,
        rules::PlayerIntentPayloadV1::WarmSpell { spell_id } => wire::Intent::WarmSpell {
            spell_id: label(spell_id)?,
        },
        rules::PlayerIntentPayloadV1::CastSpell {
            spell_id,
            target,
            authorization: value,
        } => wire::Intent::CastSpell {
            spell_id: label(spell_id)?,
            target: target.as_ref().map(spell_target).transpose()?,
            authorization: authorization(*value),
        },
        rules::PlayerIntentPayloadV1::CastWarmedSpell {
            target,
            authorization: value,
        } => wire::Intent::CastWarmedSpell {
            target: target.as_ref().map(spell_target).transpose()?,
            authorization: authorization(*value),
        },
        rules::PlayerIntentPayloadV1::FizzleWarmedSpell => wire::Intent::FizzleWarmedSpell,
        rules::PlayerIntentPayloadV1::SearchCorpse { corpse_id } => wire::Intent::SearchCorpse {
            corpse_id: wire::CorpseId::new(corpse_id.as_str())?,
        },
        rules::PlayerIntentPayloadV1::MoveItem {
            item_instance_id,
            destination,
        } => wire::Intent::MoveItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            destination: match destination {
                rules::ItemMoveDestination::GroundHere => wire::ItemMoveDestination::GroundHere,
                rules::ItemMoveDestination::Carried { position } => {
                    wire::ItemMoveDestination::Carried {
                        position: carried_position(*position),
                    }
                }
            },
        },
        rules::PlayerIntentPayloadV1::MoveGold {
            source,
            destination,
            quantity,
        } => wire::Intent::MoveGold {
            source: match source {
                rules::GoldMoveSource::Carried { position } => wire::GoldMoveSource::Carried {
                    position: gold_position(*position),
                },
                rules::GoldMoveSource::Ground { gold_pile_id } => wire::GoldMoveSource::Ground {
                    gold_pile_id: label(gold_pile_id.as_str())?,
                },
            },
            destination: match destination {
                rules::GoldMoveDestination::Carried { position } => {
                    wire::GoldMoveDestination::Carried {
                        position: gold_position(*position),
                    }
                }
                rules::GoldMoveDestination::GroundHere => wire::GoldMoveDestination::GroundHere,
            },
            quantity: match quantity {
                rules::GoldMoveQuantity::All => wire::GoldMoveQuantity::All,
                rules::GoldMoveQuantity::Exact { amount } => wire::GoldMoveQuantity::Exact {
                    amount: wire::DecimalI64::new(*amount),
                },
            },
        },
        rules::PlayerIntentPayloadV1::DepositBankGold {
            service_id,
            capability_id,
            gold_pile_id,
        } => wire::Intent::DepositBankGold {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            gold_pile_id: label(gold_pile_id.as_str())?,
        },
        rules::PlayerIntentPayloadV1::WithdrawBankGold {
            service_id,
            capability_id,
            amount,
        } => wire::Intent::WithdrawBankGold {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            amount: wire::DecimalI64::new(*amount),
        },
        rules::PlayerIntentPayloadV1::DepositLockerItem {
            service_id,
            capability_id,
            item_instance_id,
        } => wire::Intent::DepositLockerItem {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::PlayerIntentPayloadV1::WithdrawLockerItem {
            service_id,
            capability_id,
            item_instance_id,
            destination,
        } => wire::Intent::WithdrawLockerItem {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            destination: carried_position(*destination),
        },
        rules::PlayerIntentPayloadV1::Drink { item_instance_id } => wire::Intent::DrinkItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::PlayerIntentPayloadV1::Train {
            service_id,
            offered_gold,
        } => wire::Intent::Train {
            service_id: label(service_id)?,
            offered_gold: wire::DecimalI64::new(*offered_gold),
        },
        rules::PlayerIntentPayloadV1::Critique {
            service_id,
            track_id,
        } => wire::Intent::Critique {
            service_id: label(service_id)?,
            track_id: label(track_id)?,
        },
        rules::PlayerIntentPayloadV1::PromoteClass { target_class_id } => {
            wire::Intent::PromoteClass {
                target_class_id: label(target_class_id)?,
            }
        }
        rules::PlayerIntentPayloadV1::LearnSpell { spell_id } => wire::Intent::LearnSpell {
            spell_id: label(spell_id)?,
        },
        rules::PlayerIntentPayloadV1::CommitServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
            item_instance_id,
        } => wire::Intent::CommitServiceTransaction {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            transaction_id: label(transaction_id)?,
            item_instance_id: item_instance_id
                .as_deref()
                .map(wire::ItemInstanceId::new)
                .transpose()?,
        },
        rules::PlayerIntentPayloadV1::BuyFromMerchant {
            service_id,
            capability_id,
            item_instance_ids,
        } => wire::Intent::BuyFromMerchant {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_ids: item_instance_ids
                .iter()
                .map(wire::ItemInstanceId::new)
                .collect::<Result<_, _>>()?,
        },
        rules::PlayerIntentPayloadV1::SellToMerchant {
            service_id,
            capability_id,
            item_instance_id,
        } => wire::Intent::SellToMerchant {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::PlayerIntentPayloadV1::UseItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => wire::Intent::UseItemService {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            operation: item_service_operation(*operation),
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::PlayerIntentPayloadV1::UseRestorationService {
            service_id,
            capability_id,
            operation_id,
            item_instance_id,
            corpse_id,
        } => wire::Intent::UseRestorationService {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            operation_id: label(operation_id)?,
            item_instance_id: item_instance_id
                .as_deref()
                .map(wire::ItemInstanceId::new)
                .transpose()?,
            corpse_id: corpse_id
                .as_ref()
                .map(|value| wire::CorpseId::new(value.as_str()))
                .transpose()?,
        },
        rules::PlayerIntentPayloadV1::InteractWithNpc {
            npc_actor_id,
            interaction_id,
            item_instance_id,
        } => wire::Intent::InteractWithNpc {
            npc_actor_id: actor_id(npc_actor_id)?,
            interaction_id: label(interaction_id)?,
            item_instance_id: item_instance_id
                .as_deref()
                .map(wire::ItemInstanceId::new)
                .transpose()?,
        },
        rules::PlayerIntentPayloadV1::OfferItem {
            recipient_character_id,
            item_instance_id,
        } => wire::Intent::OfferItem {
            recipient_character_id: character_id(recipient_character_id)?,
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::PlayerIntentPayloadV1::AcceptItemOffer {
            item_instance_id,
            destination,
        } => wire::Intent::AcceptItemOffer {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            destination: carried_position(*destination),
        },
        rules::PlayerIntentPayloadV1::RefuseItemOffer { item_instance_id } => {
            wire::Intent::RefuseItemOffer {
                item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            }
        }
        rules::PlayerIntentPayloadV1::WithdrawItemOffer { item_instance_id } => {
            wire::Intent::WithdrawItemOffer {
                item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            }
        }
        rules::PlayerIntentPayloadV1::ClearSelfDefense {
            attacker_character_id,
        } => wire::Intent::ClearSelfDefense {
            attacker_character_id: character_id(attacker_character_id)?,
        },
    })
}

pub(super) fn observer_action_option(
    value: &rules::ActionOptionV1,
) -> Result<wire::ObserverActionOption, wire::ProtocolError> {
    let intent = value
        .command
        .as_ref()
        .map(|command| observer_intent(&command.intent))
        .transpose()?;
    Ok(wire::ObserverActionOption {
        id: wire::ActionId::new(&value.id)?,
        label: wire::ActionLabel::new(&value.label)?,
        enabled: value.enabled,
        blocked_reason: value
            .blocked_reason
            .map(|reason| label(reason.code()))
            .transpose()?,
        intent,
    })
}

pub(super) fn vertical(value: rules::VerticalDirection) -> wire::VerticalDirection {
    match value {
        rules::VerticalDirection::Up => wire::VerticalDirection::Up,
        rules::VerticalDirection::Down => wire::VerticalDirection::Down,
    }
}

pub(super) fn navigation(value: rules::NavigationKind) -> wire::NavigationKind {
    match value {
        rules::NavigationKind::Walk => wire::NavigationKind::Walk,
        rules::NavigationKind::Swim => wire::NavigationKind::Swim,
        rules::NavigationKind::Door => wire::NavigationKind::Door,
        rules::NavigationKind::Stairs { direction } => wire::NavigationKind::Stairs {
            direction: vertical(direction),
        },
        rules::NavigationKind::Pit => wire::NavigationKind::Pit,
        rules::NavigationKind::Climb { direction } => wire::NavigationKind::Climb {
            direction: vertical(direction),
        },
        rules::NavigationKind::Passage => wire::NavigationKind::Passage,
        rules::NavigationKind::Portal => wire::NavigationKind::Portal,
    }
}

pub(super) fn transition(
    value: &rules::TransitionViewV1,
) -> Result<wire::Transition, wire::ProtocolError> {
    let navigation = match value.kind {
        rules::TransitionKindViewV1::Walk => wire::NavigationKind::Walk,
        rules::TransitionKindViewV1::Swim => wire::NavigationKind::Swim,
        rules::TransitionKindViewV1::Door => wire::NavigationKind::Door,
        rules::TransitionKindViewV1::Stairs { direction } => wire::NavigationKind::Stairs {
            direction: vertical(direction),
        },
        rules::TransitionKindViewV1::Pit => wire::NavigationKind::Pit,
        rules::TransitionKindViewV1::Climb { direction } => wire::NavigationKind::Climb {
            direction: vertical(direction),
        },
        rules::TransitionKindViewV1::Passage => wire::NavigationKind::Passage,
        rules::TransitionKindViewV1::Portal => wire::NavigationKind::Portal,
    };
    Ok(wire::Transition {
        navigation,
        target: position(&value.target)?,
        door_open: value
            .door_state
            .map(|state| matches!(state, rules::DoorStateViewV1::Open)),
    })
}

pub fn frame(value: &rules::ObserverFrameV1) -> Result<wire::ObserverFrame, wire::ProtocolError> {
    Ok(wire::ObserverFrame {
        contract_version: value.contract_version,
        logical_time: wire::DecimalU64::new(value.logical_time.as_millis()),
        ready_at: wire::DecimalU64::new(value.ready_at.as_millis()),
        observer_actor_id: actor_id(&value.observer_actor_id)?,
        observation_center: position(&value.observation_center)?,
        observation_radius: value.observation_radius,
        can_act: value.can_act,
        tiles: value
            .tiles
            .iter()
            .map(|tile| {
                Ok(wire::ObserverTile {
                    position: wire::Coord {
                        x: tile.position.x,
                        y: tile.position.y,
                    },
                    terrain_id: tile.terrain_id.as_deref().map(label).transpose()?,
                    terrain_name: tile.terrain_name.as_deref().map(label).transpose()?,
                    passable: tile.passable,
                    move_cost: tile.move_cost,
                    transition: tile.transition.as_ref().map(transition).transpose()?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        actors: value
            .actors
            .iter()
            .map(|actor| {
                Ok(wire::ObserverActor {
                    actor_id: actor_id(&actor.actor_id)?,
                    character_id: actor.character_id.as_ref().map(character_id).transpose()?,
                    name: label(&actor.name)?,
                    kind: match actor.kind {
                        rules::ActorKind::Player => wire::ActorKind::Player,
                        rules::ActorKind::Monster => wire::ActorKind::Monster,
                        rules::ActorKind::Npc => wire::ActorKind::Npc,
                    },
                    position: position(&actor.position)?,
                    life_state: match actor.life_state {
                        rules::ObserverLifeStateV1::Alive => wire::LifeState::Alive,
                        rules::ObserverLifeStateV1::Ghost => wire::LifeState::Ghost,
                        rules::ObserverLifeStateV1::AwaitingResurrection => {
                            wire::LifeState::AwaitingResurrection
                        }
                        rules::ObserverLifeStateV1::Dead => wire::LifeState::Dead,
                    },
                    hp: actor.hp,
                    max_hp: actor.max_hp,
                    attack_safety: attack_safety(actor.attack_safety),
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        corpses: value
            .corpses
            .iter()
            .map(|corpse| {
                Ok(wire::ObserverCorpse {
                    corpse_id: wire::CorpseId::new(corpse.corpse_id.as_str())?,
                    origin_actor_id: actor_id(&corpse.origin_actor_id)?,
                    origin_kind: match corpse.origin_kind {
                        rules::ActorKind::Player => wire::ActorKind::Player,
                        rules::ActorKind::Monster => wire::ActorKind::Monster,
                        rules::ActorKind::Npc => wire::ActorKind::Npc,
                    },
                    origin_name: label(&corpse.origin_name)?,
                    location: position(&corpse.location)?,
                    sequence: wire::DecimalU64::new(corpse.sequence),
                    searched: corpse.searched,
                    loot_claim: corpse.loot_claim.as_ref().map(loot_claim).transpose()?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        corpses_truncated: value.corpses_truncated,
        ground_items: value
            .ground_items
            .iter()
            .map(|item| {
                Ok(wire::ObserverGroundItem {
                    item: observer_item(&item.item)?,
                    location: position(&item.location)?,
                    loot_claim: item.loot_claim.as_ref().map(loot_claim).transpose()?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        ground_items_truncated: value.ground_items_truncated,
        gold_piles: value
            .gold_piles
            .iter()
            .map(|pile| {
                Ok(wire::ObserverGoldPile {
                    gold_pile_id: label(pile.gold_pile_id.as_str())?,
                    amount: wire::DecimalI64::new(pile.amount),
                    location: position(&pile.location)?,
                    loot_claim: pile.loot_claim.as_ref().map(loot_claim).transpose()?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        gold_piles_truncated: value.gold_piles_truncated,
        character: controlled_character(&value.character)?,
        carried: carried_layout(&value.carried)?,
        burden: burden(&value.burden),
        warmed_spell: value.warmed_spell.as_ref().map(warmed_spell).transpose()?,
        spell_actions: value
            .spell_actions
            .iter()
            .map(spell_action)
            .collect::<Result<_, _>>()?,
        services_here: value
            .services_here
            .iter()
            .map(service)
            .collect::<Result<_, _>>()?,
        npcs_here: value.npcs_here.iter().map(npc).collect::<Result<_, _>>()?,
        quest_log: value
            .quest_log
            .iter()
            .map(quest)
            .collect::<Result<_, _>>()?,
        action_options: value
            .action_options
            .iter()
            .map(observer_action_option)
            .collect::<Result<_, wire::ProtocolError>>()?,
        action_options_truncated: value.action_options_truncated,
        social: wire::SocialView {
            character_id: character_id(&value.social.character_id)?,
            group: value
                .social
                .group
                .as_ref()
                .map(|group| {
                    Ok(wire::GroupView {
                        group_id: wire::DecimalU64::new(group.group_id.value()),
                        leader_character_id: character_id(&group.leader_character_id)?,
                        members: group
                            .members
                            .iter()
                            .map(|member| {
                                Ok(wire::GroupMember {
                                    character_id: character_id(&member.character_id)?,
                                    joined_order: wire::DecimalU64::new(member.joined_order),
                                    membership_epoch: wire::DecimalU64::new(
                                        member.membership_epoch,
                                    ),
                                    connected: member.connected,
                                    absent_since: member
                                        .absent_since
                                        .map(|time| wire::DecimalU64::new(time.as_millis())),
                                })
                            })
                            .collect::<Result<_, wire::ProtocolError>>()?,
                    })
                })
                .transpose()?,
            incoming_invitations: value
                .social
                .incoming_invitations
                .iter()
                .map(invitation)
                .collect::<Result<_, _>>()?,
            outgoing_invitations: value
                .social
                .outgoing_invitations
                .iter()
                .map(invitation)
                .collect::<Result<_, _>>()?,
            following_character_id: value
                .social
                .following_character_id
                .as_ref()
                .map(character_id)
                .transpose()?,
            pages_enabled: value.social.pages_enabled,
            blocked_character_ids: value
                .social
                .blocked_character_ids
                .iter()
                .map(character_id)
                .collect::<Result<_, _>>()?,
        },
        incoming_item_offers: value
            .incoming_item_offers
            .iter()
            .map(item_offer)
            .collect::<Result<_, _>>()?,
        outgoing_item_offers: value
            .outgoing_item_offers
            .iter()
            .map(item_offer)
            .collect::<Result<_, _>>()?,
    })
}

pub fn static_scene_context(
    value: &rules::StaticSceneContextV1,
) -> Result<wire::StaticSceneContext, wire::ProtocolError> {
    Ok(wire::StaticSceneContext {
        contract_version: value.contract_version,
        site: wire::StaticSceneSite {
            realm: label(&value.site.realm)?,
            level: label(&value.site.level)?,
        },
        bounds: wire::StaticSceneBounds {
            min: wire::Coord {
                x: value.bounds.min.x,
                y: value.bounds.min.y,
            },
            max: wire::Coord {
                x: value.bounds.max.x,
                y: value.bounds.max.y,
            },
        },
        content_digest: label(&value.content_digest)?,
        visual_manifest_digest: label(&value.visual_manifest_digest)?,
        scene_role: match value.scene_role {
            rules::StaticSceneRoleV1::Overworld => wire::StaticSceneRole::Overworld,
            rules::StaticSceneRoleV1::CombatSpace => wire::StaticSceneRole::CombatSpace,
            rules::StaticSceneRoleV1::Interior => wire::StaticSceneRole::Interior,
        },
        presentation_mode: match value.presentation_mode {
            rules::StaticPresentationModeV1::OverworldTown => wire::PresentationMode::OverworldTown,
            rules::StaticPresentationModeV1::CombatSpace => wire::PresentationMode::CombatSpace,
        },
        world_zoom: value.world_zoom,
        tiles: value
            .tiles
            .iter()
            .map(|tile| {
                Ok(wire::StaticSceneTile {
                    position: wire::Coord {
                        x: tile.position.x,
                        y: tile.position.y,
                    },
                    terrain_ids: tile
                        .terrain_ids
                        .iter()
                        .map(|terrain_id| label(terrain_id))
                        .collect::<Result<_, _>>()?,
                    walkable: tile.walkable,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        walkable_mask: value
            .walkable_mask
            .iter()
            .map(|position| wire::Coord {
                x: position.x,
                y: position.y,
            })
            .collect(),
        static_props: value
            .static_props
            .iter()
            .map(|prop| {
                Ok(wire::StaticSceneProp {
                    id: label(&prop.id)?,
                    visual_family: label(&prop.visual_family)?,
                    anchor: wire::Coord {
                        x: prop.anchor.x,
                        y: prop.anchor.y,
                    },
                    layer: prop.layer,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        transition_apertures: value
            .transition_apertures
            .iter()
            .map(|aperture| {
                Ok(wire::StaticTransitionAperture {
                    at: wire::Coord {
                        x: aperture.at.x,
                        y: aperture.at.y,
                    },
                    navigation: navigation(aperture.navigation),
                    target: position(&aperture.target)?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
    })
}

pub(super) fn invitation(
    value: &rules::ObserverGroupInvitationV2,
) -> Result<wire::GroupInvitation, wire::ProtocolError> {
    Ok(wire::GroupInvitation {
        invitation_id: wire::DecimalU64::new(value.invitation_id.value()),
        issuer_character_id: character_id(&value.issuer_character_id)?,
        target_character_id: character_id(&value.target_character_id)?,
        group_id: value.group_id.map(|id| wire::DecimalU64::new(id.value())),
        expires_at: wire::DecimalU64::new(value.expires_at.as_millis()),
    })
}

pub(super) fn item_offer(
    value: &rules::ItemOfferViewV1,
) -> Result<wire::ItemOffer, wire::ProtocolError> {
    Ok(wire::ItemOffer {
        item: owned_item(&value.item)?,
        sender_character_id: character_id(&value.sender_character_id)?,
        recipient_character_id: character_id(&value.recipient_character_id)?,
        source_position: carried_position(value.source_position),
        actions: action_options(&value.actions)?,
    })
}
