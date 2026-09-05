use super::*;

pub fn actor_id(value: &rules::ActorId) -> Result<wire::ActorId, wire::ProtocolError> {
    wire::ActorId::new(value.as_str())
}

pub fn rules_actor_id(value: &wire::ActorId) -> rules::ActorId {
    rules::ActorId::new(value.as_str())
}

pub(super) fn label(value: &str) -> Result<wire::WireLabel, wire::ProtocolError> {
    wire::WireLabel::new(value)
}

pub(super) fn character_id(
    value: &rules::CharacterId,
) -> Result<wire::CharacterId, wire::ProtocolError> {
    let parsed = uuid::Uuid::parse_str(value.as_str())
        .map_err(|_| wire::ProtocolError::new("rules character ID is not a UUID"))?;
    wire::CharacterId::new(parsed)
}

pub(super) fn rules_character_id(value: wire::CharacterId) -> rules::CharacterId {
    rules::CharacterId::new(value.to_string())
}

pub(super) fn explicit_traversal(
    value: rules::ExplicitTraversalKind,
) -> wire::ExplicitTraversalKind {
    match value {
        rules::ExplicitTraversalKind::StairsUp => wire::ExplicitTraversalKind::StairsUp,
        rules::ExplicitTraversalKind::StairsDown => wire::ExplicitTraversalKind::StairsDown,
        rules::ExplicitTraversalKind::ClimbUp => wire::ExplicitTraversalKind::ClimbUp,
        rules::ExplicitTraversalKind::ClimbDown => wire::ExplicitTraversalKind::ClimbDown,
    }
}

pub(super) fn rules_explicit_traversal(
    value: wire::ExplicitTraversalKind,
) -> rules::ExplicitTraversalKind {
    match value {
        wire::ExplicitTraversalKind::StairsUp => rules::ExplicitTraversalKind::StairsUp,
        wire::ExplicitTraversalKind::StairsDown => rules::ExplicitTraversalKind::StairsDown,
        wire::ExplicitTraversalKind::ClimbUp => rules::ExplicitTraversalKind::ClimbUp,
        wire::ExplicitTraversalKind::ClimbDown => rules::ExplicitTraversalKind::ClimbDown,
    }
}

pub(super) fn carried_position(value: rules::CarriedPosition) -> wire::CarriedPosition {
    use rules::CarriedPosition as R;
    use wire::CarriedPosition as W;
    match value {
        R::LeftHand => W::LeftHand,
        R::RightHand => W::RightHand,
        R::LeftFinger1 => W::LeftFinger1,
        R::LeftFinger2 => W::LeftFinger2,
        R::LeftFinger3 => W::LeftFinger3,
        R::LeftFinger4 => W::LeftFinger4,
        R::RightFinger1 => W::RightFinger1,
        R::RightFinger2 => W::RightFinger2,
        R::RightFinger3 => W::RightFinger3,
        R::RightFinger4 => W::RightFinger4,
        R::Belt1 => W::Belt1,
        R::Belt2 => W::Belt2,
        R::Belt3 => W::Belt3,
        R::Belt4 => W::Belt4,
        R::BeltBack => W::BeltBack,
        R::SackItem1 => W::SackItem1,
        R::SackItem2 => W::SackItem2,
        R::SackItem3 => W::SackItem3,
        R::SackItem4 => W::SackItem4,
        R::SackItem5 => W::SackItem5,
        R::SackItem6 => W::SackItem6,
        R::SackItem7 => W::SackItem7,
        R::SackItem8 => W::SackItem8,
        R::SackItem9 => W::SackItem9,
        R::SackItem10 => W::SackItem10,
        R::SackItem11 => W::SackItem11,
        R::SackItem12 => W::SackItem12,
        R::SackItem13 => W::SackItem13,
        R::SackItem14 => W::SackItem14,
        R::SackItem15 => W::SackItem15,
        R::SackItem16 => W::SackItem16,
        R::SackItem17 => W::SackItem17,
        R::SackItem18 => W::SackItem18,
        R::SackItem19 => W::SackItem19,
        R::SackItem20 => W::SackItem20,
        R::Head => W::Head,
        R::Neck => W::Neck,
        R::LeftArm => W::LeftArm,
        R::RightArm => W::RightArm,
        R::Gloves => W::Gloves,
        R::InnerArmor => W::InnerArmor,
        R::OuterArmor => W::OuterArmor,
        R::Boots => W::Boots,
    }
}

pub(super) fn rules_carried_position(value: wire::CarriedPosition) -> rules::CarriedPosition {
    use rules::CarriedPosition as R;
    use wire::CarriedPosition as W;
    match value {
        W::LeftHand => R::LeftHand,
        W::RightHand => R::RightHand,
        W::LeftFinger1 => R::LeftFinger1,
        W::LeftFinger2 => R::LeftFinger2,
        W::LeftFinger3 => R::LeftFinger3,
        W::LeftFinger4 => R::LeftFinger4,
        W::RightFinger1 => R::RightFinger1,
        W::RightFinger2 => R::RightFinger2,
        W::RightFinger3 => R::RightFinger3,
        W::RightFinger4 => R::RightFinger4,
        W::Belt1 => R::Belt1,
        W::Belt2 => R::Belt2,
        W::Belt3 => R::Belt3,
        W::Belt4 => R::Belt4,
        W::BeltBack => R::BeltBack,
        W::SackItem1 => R::SackItem1,
        W::SackItem2 => R::SackItem2,
        W::SackItem3 => R::SackItem3,
        W::SackItem4 => R::SackItem4,
        W::SackItem5 => R::SackItem5,
        W::SackItem6 => R::SackItem6,
        W::SackItem7 => R::SackItem7,
        W::SackItem8 => R::SackItem8,
        W::SackItem9 => R::SackItem9,
        W::SackItem10 => R::SackItem10,
        W::SackItem11 => R::SackItem11,
        W::SackItem12 => R::SackItem12,
        W::SackItem13 => R::SackItem13,
        W::SackItem14 => R::SackItem14,
        W::SackItem15 => R::SackItem15,
        W::SackItem16 => R::SackItem16,
        W::SackItem17 => R::SackItem17,
        W::SackItem18 => R::SackItem18,
        W::SackItem19 => R::SackItem19,
        W::SackItem20 => R::SackItem20,
        W::Head => R::Head,
        W::Neck => R::Neck,
        W::LeftArm => R::LeftArm,
        W::RightArm => R::RightArm,
        W::Gloves => R::Gloves,
        W::InnerArmor => R::InnerArmor,
        W::OuterArmor => R::OuterArmor,
        W::Boots => R::Boots,
    }
}

pub(super) fn position(
    value: &rules::WorldPosition,
) -> Result<wire::Position, wire::ProtocolError> {
    Ok(wire::Position {
        realm: label(&value.realm)?,
        level: label(&value.level)?,
        position: wire::Coord {
            x: value.position.x,
            y: value.position.y,
        },
    })
}

pub(super) fn rules_position(value: &wire::Position) -> rules::WorldPosition {
    rules::WorldPosition {
        realm: value.realm.as_str().to_string(),
        level: value.level.as_str().to_string(),
        position: rules::Coord {
            x: value.position.x,
            y: value.position.y,
        },
    }
}

pub(super) fn rules_authorization(
    value: wire::HostilityAuthorization,
) -> rules::HostilityAuthorization {
    match value {
        wire::HostilityAuthorization::Safe => rules::HostilityAuthorization::Safe,
        wire::HostilityAuthorization::ConfirmedUnsafe => {
            rules::HostilityAuthorization::ConfirmedUnsafe
        }
    }
}

pub(super) fn rules_physical_mode(value: wire::PhysicalAttackMode) -> rules::PhysicalAttackMode {
    match value {
        wire::PhysicalAttackMode::Fight => rules::PhysicalAttackMode::Fight,
        wire::PhysicalAttackMode::Kick => rules::PhysicalAttackMode::Kick,
        wire::PhysicalAttackMode::Jumpkick => rules::PhysicalAttackMode::Jumpkick,
        wire::PhysicalAttackMode::Poke => rules::PhysicalAttackMode::Poke,
        wire::PhysicalAttackMode::Shoot => rules::PhysicalAttackMode::Shoot,
        wire::PhysicalAttackMode::Throw => rules::PhysicalAttackMode::Throw,
    }
}

pub(super) fn rules_spell_target(value: &wire::SpellTarget) -> rules::SpellTarget {
    match value {
        wire::SpellTarget::None => rules::SpellTarget::None,
        wire::SpellTarget::SelfTarget => rules::SpellTarget::SelfTarget,
        wire::SpellTarget::Actor { actor_id } => rules::SpellTarget::Actor {
            actor_id: rules_actor_id(actor_id),
        },
        wire::SpellTarget::Path { directions } => rules::SpellTarget::Path {
            directions: directions.iter().map(rules_direction).collect(),
        },
        wire::SpellTarget::Coordinate { position } => rules::SpellTarget::Coordinate {
            position: rules_position(position),
        },
        wire::SpellTarget::Area { center } => rules::SpellTarget::Area {
            center: rules_position(center),
        },
        wire::SpellTarget::Direction { direction } => rules::SpellTarget::Direction {
            direction: rules_direction(direction),
        },
        wire::SpellTarget::Door { direction } => rules::SpellTarget::Door {
            direction: rules_direction(direction),
        },
        wire::SpellTarget::Item {
            item_instance_id,
            location,
        } => rules::SpellTarget::Item {
            item_instance_id: item_instance_id.as_str().to_string(),
            location: match location {
                wire::SpellItemLocation::Sack => rules::SpellItemLocation::Sack,
                wire::SpellItemLocation::ActiveEquipment => {
                    rules::SpellItemLocation::ActiveEquipment
                }
                wire::SpellItemLocation::GroundHere => rules::SpellItemLocation::GroundHere,
            },
        },
    }
}

pub(super) fn attack_safety(value: rules::AttackSafety) -> wire::AttackSafety {
    match value {
        rules::AttackSafety::Invalid => wire::AttackSafety::Invalid,
        rules::AttackSafety::Protected => wire::AttackSafety::Protected,
        rules::AttackSafety::OpenSelfDefense => wire::AttackSafety::OpenSelfDefense,
        rules::AttackSafety::OpenEvilPlayer => wire::AttackSafety::OpenEvilPlayer,
        rules::AttackSafety::OpenHostile => wire::AttackSafety::OpenHostile,
    }
}

pub(super) fn authorization(value: rules::HostilityAuthorization) -> wire::HostilityAuthorization {
    match value {
        rules::HostilityAuthorization::Safe => wire::HostilityAuthorization::Safe,
        rules::HostilityAuthorization::ConfirmedUnsafe => {
            wire::HostilityAuthorization::ConfirmedUnsafe
        }
    }
}

pub(super) fn physical_mode(value: rules::PhysicalAttackMode) -> wire::PhysicalAttackMode {
    match value {
        rules::PhysicalAttackMode::Fight => wire::PhysicalAttackMode::Fight,
        rules::PhysicalAttackMode::Kick => wire::PhysicalAttackMode::Kick,
        rules::PhysicalAttackMode::Jumpkick => wire::PhysicalAttackMode::Jumpkick,
        rules::PhysicalAttackMode::Poke => wire::PhysicalAttackMode::Poke,
        rules::PhysicalAttackMode::Shoot => wire::PhysicalAttackMode::Shoot,
        rules::PhysicalAttackMode::Throw => wire::PhysicalAttackMode::Throw,
    }
}

pub(super) fn actor_kind(value: rules::ActorKind) -> wire::ActorKind {
    match value {
        rules::ActorKind::Player => wire::ActorKind::Player,
        rules::ActorKind::Monster => wire::ActorKind::Monster,
        rules::ActorKind::Npc => wire::ActorKind::Npc,
    }
}

pub(super) fn life_state(value: rules::ObserverLifeStateV1) -> wire::LifeState {
    match value {
        rules::ObserverLifeStateV1::Alive => wire::LifeState::Alive,
        rules::ObserverLifeStateV1::Ghost => wire::LifeState::Ghost,
        rules::ObserverLifeStateV1::AwaitingResurrection => wire::LifeState::AwaitingResurrection,
        rules::ObserverLifeStateV1::Dead => wire::LifeState::Dead,
    }
}

pub(super) fn resource_kind(value: rules::ResourceKind) -> wire::ResourceKind {
    match value {
        rules::ResourceKind::Hp => wire::ResourceKind::Hp,
        rules::ResourceKind::Mp => wire::ResourceKind::Mp,
        rules::ResourceKind::Stamina => wire::ResourceKind::Stamina,
    }
}

pub(super) fn restoration_status(
    value: rules::RestorationStatusKind,
) -> wire::RestorationStatusKind {
    match value {
        rules::RestorationStatusKind::Blindness => wire::RestorationStatusKind::Blindness,
        rules::RestorationStatusKind::Poison => wire::RestorationStatusKind::Poison,
    }
}

pub(super) fn feedback_resurrection_method(
    value: rules::ResurrectionMethod,
) -> wire::FeedbackResurrectionMethod {
    match value {
        rules::ResurrectionMethod::Gods => wire::FeedbackResurrectionMethod::Gods,
        rules::ResurrectionMethod::Priest => wire::FeedbackResurrectionMethod::Priest,
        rules::ResurrectionMethod::Thaumaturge => wire::FeedbackResurrectionMethod::Thaumaturge,
    }
}

pub(super) fn npc_interaction_outcome(
    value: &rules::NpcInteractionOutcome,
) -> Result<wire::NpcInteractionOutcome, wire::ProtocolError> {
    Ok(match value {
        rules::NpcInteractionOutcome::Speak => wire::NpcInteractionOutcome::Speak,
        rules::NpcInteractionOutcome::BeginFollow => wire::NpcInteractionOutcome::BeginFollow,
        rules::NpcInteractionOutcome::EndFollow => wire::NpcInteractionOutcome::EndFollow,
        rules::NpcInteractionOutcome::CompleteEscort { npc_actor_id } => {
            wire::NpcInteractionOutcome::CompleteEscort {
                npc_actor_id: actor_id(npc_actor_id)?,
            }
        }
        rules::NpcInteractionOutcome::Climb { direction: value } => {
            wire::NpcInteractionOutcome::Climb {
                direction: vertical(*value),
            }
        }
    })
}

pub(super) fn direction(value: rules::Direction) -> wire::Direction {
    match value {
        rules::Direction::North => wire::Direction::North,
        rules::Direction::Northeast => wire::Direction::Northeast,
        rules::Direction::East => wire::Direction::East,
        rules::Direction::Southeast => wire::Direction::Southeast,
        rules::Direction::South => wire::Direction::South,
        rules::Direction::Southwest => wire::Direction::Southwest,
        rules::Direction::West => wire::Direction::West,
        rules::Direction::Northwest => wire::Direction::Northwest,
    }
}

pub(super) fn spell_target(
    value: &rules::SpellTarget,
) -> Result<wire::SpellTarget, wire::ProtocolError> {
    Ok(match value {
        rules::SpellTarget::None => wire::SpellTarget::None,
        rules::SpellTarget::SelfTarget => wire::SpellTarget::SelfTarget,
        rules::SpellTarget::Actor { actor_id: value } => wire::SpellTarget::Actor {
            actor_id: actor_id(value)?,
        },
        rules::SpellTarget::Path { directions } => wire::SpellTarget::Path {
            directions: directions.iter().copied().map(direction).collect(),
        },
        rules::SpellTarget::Coordinate { position: value } => wire::SpellTarget::Coordinate {
            position: position(value)?,
        },
        rules::SpellTarget::Area { center } => wire::SpellTarget::Area {
            center: position(center)?,
        },
        rules::SpellTarget::Direction { direction: value } => wire::SpellTarget::Direction {
            direction: direction(*value),
        },
        rules::SpellTarget::Door { direction: value } => wire::SpellTarget::Door {
            direction: direction(*value),
        },
        rules::SpellTarget::Item {
            item_instance_id,
            location,
        } => wire::SpellTarget::Item {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            location: match location {
                rules::SpellItemLocation::Sack => wire::SpellItemLocation::Sack,
                rules::SpellItemLocation::ActiveEquipment => {
                    wire::SpellItemLocation::ActiveEquipment
                }
                rules::SpellItemLocation::GroundHere => wire::SpellItemLocation::GroundHere,
            },
        },
    })
}

pub(super) fn observer_item(
    value: &rules::ObserverItemV1,
) -> Result<wire::ObserverItem, wire::ProtocolError> {
    Ok(wire::ObserverItem {
        item_instance_id: wire::ItemInstanceId::new(&value.item_instance_id)?,
        item_definition_id: label(&value.item_definition_id)?,
        name: label(&value.name)?,
        quantity: value.quantity,
        binding: match value.binding {
            rules::ObserverItemBindingV1::Unbound => wire::ObserverItemBinding::Unbound,
            rules::ObserverItemBindingV1::Bound => wire::ObserverItemBinding::Bound,
        },
    })
}

pub(super) fn character_alignment(value: rules::CharacterAlignment) -> wire::CharacterAlignment {
    match value {
        rules::CharacterAlignment::Lawful => wire::CharacterAlignment::Lawful,
        rules::CharacterAlignment::Neutral => wire::CharacterAlignment::Neutral,
        rules::CharacterAlignment::Chaotic => wire::CharacterAlignment::Chaotic,
        rules::CharacterAlignment::Evil => wire::CharacterAlignment::Evil,
    }
}

pub(super) fn controlled_character(
    value: &rules::CharacterSheetViewV1,
) -> Result<wire::ControlledCharacter, wire::ProtocolError> {
    Ok(wire::ControlledCharacter {
        identity: wire::CharacterIdentity {
            base_class_id: label(&value.identity.base_class_id)?,
            current_class_id: label(&value.identity.current_class_id)?,
            display_class: label(&value.identity.display_class)?,
            nationality_id: label(&value.identity.nationality_id)?,
            sex_or_gender_display: value
                .identity
                .sex_or_gender_display
                .as_deref()
                .map(label)
                .transpose()?,
        },
        alignment: character_alignment(value.alignment_state.alignment),
        karma_points: value.alignment_state.karma_points,
        attributes: wire::CharacterAttributes {
            strength: value.attributes.strength,
            dexterity: value.attributes.dexterity,
            constitution: value.attributes.constitution,
            intelligence: value.attributes.intelligence,
            wisdom: value.attributes.wisdom,
            charisma: value.attributes.charisma,
        },
        resources: wire::CharacterResources {
            hp: value.resources.hp,
            max_hp: value.resources.max_hp,
            peak_hp: value.resources.peak_hp,
            mp: value.resources.mp,
            max_mp: value.resources.max_mp,
            stamina: value.resources.stamina,
            max_stamina: value.resources.max_stamina,
        },
        progression: wire::CharacterProgression {
            level: value.progression.level,
            experience: wire::DecimalI64::new(value.progression.experience),
            pending_target_level: value.progression.pending_target_level,
        },
        physical_attribute_adds: wire::PhysicalAttributeAdds {
            strength_adds: value.physical_attribute_adds.strength_adds,
            dexterity_adds: value.physical_attribute_adds.dexterity_adds,
        },
        promotion_history: value
            .promotion_history
            .iter()
            .map(|entry| {
                Ok(wire::PromotionEntry {
                    from_class_id: label(&entry.from_class_id)?,
                    to_class_id: label(&entry.to_class_id)?,
                    level: entry.level,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        known_spells: value
            .known_spells
            .iter()
            .map(|spell| {
                Ok(wire::KnownSpell {
                    spell_id: label(&spell.spell_id)?,
                    lane: label(&spell.lane)?,
                    learned_at_level: spell.learned_at_level,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        skill_ledger: value
            .skill_ledger
            .iter()
            .map(|entry| {
                Ok(wire::SkillEntry {
                    track_id: label(&entry.track_id)?,
                    level: entry.level,
                    critique_rank: entry.critique_rank,
                    practice_points: wire::DecimalU64::new(entry.practice_points),
                    learning_rate: wire::DecimalU64::new(entry.learning_rate),
                    track_display: entry.track_display.as_deref().map(label).transpose()?,
                    level_title: entry.level_title.as_deref().map(label).transpose()?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
    })
}

pub(super) fn owned_item(
    value: &rules::ItemInstanceViewV1,
) -> Result<wire::OwnedItem, wire::ProtocolError> {
    Ok(wire::OwnedItem {
        item_instance_id: wire::ItemInstanceId::new(&value.item_instance_id)?,
        item_definition_id: label(&value.item_definition_id)?,
        name: label(&value.name)?,
        quantity: value.quantity,
        identified: value.identified,
        appraised: value.appraised,
        known_unit_value_gold: value.known_unit_value_gold.map(wire::DecimalU64::new),
        known_stack_value_gold: value.known_stack_value_gold.map(wire::DecimalU64::new),
        unit_burden: wire::DecimalU64::new(value.unit_burden),
        stack_burden: wire::DecimalU64::new(value.stack_burden),
        binding: match value.binding {
            rules::ItemBindingViewV1::Unrestricted => wire::OwnedItemBinding::Unrestricted,
            rules::ItemBindingViewV1::BindOnFirstCharacterTouch => {
                wire::OwnedItemBinding::BindOnFirstCharacterTouch
            }
            rules::ItemBindingViewV1::Bound => wire::OwnedItemBinding::Bound,
        },
        bow_readiness: value.bow_readiness.map(|readiness| match readiness {
            rules::BowReadiness::Unnocked => wire::BowReadiness::Unnocked,
            rules::BowReadiness::Nocked => wire::BowReadiness::Nocked,
        }),
    })
}

pub(super) fn item_placement(value: rules::ItemPlacementKind) -> wire::ItemPlacementKind {
    match value {
        rules::ItemPlacementKind::Hand => wire::ItemPlacementKind::Hand,
        rules::ItemPlacementKind::RingFinger => wire::ItemPlacementKind::RingFinger,
        rules::ItemPlacementKind::BeltSide => wire::ItemPlacementKind::BeltSide,
        rules::ItemPlacementKind::BeltBack => wire::ItemPlacementKind::BeltBack,
        rules::ItemPlacementKind::Sack => wire::ItemPlacementKind::Sack,
        rules::ItemPlacementKind::Head => wire::ItemPlacementKind::Head,
        rules::ItemPlacementKind::Neck => wire::ItemPlacementKind::Neck,
        rules::ItemPlacementKind::Arm => wire::ItemPlacementKind::Arm,
        rules::ItemPlacementKind::Gloves => wire::ItemPlacementKind::Gloves,
        rules::ItemPlacementKind::InnerArmor => wire::ItemPlacementKind::InnerArmor,
        rules::ItemPlacementKind::OuterArmor => wire::ItemPlacementKind::OuterArmor,
        rules::ItemPlacementKind::Boots => wire::ItemPlacementKind::Boots,
    }
}

pub(super) fn carried_layout(
    value: &rules::CarriedLayoutViewV1,
) -> Result<wire::CarriedLayout, wire::ProtocolError> {
    Ok(wire::CarriedLayout {
        items: value
            .items
            .iter()
            .map(|positioned| {
                Ok(wire::PositionedItem {
                    position: carried_position(positioned.position),
                    item: owned_item(&positioned.item)?,
                    valid_placements: positioned
                        .valid_placements
                        .iter()
                        .copied()
                        .map(item_placement)
                        .collect(),
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        gold: wire::CarriedGold {
            left_hand: wire::DecimalI64::new(value.gold.left_hand),
            right_hand: wire::DecimalI64::new(value.gold.right_hand),
            sack: wire::DecimalI64::new(value.gold.sack),
        },
    })
}

pub(super) fn burden(value: &rules::BurdenViewV1) -> wire::Burden {
    wire::Burden {
        item_burden: wire::DecimalU64::new(value.item_burden),
        coin_burden: wire::DecimalU64::new(value.coin_burden),
        total_burden: wire::DecimalU64::new(value.total_burden),
        lightly_loaded_limit: value.lightly_loaded_limit.map(wire::DecimalU64::new),
        moderately_loaded_limit: value.moderately_loaded_limit.map(wire::DecimalU64::new),
        heavily_loaded_limit: value.heavily_loaded_limit.map(wire::DecimalU64::new),
        tier: value.tier.map(|tier| match tier {
            rules::BurdenTier::LightlyLoaded => wire::BurdenTier::LightlyLoaded,
            rules::BurdenTier::ModeratelyLoaded => wire::BurdenTier::ModeratelyLoaded,
            rules::BurdenTier::HeavilyLoaded => wire::BurdenTier::HeavilyLoaded,
            rules::BurdenTier::VeryHeavilyLoaded => wire::BurdenTier::VeryHeavilyLoaded,
        }),
    }
}

pub(super) fn warmed_spell(
    value: &rules::WarmedSpellViewV1,
) -> Result<wire::WarmedSpell, wire::ProtocolError> {
    Ok(wire::WarmedSpell {
        spell_id: label(&value.spell_id)?,
        warmed_at: wire::DecimalU64::new(value.warmed_at.as_millis()),
        ready_at: wire::DecimalU64::new(value.ready_at.as_millis()),
        status: match value.status {
            rules::WarmedSpellStatus::Warming => wire::WarmedSpellStatus::Warming,
            rules::WarmedSpellStatus::Ready => wire::WarmedSpellStatus::Ready,
        },
    })
}

pub(super) fn spell_action_state(
    value: &rules::SpellActionStateV1,
) -> Result<wire::SpellActionState, wire::ProtocolError> {
    Ok(wire::SpellActionState {
        enabled: value.enabled,
        blocked_reason: value
            .blocked_reason
            .map(|reason| label(reason.code()))
            .transpose()?,
        requires_target_selection: value.requires_target_selection,
        intent: value
            .command
            .as_ref()
            .map(|command| observer_intent(&command.intent))
            .transpose()?,
    })
}

pub(super) fn spell_action(
    value: &rules::SpellActionV1,
) -> Result<wire::SpellAction, wire::ProtocolError> {
    Ok(wire::SpellAction {
        spell_id: label(&value.spell_id)?,
        spell_name: label(&value.spell_name)?,
        casting_method: match value.casting_method {
            rules::SpellCastingMethod::Direct => wire::SpellCastingMethod::Direct,
            rules::SpellCastingMethod::WarmThenCast => wire::SpellCastingMethod::WarmThenCast,
        },
        cast_class: match value.cast_class {
            rules::SpellCastClass::Character => wire::SpellCastClass::Character,
            rules::SpellCastClass::Path => wire::SpellCastClass::Path,
            rules::SpellCastClass::PathOrCharacter => wire::SpellCastClass::PathOrCharacter,
            rules::SpellCastClass::SelfTarget => wire::SpellCastClass::SelfTarget,
            rules::SpellCastClass::NotApplicable => wire::SpellCastClass::NotApplicable,
        },
        target_kind: value.target_kind.map(|kind| match kind {
            rules::SpellTargetKind::Actor => wire::SpellTargetKind::Actor,
            rules::SpellTargetKind::Area => wire::SpellTargetKind::Area,
            rules::SpellTargetKind::Coordinate => wire::SpellTargetKind::Coordinate,
            rules::SpellTargetKind::Direction => wire::SpellTargetKind::Direction,
            rules::SpellTargetKind::Door => wire::SpellTargetKind::Door,
            rules::SpellTargetKind::Item => wire::SpellTargetKind::Item,
            rules::SpellTargetKind::None => wire::SpellTargetKind::None,
            rules::SpellTargetKind::SelfTarget => wire::SpellTargetKind::SelfTarget,
        }),
        mp_cost: value.mp_cost,
        stamina_cost: value.stamina_cost,
        hostile_act: value.social.hostile_act,
        town_law_violation: matches!(
            value.social.town_law,
            rules::SpellTownLawViewV1::TerrainAlignmentViolation
        ),
        warm: spell_action_state(&value.warm)?,
        cast: spell_action_state(&value.cast)?,
    })
}
