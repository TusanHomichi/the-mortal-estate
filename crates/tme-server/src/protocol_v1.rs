use tme_protocol as wire;
use tme_rules as rules;

pub fn actor_id(value: &rules::ActorId) -> Result<wire::ActorId, wire::ProtocolError> {
    wire::ActorId::new(value.as_str())
}

pub fn rules_actor_id(value: &wire::ActorId) -> rules::ActorId {
    rules::ActorId::new(value.as_str())
}

fn label(value: &str) -> Result<wire::WireLabel, wire::ProtocolError> {
    wire::WireLabel::new(value)
}

fn character_id(value: &rules::CharacterId) -> Result<wire::CharacterId, wire::ProtocolError> {
    let parsed = uuid::Uuid::parse_str(value.as_str())
        .map_err(|_| wire::ProtocolError::new("rules character ID is not a UUID"))?;
    wire::CharacterId::new(parsed)
}

fn rules_character_id(value: wire::CharacterId) -> rules::CharacterId {
    rules::CharacterId::new(value.to_string())
}

fn explicit_traversal(value: rules::ExplicitTraversalKind) -> wire::ExplicitTraversalKind {
    match value {
        rules::ExplicitTraversalKind::StairsUp => wire::ExplicitTraversalKind::StairsUp,
        rules::ExplicitTraversalKind::StairsDown => wire::ExplicitTraversalKind::StairsDown,
        rules::ExplicitTraversalKind::ClimbUp => wire::ExplicitTraversalKind::ClimbUp,
        rules::ExplicitTraversalKind::ClimbDown => wire::ExplicitTraversalKind::ClimbDown,
    }
}

fn rules_explicit_traversal(value: wire::ExplicitTraversalKind) -> rules::ExplicitTraversalKind {
    match value {
        wire::ExplicitTraversalKind::StairsUp => rules::ExplicitTraversalKind::StairsUp,
        wire::ExplicitTraversalKind::StairsDown => rules::ExplicitTraversalKind::StairsDown,
        wire::ExplicitTraversalKind::ClimbUp => rules::ExplicitTraversalKind::ClimbUp,
        wire::ExplicitTraversalKind::ClimbDown => rules::ExplicitTraversalKind::ClimbDown,
    }
}

fn carried_position(value: rules::CarriedPosition) -> wire::CarriedPosition {
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

fn rules_carried_position(value: wire::CarriedPosition) -> rules::CarriedPosition {
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

fn position(value: &rules::WorldPosition) -> Result<wire::Position, wire::ProtocolError> {
    Ok(wire::Position {
        realm: label(&value.realm)?,
        level: label(&value.level)?,
        position: wire::Coord {
            x: value.position.x,
            y: value.position.y,
        },
    })
}

fn rules_position(value: &wire::Position) -> rules::WorldPosition {
    rules::WorldPosition {
        realm: value.realm.as_str().to_string(),
        level: value.level.as_str().to_string(),
        position: rules::Coord {
            x: value.position.x,
            y: value.position.y,
        },
    }
}

fn rules_authorization(value: wire::HostilityAuthorization) -> rules::HostilityAuthorization {
    match value {
        wire::HostilityAuthorization::Safe => rules::HostilityAuthorization::Safe,
        wire::HostilityAuthorization::ConfirmedUnsafe => {
            rules::HostilityAuthorization::ConfirmedUnsafe
        }
    }
}

fn rules_physical_mode(value: wire::PhysicalAttackMode) -> rules::PhysicalAttackMode {
    match value {
        wire::PhysicalAttackMode::Fight => rules::PhysicalAttackMode::Fight,
        wire::PhysicalAttackMode::Kick => rules::PhysicalAttackMode::Kick,
        wire::PhysicalAttackMode::Jumpkick => rules::PhysicalAttackMode::Jumpkick,
        wire::PhysicalAttackMode::Poke => rules::PhysicalAttackMode::Poke,
        wire::PhysicalAttackMode::Shoot => rules::PhysicalAttackMode::Shoot,
        wire::PhysicalAttackMode::Throw => rules::PhysicalAttackMode::Throw,
    }
}

fn rules_spell_target(value: &wire::SpellTarget) -> rules::SpellTarget {
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

fn attack_safety(value: rules::AttackSafety) -> wire::AttackSafety {
    match value {
        rules::AttackSafety::Invalid => wire::AttackSafety::Invalid,
        rules::AttackSafety::Protected => wire::AttackSafety::Protected,
        rules::AttackSafety::OpenSelfDefense => wire::AttackSafety::OpenSelfDefense,
        rules::AttackSafety::OpenEvilPlayer => wire::AttackSafety::OpenEvilPlayer,
        rules::AttackSafety::OpenHostile => wire::AttackSafety::OpenHostile,
    }
}

fn authorization(value: rules::HostilityAuthorization) -> wire::HostilityAuthorization {
    match value {
        rules::HostilityAuthorization::Safe => wire::HostilityAuthorization::Safe,
        rules::HostilityAuthorization::ConfirmedUnsafe => {
            wire::HostilityAuthorization::ConfirmedUnsafe
        }
    }
}

fn physical_mode(value: rules::PhysicalAttackMode) -> wire::PhysicalAttackMode {
    match value {
        rules::PhysicalAttackMode::Fight => wire::PhysicalAttackMode::Fight,
        rules::PhysicalAttackMode::Kick => wire::PhysicalAttackMode::Kick,
        rules::PhysicalAttackMode::Jumpkick => wire::PhysicalAttackMode::Jumpkick,
        rules::PhysicalAttackMode::Poke => wire::PhysicalAttackMode::Poke,
        rules::PhysicalAttackMode::Shoot => wire::PhysicalAttackMode::Shoot,
        rules::PhysicalAttackMode::Throw => wire::PhysicalAttackMode::Throw,
    }
}

fn actor_kind(value: rules::ActorKind) -> wire::ActorKind {
    match value {
        rules::ActorKind::Player => wire::ActorKind::Player,
        rules::ActorKind::Monster => wire::ActorKind::Monster,
        rules::ActorKind::Npc => wire::ActorKind::Npc,
    }
}

fn life_state(value: rules::ObserverLifeStateV1) -> wire::LifeState {
    match value {
        rules::ObserverLifeStateV1::Alive => wire::LifeState::Alive,
        rules::ObserverLifeStateV1::Ghost => wire::LifeState::Ghost,
        rules::ObserverLifeStateV1::AwaitingResurrection => wire::LifeState::AwaitingResurrection,
        rules::ObserverLifeStateV1::Dead => wire::LifeState::Dead,
    }
}

fn resource_kind(value: rules::ResourceKind) -> wire::ResourceKind {
    match value {
        rules::ResourceKind::Hp => wire::ResourceKind::Hp,
        rules::ResourceKind::Mp => wire::ResourceKind::Mp,
        rules::ResourceKind::Stamina => wire::ResourceKind::Stamina,
    }
}

fn restoration_status(value: rules::RestorationStatusKind) -> wire::RestorationStatusKind {
    match value {
        rules::RestorationStatusKind::Blindness => wire::RestorationStatusKind::Blindness,
        rules::RestorationStatusKind::Poison => wire::RestorationStatusKind::Poison,
    }
}

fn feedback_resurrection_method(
    value: rules::ResurrectionMethod,
) -> wire::FeedbackResurrectionMethod {
    match value {
        rules::ResurrectionMethod::Gods => wire::FeedbackResurrectionMethod::Gods,
        rules::ResurrectionMethod::Priest => wire::FeedbackResurrectionMethod::Priest,
        rules::ResurrectionMethod::Thaumaturge => wire::FeedbackResurrectionMethod::Thaumaturge,
    }
}

fn npc_interaction_outcome(
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

fn direction(value: rules::Direction) -> wire::Direction {
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

fn spell_target(value: &rules::SpellTarget) -> Result<wire::SpellTarget, wire::ProtocolError> {
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

fn observer_item(value: &rules::ObserverItemV1) -> Result<wire::ObserverItem, wire::ProtocolError> {
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

fn character_alignment(value: rules::CharacterAlignment) -> wire::CharacterAlignment {
    match value {
        rules::CharacterAlignment::Lawful => wire::CharacterAlignment::Lawful,
        rules::CharacterAlignment::Neutral => wire::CharacterAlignment::Neutral,
        rules::CharacterAlignment::Chaotic => wire::CharacterAlignment::Chaotic,
        rules::CharacterAlignment::Evil => wire::CharacterAlignment::Evil,
    }
}

fn controlled_character(
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

fn owned_item(value: &rules::ItemInstanceViewV1) -> Result<wire::OwnedItem, wire::ProtocolError> {
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

fn item_placement(value: rules::ItemPlacementKind) -> wire::ItemPlacementKind {
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

fn carried_layout(
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

fn burden(value: &rules::BurdenViewV1) -> wire::Burden {
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

fn warmed_spell(
    value: &rules::WarmedSpellViewV1,
) -> Result<wire::WarmedSpell, wire::ProtocolError> {
    Ok(wire::WarmedSpell {
        spell_id: label(&value.spell_id)?,
        warmed_at: wire::DecimalU64::new(value.warmed_at.value()),
        ready_at: wire::DecimalU64::new(value.ready_at.value()),
        status: match value.status {
            rules::WarmedSpellStatus::Warming => wire::WarmedSpellStatus::Warming,
            rules::WarmedSpellStatus::Ready => wire::WarmedSpellStatus::Ready,
        },
    })
}

fn spell_action_state(
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

fn spell_action(value: &rules::SpellActionV1) -> Result<wire::SpellAction, wire::ProtocolError> {
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

fn transaction_requirement(
    value: &rules::TransactionRequirementViewV1,
) -> Result<wire::TransactionRequirement, wire::ProtocolError> {
    Ok(match value {
        rules::TransactionRequirementViewV1::CurrentClass { class_id } => {
            wire::TransactionRequirement::CurrentClass {
                class_id: label(class_id)?,
            }
        }
        rules::TransactionRequirementViewV1::MinimumLevel { level } => {
            wire::TransactionRequirement::MinimumLevel { level: *level }
        }
        rules::TransactionRequirementViewV1::ExactKarma { karma_points } => {
            wire::TransactionRequirement::ExactKarma {
                karma_points: *karma_points,
            }
        }
        rules::TransactionRequirementViewV1::ExactAlignment { alignment } => {
            wire::TransactionRequirement::ExactAlignment {
                alignment: character_alignment(*alignment),
            }
        }
        rules::TransactionRequirementViewV1::MinimumSkillLevel { track_id, level } => {
            wire::TransactionRequirement::MinimumSkillLevel {
                track_id: label(track_id)?,
                level: *level,
            }
        }
        rules::TransactionRequirementViewV1::MinimumCarriedGold { amount } => {
            wire::TransactionRequirement::MinimumCarriedGold {
                amount: wire::DecimalI64::new(*amount),
            }
        }
        rules::TransactionRequirementViewV1::CarriedItem {
            item_definition_id,
            quantity,
        } => wire::TransactionRequirement::CarriedItem {
            item_definition_id: label(item_definition_id)?,
            quantity: *quantity,
        },
        rules::TransactionRequirementViewV1::CarriedPositionEmpty { position } => {
            wire::TransactionRequirement::CarriedPositionEmpty {
                position: carried_position(*position),
            }
        }
        rules::TransactionRequirementViewV1::SpellUnknown { spell_id } => {
            wire::TransactionRequirement::SpellUnknown {
                spell_id: label(spell_id)?,
            }
        }
        rules::TransactionRequirementViewV1::QuestUnstarted { quest_id } => {
            wire::TransactionRequirement::QuestUnstarted {
                quest_id: label(quest_id)?,
            }
        }
        rules::TransactionRequirementViewV1::QuestAtStage { quest_id, stage_id } => {
            wire::TransactionRequirement::QuestAtStage {
                quest_id: label(quest_id)?,
                stage_id: label(stage_id)?,
            }
        }
        rules::TransactionRequirementViewV1::NpcAccompanying { npc_actor_id } => {
            wire::TransactionRequirement::NpcAccompanying {
                npc_actor_id: actor_id(npc_actor_id)?,
            }
        }
    })
}

fn transaction_cost(value: &rules::TransactionCostViewV1) -> wire::TransactionCost {
    match value {
        rules::TransactionCostViewV1::CarriedGold { amount } => {
            wire::TransactionCost::CarriedGold {
                amount: wire::DecimalI64::new(*amount),
            }
        }
        rules::TransactionCostViewV1::SelectedCarriedItem { quantity } => {
            wire::TransactionCost::SelectedCarriedItem {
                quantity: *quantity,
            }
        }
    }
}

fn transaction_reward(
    value: &rules::TransactionRewardViewV1,
) -> Result<wire::TransactionReward, wire::ProtocolError> {
    Ok(match value {
        rules::TransactionRewardViewV1::Experience { amount } => {
            wire::TransactionReward::Experience { amount: *amount }
        }
        rules::TransactionRewardViewV1::Item {
            item_instance_id,
            item_definition_id,
            position,
        } => wire::TransactionReward::Item {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            position: carried_position(*position),
        },
        rules::TransactionRewardViewV1::Class {
            to_class_id,
            to_class_display,
        } => wire::TransactionReward::Class {
            to_class_id: label(to_class_id)?,
            to_class_display: label(to_class_display)?,
        },
        rules::TransactionRewardViewV1::Spell { spell_id } => wire::TransactionReward::Spell {
            spell_id: label(spell_id)?,
        },
        rules::TransactionRewardViewV1::QuestStage { quest_id, stage_id } => {
            wire::TransactionReward::QuestStage {
                quest_id: label(quest_id)?,
                stage_id: label(stage_id)?,
            }
        }
    })
}

fn action_options(
    values: &[rules::ActionOptionV1],
) -> Result<Vec<wire::ObserverActionOption>, wire::ProtocolError> {
    values.iter().map(observer_action_option).collect()
}

fn service_transaction(
    value: &rules::ServiceTransactionViewV1,
) -> Result<wire::ServiceTransaction, wire::ProtocolError> {
    Ok(wire::ServiceTransaction {
        transaction_id: label(&value.transaction_id)?,
        label: label(&value.label)?,
        requirements: value
            .requirements
            .iter()
            .map(transaction_requirement)
            .collect::<Result<_, _>>()?,
        costs: value.costs.iter().map(transaction_cost).collect(),
        rewards: value
            .rewards
            .iter()
            .map(transaction_reward)
            .collect::<Result<_, _>>()?,
        actions: action_options(&value.actions)?,
    })
}

fn restoration_outcome(value: &rules::RestorationOutcomeViewV1) -> wire::RestorationOutcome {
    match value {
        rules::RestorationOutcomeViewV1::RestoreResource { resource } => {
            wire::RestorationOutcome::RestoreResource {
                resource: match resource {
                    rules::ResourceKind::Hp => wire::ResourceKind::Hp,
                    rules::ResourceKind::Mp => wire::ResourceKind::Mp,
                    rules::ResourceKind::Stamina => wire::ResourceKind::Stamina,
                },
            }
        }
        rules::RestorationOutcomeViewV1::CureStatus { status } => {
            wire::RestorationOutcome::CureStatus {
                status: match status {
                    rules::RestorationStatusKind::Blindness => {
                        wire::RestorationStatusKind::Blindness
                    }
                    rules::RestorationStatusKind::Poison => wire::RestorationStatusKind::Poison,
                },
            }
        }
        rules::RestorationOutcomeViewV1::PriestResurrection => {
            wire::RestorationOutcome::PriestResurrection
        }
    }
}

fn service_capability(
    value: &rules::ServiceCapabilityViewV1,
) -> Result<wire::ServiceCapability, wire::ProtocolError> {
    Ok(match value {
        rules::ServiceCapabilityViewV1::SkillTraining {
            capability_id,
            offered_track_ids,
            selected_track_id,
            actions,
        } => wire::ServiceCapability::SkillTraining {
            capability_id: label(capability_id)?,
            offered_track_ids: offered_track_ids
                .iter()
                .map(|value| label(value))
                .collect::<Result<_, _>>()?,
            selected_track_id: selected_track_id.as_deref().map(label).transpose()?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::SkillCritique {
            capability_id,
            actions,
        } => wire::ServiceCapability::SkillCritique {
            capability_id: label(capability_id)?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::SpellTeaching {
            capability_id,
            spell_ids,
            actions,
        } => wire::ServiceCapability::SpellTeaching {
            capability_id: label(capability_id)?,
            spell_ids: spell_ids
                .iter()
                .map(|value| label(value))
                .collect::<Result<_, _>>()?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::ClassPromotion {
            capability_id,
            target_class_id,
            actions,
        } => wire::ServiceCapability::ClassPromotion {
            capability_id: label(capability_id)?,
            target_class_id: label(target_class_id)?,
            actions: action_options(actions)?,
        },
        rules::ServiceCapabilityViewV1::ServiceTransaction {
            capability_id,
            transactions,
        } => wire::ServiceCapability::ServiceTransaction {
            capability_id: label(capability_id)?,
            transactions: transactions
                .iter()
                .map(service_transaction)
                .collect::<Result<_, _>>()?,
        },
        rules::ServiceCapabilityViewV1::Merchant {
            capability_id,
            listings,
            buy_all,
            sales,
        } => wire::ServiceCapability::Merchant {
            capability_id: label(capability_id)?,
            listings: listings
                .iter()
                .map(|listing| {
                    Ok(wire::MerchantListing {
                        item: owned_item(&listing.item)?,
                        origin: match listing.origin {
                            rules::MerchantListingOriginViewV1::AuthoredStock => {
                                wire::MerchantListingOrigin::AuthoredStock
                            }
                            rules::MerchantListingOriginViewV1::PawnPool => {
                                wire::MerchantListingOrigin::PawnPool
                            }
                        },
                        price_gold: wire::DecimalI64::new(listing.price_gold),
                        purchase: observer_action_option(&listing.purchase)?,
                    })
                })
                .collect::<Result<_, wire::ProtocolError>>()?,
            buy_all: observer_action_option(buy_all)?,
            sales: action_options(sales)?,
        },
        rules::ServiceCapabilityViewV1::ItemService {
            capability_id,
            operations,
        } => wire::ServiceCapability::ItemService {
            capability_id: label(capability_id)?,
            operations: operations
                .iter()
                .map(|operation| {
                    Ok(wire::ItemServiceOperation {
                        operation: item_service_operation(operation.operation),
                        actions: action_options(&operation.actions)?,
                    })
                })
                .collect::<Result<_, wire::ProtocolError>>()?,
        },
        rules::ServiceCapabilityViewV1::Restoration {
            capability_id,
            operations,
        } => wire::ServiceCapability::Restoration {
            capability_id: label(capability_id)?,
            operations: operations
                .iter()
                .map(|operation| {
                    Ok(wire::RestorationOperation {
                        operation_id: label(&operation.operation_id)?,
                        label: label(&operation.label)?,
                        requirements: operation
                            .requirements
                            .iter()
                            .map(transaction_requirement)
                            .collect::<Result<_, _>>()?,
                        costs: operation.costs.iter().map(transaction_cost).collect(),
                        outcome: restoration_outcome(&operation.outcome),
                        actions: action_options(&operation.actions)?,
                    })
                })
                .collect::<Result<_, wire::ProtocolError>>()?,
        },
        rules::ServiceCapabilityViewV1::Bank {
            capability_id,
            bank_id,
            balance_gold,
            transaction_cap_gold,
            deposit_actions,
            withdrawal_actions,
        } => wire::ServiceCapability::Bank {
            capability_id: label(capability_id)?,
            bank_id: label(bank_id)?,
            balance_gold: wire::DecimalI64::new(*balance_gold),
            transaction_cap_gold: wire::DecimalI64::new(*transaction_cap_gold),
            deposit_actions: action_options(deposit_actions)?,
            withdrawal_actions: action_options(withdrawal_actions)?,
        },
        rules::ServiceCapabilityViewV1::Locker {
            capability_id,
            vault_id,
            capacity,
            item_count,
            items,
            deposit_actions,
            withdrawal_actions,
        } => wire::ServiceCapability::Locker {
            capability_id: label(capability_id)?,
            vault_id: label(vault_id)?,
            capacity: *capacity,
            item_count: *item_count,
            items: items.iter().map(owned_item).collect::<Result<_, _>>()?,
            deposit_actions: action_options(deposit_actions)?,
            withdrawal_actions: action_options(withdrawal_actions)?,
        },
    })
}

fn service(value: &rules::ServiceViewV1) -> Result<wire::Service, wire::ProtocolError> {
    Ok(wire::Service {
        service_id: label(&value.service_id)?,
        name: label(&value.name)?,
        position: position(&value.position)?,
        capabilities: value
            .capabilities
            .iter()
            .map(service_capability)
            .collect::<Result<_, _>>()?,
    })
}

fn npc(value: &rules::NpcViewV1) -> Result<wire::Npc, wire::ProtocolError> {
    Ok(wire::Npc {
        actor_id: actor_id(&value.actor_id)?,
        name: label(&value.name)?,
        following_character_id: value
            .following_character_id
            .as_ref()
            .map(character_id)
            .transpose()?,
        interactions: value
            .interactions
            .iter()
            .map(|interaction| {
                Ok(wire::NpcInteraction {
                    interaction_id: label(&interaction.interaction_id)?,
                    label: label(&interaction.label)?,
                    requirements: interaction
                        .requirements
                        .iter()
                        .map(transaction_requirement)
                        .collect::<Result<_, _>>()?,
                    costs: interaction.costs.iter().map(transaction_cost).collect(),
                    rewards: interaction
                        .rewards
                        .iter()
                        .map(transaction_reward)
                        .collect::<Result<_, _>>()?,
                    outcome: match &interaction.outcome {
                        rules::NpcInteractionOutcome::Speak => wire::NpcInteractionOutcome::Speak,
                        rules::NpcInteractionOutcome::BeginFollow => {
                            wire::NpcInteractionOutcome::BeginFollow
                        }
                        rules::NpcInteractionOutcome::EndFollow => {
                            wire::NpcInteractionOutcome::EndFollow
                        }
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
                    },
                    actions: action_options(&interaction.actions)?,
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
    })
}

fn quest(value: &rules::QuestStateViewV1) -> Result<wire::QuestState, wire::ProtocolError> {
    Ok(wire::QuestState {
        quest_id: label(&value.quest_id)?,
        quest_title: label(&value.quest_title)?,
        stage_id: label(&value.stage_id)?,
        stage_label: label(&value.stage_label)?,
        terminal: value.terminal,
    })
}

fn loot_claim(value: &rules::LootClaimViewV1) -> Result<wire::LootClaim, wire::ProtocolError> {
    Ok(wire::LootClaim {
        owner: match &value.owner {
            rules::LootOwnerId::Character(value) => {
                wire::LootOwner::Character(character_id(value)?)
            }
            rules::LootOwnerId::TransientActor(value) => {
                wire::LootOwner::TransientActor(actor_id(value)?)
            }
        },
        basis: match value.basis {
            rules::LootClaimBasis::KillingBlow => wire::LootClaimBasis::KillingBlow,
            rules::LootClaimBasis::CharacterDeathPile => wire::LootClaimBasis::CharacterDeathPile,
        },
    })
}

pub(crate) fn rules_direction(value: &wire::Direction) -> rules::Direction {
    match value {
        wire::Direction::North => rules::Direction::North,
        wire::Direction::Northeast => rules::Direction::Northeast,
        wire::Direction::East => rules::Direction::East,
        wire::Direction::Southeast => rules::Direction::Southeast,
        wire::Direction::South => rules::Direction::South,
        wire::Direction::Southwest => rules::Direction::Southwest,
        wire::Direction::West => rules::Direction::West,
        wire::Direction::Northwest => rules::Direction::Northwest,
    }
}

fn rules_gold_position(value: wire::CarriedGoldPosition) -> rules::CarriedGoldPosition {
    match value {
        wire::CarriedGoldPosition::LeftHand => rules::CarriedGoldPosition::LeftHand,
        wire::CarriedGoldPosition::RightHand => rules::CarriedGoldPosition::RightHand,
        wire::CarriedGoldPosition::Sack => rules::CarriedGoldPosition::Sack,
    }
}

fn gold_position(value: rules::CarriedGoldPosition) -> wire::CarriedGoldPosition {
    match value {
        rules::CarriedGoldPosition::LeftHand => wire::CarriedGoldPosition::LeftHand,
        rules::CarriedGoldPosition::RightHand => wire::CarriedGoldPosition::RightHand,
        rules::CarriedGoldPosition::Sack => wire::CarriedGoldPosition::Sack,
    }
}

fn item_service_operation(
    value: rules::ItemServiceOperationKind,
) -> wire::ItemServiceOperationKind {
    match value {
        rules::ItemServiceOperationKind::Appraise => wire::ItemServiceOperationKind::Appraise,
        rules::ItemServiceOperationKind::Identify => wire::ItemServiceOperationKind::Identify,
        rules::ItemServiceOperationKind::EnchantWeapon => {
            wire::ItemServiceOperationKind::EnchantWeapon
        }
    }
}

fn rules_item_service_operation(
    value: wire::ItemServiceOperationKind,
) -> rules::ItemServiceOperationKind {
    match value {
        wire::ItemServiceOperationKind::Appraise => rules::ItemServiceOperationKind::Appraise,
        wire::ItemServiceOperationKind::Identify => rules::ItemServiceOperationKind::Identify,
        wire::ItemServiceOperationKind::EnchantWeapon => {
            rules::ItemServiceOperationKind::EnchantWeapon
        }
    }
}

fn observer_intent(
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

fn observer_action_option(
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

fn vertical(value: rules::VerticalDirection) -> wire::VerticalDirection {
    match value {
        rules::VerticalDirection::Up => wire::VerticalDirection::Up,
        rules::VerticalDirection::Down => wire::VerticalDirection::Down,
    }
}

fn navigation(value: rules::NavigationKind) -> wire::NavigationKind {
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

fn transition(value: &rules::TransitionViewV1) -> Result<wire::Transition, wire::ProtocolError> {
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
        logical_time: wire::DecimalU64::new(value.logical_time.value()),
        ready_at: wire::DecimalU64::new(value.ready_at.value()),
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
                                        .map(|time| wire::DecimalU64::new(time.value())),
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

fn invitation(
    value: &rules::ObserverGroupInvitationV2,
) -> Result<wire::GroupInvitation, wire::ProtocolError> {
    Ok(wire::GroupInvitation {
        invitation_id: wire::DecimalU64::new(value.invitation_id.value()),
        issuer_character_id: character_id(&value.issuer_character_id)?,
        target_character_id: character_id(&value.target_character_id)?,
        group_id: value.group_id.map(|id| wire::DecimalU64::new(id.value())),
        expires_at: wire::DecimalU64::new(value.expires_at.value()),
    })
}

fn item_offer(value: &rules::ItemOfferViewV1) -> Result<wire::ItemOffer, wire::ProtocolError> {
    Ok(wire::ItemOffer {
        item: owned_item(&value.item)?,
        sender_character_id: character_id(&value.sender_character_id)?,
        recipient_character_id: character_id(&value.recipient_character_id)?,
        source_position: carried_position(value.source_position),
        actions: action_options(&value.actions)?,
    })
}

fn feedback_actor(
    value: &rules::ObserverFeedbackActorV1,
) -> Result<wire::FeedbackActor, wire::ProtocolError> {
    Ok(wire::FeedbackActor {
        actor_id: actor_id(&value.actor_id)?,
        name: label(&value.name)?,
        kind: actor_kind(value.kind),
    })
}

fn feedback_wound(value: rules::WoundState) -> wire::FeedbackWoundState {
    match value {
        rules::WoundState::Unhurt => wire::FeedbackWoundState::Unhurt,
        rules::WoundState::Wounded => wire::FeedbackWoundState::Wounded,
        rules::WoundState::BadlyWounded => wire::FeedbackWoundState::BadlyWounded,
        rules::WoundState::NearDeath => wire::FeedbackWoundState::NearDeath,
        rules::WoundState::Dead => wire::FeedbackWoundState::Dead,
    }
}

fn feedback_effect_change(value: &rules::ObserverEffectChangeV1) -> wire::FeedbackEffectChange {
    match value {
        rules::ObserverEffectChangeV1::Applied { remaining_rounds } => {
            wire::FeedbackEffectChange::Applied {
                remaining_rounds: *remaining_rounds,
            }
        }
        rules::ObserverEffectChangeV1::Ticked { remaining_rounds } => {
            wire::FeedbackEffectChange::Ticked {
                remaining_rounds: *remaining_rounds,
            }
        }
        rules::ObserverEffectChangeV1::Expired => wire::FeedbackEffectChange::Expired {},
        rules::ObserverEffectChangeV1::Removed => wire::FeedbackEffectChange::Removed {},
    }
}

fn feedback_transaction_source(
    value: &rules::ObserverTransactionSourceV1,
) -> Result<wire::FeedbackTransactionSource, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverTransactionSourceV1::SkillTraining {
            service_id,
            capability_id,
            track_id,
        } => wire::FeedbackTransactionSource::SkillTraining {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            track_id: label(track_id)?,
        },
        rules::ObserverTransactionSourceV1::SpellLearning {
            service_id,
            capability_id,
            spell_id,
        } => wire::FeedbackTransactionSource::SpellLearning {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            spell_id: label(spell_id)?,
        },
        rules::ObserverTransactionSourceV1::ClassPromotion {
            service_id,
            capability_id,
            transaction_id,
            target_class_id,
        } => wire::FeedbackTransactionSource::ClassPromotion {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            transaction_id: label(transaction_id)?,
            target_class_id: label(target_class_id)?,
        },
        rules::ObserverTransactionSourceV1::ServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
        } => wire::FeedbackTransactionSource::ServiceTransaction {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            transaction_id: label(transaction_id)?,
        },
        rules::ObserverTransactionSourceV1::MerchantPurchase {
            service_id,
            capability_id,
            item_instance_ids,
        } => wire::FeedbackTransactionSource::MerchantPurchase {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_ids: item_instance_ids
                .iter()
                .map(wire::ItemInstanceId::new)
                .collect::<Result<_, _>>()?,
        },
        rules::ObserverTransactionSourceV1::MerchantSale {
            service_id,
            capability_id,
            item_instance_id,
        } => wire::FeedbackTransactionSource::MerchantSale {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::ObserverTransactionSourceV1::ItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => wire::FeedbackTransactionSource::ItemService {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            operation: item_service_operation(*operation),
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
        },
        rules::ObserverTransactionSourceV1::RestorationService {
            service_id,
            capability_id,
            operation_id,
            corpse_id,
        } => wire::FeedbackTransactionSource::RestorationService {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            operation_id: label(operation_id)?,
            corpse_id: corpse_id
                .as_ref()
                .map(|id| wire::CorpseId::new(id.as_str()))
                .transpose()?,
        },
        rules::ObserverTransactionSourceV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
        } => wire::FeedbackTransactionSource::NpcInteraction {
            npc_actor_id: actor_id(npc_actor_id)?,
            interaction_id: label(interaction_id)?,
        },
        rules::ObserverTransactionSourceV1::BankDeposit {
            service_id,
            capability_id,
            bank_id,
            gold_pile_id,
        } => wire::FeedbackTransactionSource::BankDeposit {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            bank_id: label(bank_id)?,
            gold_pile_id: label(gold_pile_id.as_str())?,
        },
        rules::ObserverTransactionSourceV1::BankWithdrawal {
            service_id,
            capability_id,
            bank_id,
            amount,
        } => wire::FeedbackTransactionSource::BankWithdrawal {
            service_id: label(service_id)?,
            capability_id: label(capability_id)?,
            bank_id: label(bank_id)?,
            amount: wire::DecimalI64::new(*amount),
        },
    })
}

fn feedback_transaction_cost(
    value: &rules::ObserverTransactionCostV1,
) -> Result<wire::FeedbackTransactionCost, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverTransactionCostV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => wire::FeedbackTransactionCost::CarriedGold {
            amount: wire::DecimalI64::new(*amount),
            position: gold_position(*position),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionCostV1::GroundGoldPile {
            gold_pile_id,
            amount,
        } => wire::FeedbackTransactionCost::GroundGoldPile {
            gold_pile_id: label(gold_pile_id.as_str())?,
            amount: wire::DecimalI64::new(*amount),
        },
        rules::ObserverTransactionCostV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
        } => wire::FeedbackTransactionCost::BankBalance {
            bank_id: label(bank_id)?,
            amount: wire::DecimalI64::new(*amount),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionCostV1::SelectedCarriedItem {
            item_instance_id,
            item_definition_id,
            consumed_quantity,
            remaining_quantity,
        } => wire::FeedbackTransactionCost::SelectedCarriedItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            consumed_quantity: *consumed_quantity,
            remaining_quantity: *remaining_quantity,
        },
        rules::ObserverTransactionCostV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            pawn_listing_price_gold,
        } => wire::FeedbackTransactionCost::MerchantItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            quantity: *quantity,
            pawn_listing_price_gold: wire::DecimalI64::new(*pawn_listing_price_gold),
        },
    })
}

fn feedback_transaction_reward(
    value: &rules::ObserverTransactionRewardV1,
) -> Result<wire::FeedbackTransactionReward, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverTransactionRewardV1::LearningRate {
            track_id,
            before,
            after,
        } => wire::FeedbackTransactionReward::LearningRate {
            track_id: label(track_id)?,
            before: wire::DecimalU64::new(*before),
            after: wire::DecimalU64::new(*after),
        },
        rules::ObserverTransactionRewardV1::Experience { amount, total_xp } => {
            wire::FeedbackTransactionReward::Experience {
                amount: *amount,
                total_xp: wire::DecimalI64::new(*total_xp),
            }
        }
        rules::ObserverTransactionRewardV1::Item {
            item_instance_id,
            item_definition_id,
            position,
            quantity,
        } => wire::FeedbackTransactionReward::Item {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            position: carried_position(*position),
            quantity: *quantity,
        },
        rules::ObserverTransactionRewardV1::Class {
            from_class_id,
            from_class_display,
            to_class_id,
            to_class_display,
        } => wire::FeedbackTransactionReward::Class {
            from_class_id: label(from_class_id)?,
            from_class_display: label(from_class_display)?,
            to_class_id: label(to_class_id)?,
            to_class_display: label(to_class_display)?,
        },
        rules::ObserverTransactionRewardV1::Spell {
            spell_id,
            learned_at_level,
        } => wire::FeedbackTransactionReward::Spell {
            spell_id: label(spell_id)?,
            learned_at_level: *learned_at_level,
        },
        rules::ObserverTransactionRewardV1::CarriedGold {
            amount,
            position,
            before,
            after,
        } => wire::FeedbackTransactionReward::CarriedGold {
            amount: wire::DecimalI64::new(*amount),
            position: gold_position(*position),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionRewardV1::BankBalance {
            bank_id,
            amount,
            before,
            after,
        } => wire::FeedbackTransactionReward::BankBalance {
            bank_id: label(bank_id)?,
            amount: wire::DecimalI64::new(*amount),
            before: wire::DecimalI64::new(*before),
            after: wire::DecimalI64::new(*after),
        },
        rules::ObserverTransactionRewardV1::GroundGoldPile {
            gold_pile_id,
            amount,
        } => wire::FeedbackTransactionReward::GroundGoldPile {
            gold_pile_id: label(gold_pile_id.as_str())?,
            amount: wire::DecimalI64::new(*amount),
        },
        rules::ObserverTransactionRewardV1::MerchantItem {
            item_instance_id,
            item_definition_id,
            quantity,
            listing_price_gold,
        } => wire::FeedbackTransactionReward::MerchantItem {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            quantity: *quantity,
            listing_price_gold: wire::DecimalI64::new(*listing_price_gold),
        },
        rules::ObserverTransactionRewardV1::ItemAppraised {
            item_instance_id,
            item_definition_id,
            unit_value_gold,
            total_value_gold,
        } => wire::FeedbackTransactionReward::ItemAppraised {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            unit_value_gold: wire::DecimalU64::new(*unit_value_gold),
            total_value_gold: wire::DecimalU64::new(*total_value_gold),
        },
        rules::ObserverTransactionRewardV1::ItemIdentified {
            item_instance_id,
            item_definition_id,
        } => wire::FeedbackTransactionReward::ItemIdentified {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
        },
        rules::ObserverTransactionRewardV1::ItemEnchanted {
            item_instance_id,
            item_definition_id,
            enchantment_instance_id,
            combat_add_rating_bonus,
            tags,
            remaining_rounds,
        } => wire::FeedbackTransactionReward::ItemEnchanted {
            item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
            item_definition_id: label(item_definition_id)?,
            enchantment_instance_id: label(enchantment_instance_id)?,
            combat_add_rating_bonus: *combat_add_rating_bonus,
            tags: tags
                .iter()
                .map(|tag| label(tag))
                .collect::<Result<_, _>>()?,
            remaining_rounds: *remaining_rounds,
        },
        rules::ObserverTransactionRewardV1::ResourceRestored {
            resource,
            before,
            after,
            maximum,
        } => wire::FeedbackTransactionReward::ResourceRestored {
            resource: resource_kind(*resource),
            before: *before,
            after: *after,
            maximum: *maximum,
        },
        rules::ObserverTransactionRewardV1::StatusCured {
            status,
            removed_count,
        } => wire::FeedbackTransactionReward::StatusCured {
            status: restoration_status(*status),
            removed_count: *removed_count,
        },
        rules::ObserverTransactionRewardV1::PriestResurrection {
            corpse_id,
            method,
            current_hp,
            current_stamina,
        } => wire::FeedbackTransactionReward::PriestResurrection {
            corpse_id: wire::CorpseId::new(corpse_id.as_str())?,
            method: feedback_resurrection_method(*method),
            current_hp: *current_hp,
            current_stamina: *current_stamina,
        },
        rules::ObserverTransactionRewardV1::NpcInteraction {
            npc_actor_id,
            interaction_id,
            outcome,
        } => wire::FeedbackTransactionReward::NpcInteraction {
            npc_actor_id: actor_id(npc_actor_id)?,
            interaction_id: label(interaction_id)?,
            outcome: npc_interaction_outcome(outcome)?,
        },
        rules::ObserverTransactionRewardV1::QuestStage {
            quest_id,
            before_stage_id,
            after_stage_id,
        } => wire::FeedbackTransactionReward::QuestStage {
            quest_id: label(quest_id)?,
            before_stage_id: before_stage_id.as_deref().map(label).transpose()?,
            after_stage_id: label(after_stage_id)?,
        },
    })
}

fn feedback_cue(
    value: &rules::ObserverFeedbackCueV1,
) -> Result<wire::FeedbackCue, wire::ProtocolError> {
    Ok(match value {
        rules::ObserverFeedbackCueV1::PhysicalCombat {
            source,
            target,
            location,
            mode,
            outcome,
        } => wire::FeedbackCue::PhysicalCombat {
            source: source.as_ref().map(feedback_actor).transpose()?,
            target: feedback_actor(target)?,
            location: location.as_ref().map(position).transpose()?,
            mode: physical_mode(*mode),
            outcome: match outcome {
                rules::ObserverPhysicalOutcomeV1::Hit {
                    damage,
                    armor_reduction,
                    wound_before,
                    wound_after,
                    target_hp,
                } => wire::FeedbackPhysicalOutcome::Hit {
                    damage: *damage,
                    armor_reduction: *armor_reduction,
                    wound_before: feedback_wound(*wound_before),
                    wound_after: feedback_wound(*wound_after),
                    target_hp: *target_hp,
                },
                rules::ObserverPhysicalOutcomeV1::Missed => {
                    wire::FeedbackPhysicalOutcome::Missed {}
                }
                rules::ObserverPhysicalOutcomeV1::Blocked => {
                    wire::FeedbackPhysicalOutcome::Blocked {}
                }
                rules::ObserverPhysicalOutcomeV1::NoSight => {
                    wire::FeedbackPhysicalOutcome::NoSight {}
                }
                rules::ObserverPhysicalOutcomeV1::NotReady {
                    current_time,
                    ready_at,
                } => wire::FeedbackPhysicalOutcome::NotReady {
                    current_time: wire::DecimalU64::new(current_time.value()),
                    ready_at: wire::DecimalU64::new(ready_at.value()),
                },
            },
        },
        rules::ObserverFeedbackCueV1::WeaponFumbled {
            actor,
            mode,
            result,
        } => wire::FeedbackCue::WeaponFumbled {
            actor: feedback_actor(actor)?,
            mode: physical_mode(*mode),
            result: match result {
                rules::WeaponFumbleResult::Dropped => wire::FeedbackWeaponFumbleResult::Dropped,
                rules::WeaponFumbleResult::BowUnnocked => {
                    wire::FeedbackWeaponFumbleResult::BowUnnocked
                }
            },
        },
        rules::ObserverFeedbackCueV1::SpellLifecycle {
            actor,
            spell_id,
            spell_name,
            state,
        } => wire::FeedbackCue::SpellLifecycle {
            actor: feedback_actor(actor)?,
            spell_id: label(spell_id)?,
            spell_name: label(spell_name)?,
            state: match state {
                rules::ObserverSpellLifecycleStateV1::Warmed {
                    warmed_at,
                    ready_at,
                } => wire::FeedbackSpellLifecycleState::Warmed {
                    warmed_at: wire::DecimalU64::new(warmed_at.value()),
                    ready_at: wire::DecimalU64::new(ready_at.value()),
                },
                rules::ObserverSpellLifecycleStateV1::Ready { ready_at } => {
                    wire::FeedbackSpellLifecycleState::Ready {
                        ready_at: wire::DecimalU64::new(ready_at.value()),
                    }
                }
                rules::ObserverSpellLifecycleStateV1::Cast {
                    mp_cost,
                    stamina_cost,
                } => wire::FeedbackSpellLifecycleState::Cast {
                    mp_cost: *mp_cost,
                    stamina_cost: *stamina_cost,
                },
                rules::ObserverSpellLifecycleStateV1::Fizzled { reason } => {
                    wire::FeedbackSpellLifecycleState::Fizzled {
                        reason: match reason {
                            rules::ObserverSpellFizzleReasonV1::Replaced => {
                                wire::FeedbackSpellFizzleReason::Replaced
                            }
                            rules::ObserverSpellFizzleReasonV1::Canceled => {
                                wire::FeedbackSpellFizzleReason::Canceled
                            }
                            rules::ObserverSpellFizzleReasonV1::Rest => {
                                wire::FeedbackSpellFizzleReason::Rest
                            }
                            rules::ObserverSpellFizzleReasonV1::HealingBalm => {
                                wire::FeedbackSpellFizzleReason::HealingBalm
                            }
                            rules::ObserverSpellFizzleReasonV1::Damage => {
                                wire::FeedbackSpellFizzleReason::Damage
                            }
                            rules::ObserverSpellFizzleReasonV1::Defeat => {
                                wire::FeedbackSpellFizzleReason::Defeat
                            }
                        },
                    }
                }
                rules::ObserverSpellLifecycleStateV1::Failed {
                    reason,
                    mp_cost,
                    stamina_cost,
                } => wire::FeedbackSpellLifecycleState::Failed {
                    reason: match reason {
                        rules::ObserverSpellFailureReasonV1::InvalidPath => {
                            wire::FeedbackSpellFailureReason::InvalidPath
                        }
                        rules::ObserverSpellFailureReasonV1::AboveSkillAttempt => {
                            wire::FeedbackSpellFailureReason::AboveSkillAttempt
                        }
                    },
                    mp_cost: *mp_cost,
                    stamina_cost: *stamina_cost,
                },
            },
        },
        rules::ObserverFeedbackCueV1::SpellImpact {
            source,
            spell_id,
            spell_name,
            target,
            location,
            outcome,
        } => wire::FeedbackCue::SpellImpact {
            source: source.as_ref().map(feedback_actor).transpose()?,
            spell_id: label(spell_id)?,
            spell_name: label(spell_name)?,
            target: feedback_actor(target)?,
            location: position(location)?,
            outcome: match outcome {
                rules::ObserverSpellImpactOutcomeV1::Damaged { damage, target_hp } => {
                    wire::FeedbackSpellImpactOutcome::Damaged {
                        damage: *damage,
                        target_hp: *target_hp,
                    }
                }
                rules::ObserverSpellImpactOutcomeV1::Healed { amount, target_hp } => {
                    wire::FeedbackSpellImpactOutcome::Healed {
                        amount: *amount,
                        target_hp: *target_hp,
                    }
                }
            },
        },
        rules::ObserverFeedbackCueV1::ActorEffect {
            actor,
            location,
            effect_id,
            effect_kind,
            change,
        } => wire::FeedbackCue::ActorEffect {
            actor: feedback_actor(actor)?,
            location: position(location)?,
            effect_id: label(effect_id)?,
            effect_kind: label(effect_kind)?,
            change: feedback_effect_change(change),
        },
        rules::ObserverFeedbackCueV1::TileEffect {
            location,
            effect_id,
            effect_kind,
            change,
        } => wire::FeedbackCue::TileEffect {
            location: position(location)?,
            effect_id: label(effect_id)?,
            effect_kind: label(effect_kind)?,
            change: feedback_effect_change(change),
        },
        rules::ObserverFeedbackCueV1::EffectDamage {
            actor,
            location,
            effect_id,
            effect_kind,
            damage,
            actor_hp,
        } => wire::FeedbackCue::EffectDamage {
            actor: feedback_actor(actor)?,
            location: position(location)?,
            effect_id: label(effect_id)?,
            effect_kind: label(effect_kind)?,
            damage: *damage,
            actor_hp: *actor_hp,
        },
        rules::ObserverFeedbackCueV1::Resource {
            actor,
            resource,
            reason,
            amount,
            current,
            maximum,
        } => wire::FeedbackCue::Resource {
            actor: feedback_actor(actor)?,
            resource: resource_kind(*resource),
            reason: match reason {
                rules::ObserverResourceReasonV1::MovementSpend => {
                    wire::FeedbackResourceReason::MovementSpend
                }
                rules::ObserverResourceReasonV1::PhysicalSpend => {
                    wire::FeedbackResourceReason::PhysicalSpend
                }
                rules::ObserverResourceReasonV1::SpellCost => {
                    wire::FeedbackResourceReason::SpellCost
                }
                rules::ObserverResourceReasonV1::Regenerated => {
                    wire::FeedbackResourceReason::Regenerated
                }
                rules::ObserverResourceReasonV1::Restored => wire::FeedbackResourceReason::Restored,
                rules::ObserverResourceReasonV1::Balm => wire::FeedbackResourceReason::Balm,
            },
            amount: *amount,
            current: *current,
            maximum: *maximum,
        },
        rules::ObserverFeedbackCueV1::Transaction {
            actor,
            source,
            costs,
            rewards,
        } => wire::FeedbackCue::Transaction {
            actor: feedback_actor(actor)?,
            source: feedback_transaction_source(source)?,
            costs: costs
                .iter()
                .map(feedback_transaction_cost)
                .collect::<Result<_, _>>()?,
            rewards: rewards
                .iter()
                .map(feedback_transaction_reward)
                .collect::<Result<_, _>>()?,
        },
        rules::ObserverFeedbackCueV1::Quest {
            quest_id,
            quest_title,
            before_stage_id,
            after_stage_id,
            after_stage_label,
            terminal,
        } => wire::FeedbackCue::Quest {
            quest_id: label(quest_id)?,
            quest_title: label(quest_title)?,
            before_stage_id: before_stage_id.as_deref().map(label).transpose()?,
            after_stage_id: label(after_stage_id)?,
            after_stage_label: label(after_stage_label)?,
            terminal: *terminal,
        },
        rules::ObserverFeedbackCueV1::NpcMessage {
            npc_actor_id,
            npc_name,
            interaction_id,
            response,
        } => wire::FeedbackCue::NpcMessage {
            npc_actor_id: actor_id(npc_actor_id)?,
            npc_name: label(npc_name)?,
            interaction_id: label(interaction_id)?,
            response: wire::FeedbackText::new(response)?,
        },
        rules::ObserverFeedbackCueV1::Defeat {
            actor,
            location,
            cause,
            credited_source,
        } => wire::FeedbackCue::Defeat {
            actor: feedback_actor(actor)?,
            location: position(location)?,
            cause: match cause {
                rules::DeathCause::Physical => wire::FeedbackDeathCause::Physical,
                rules::DeathCause::Poison => wire::FeedbackDeathCause::Poison,
                rules::DeathCause::Fire => wire::FeedbackDeathCause::Fire,
                rules::DeathCause::OtherMagic => wire::FeedbackDeathCause::OtherMagic,
                rules::DeathCause::Hazard => wire::FeedbackDeathCause::Hazard,
            },
            credited_source: credited_source.as_ref().map(feedback_actor).transpose()?,
        },
        rules::ObserverFeedbackCueV1::Corpse {
            corpse_id,
            origin,
            location,
            change,
        } => wire::FeedbackCue::Corpse {
            corpse_id: wire::CorpseId::new(corpse_id.as_str())?,
            origin: origin.as_ref().map(feedback_actor).transpose()?,
            location: position(location)?,
            change: match change {
                rules::ObserverCorpseChangeV1::Created => wire::FeedbackCorpseChange::Created {},
                rules::ObserverCorpseChangeV1::Removed { method } => {
                    wire::FeedbackCorpseChange::Removed {
                        method: feedback_resurrection_method(*method),
                    }
                }
            },
        },
        rules::ObserverFeedbackCueV1::LifeState { actor, from, to } => {
            wire::FeedbackCue::LifeState {
                actor: feedback_actor(actor)?,
                from: life_state(*from),
                to: life_state(*to),
            }
        }
        rules::ObserverFeedbackCueV1::Resurrection {
            actor,
            corpse_id,
            method,
            destination,
            current_hp,
            current_stamina,
        } => wire::FeedbackCue::Resurrection {
            actor: feedback_actor(actor)?,
            corpse_id: corpse_id
                .as_ref()
                .map(|id| wire::CorpseId::new(id.as_str()))
                .transpose()?,
            method: feedback_resurrection_method(*method),
            destination: position(destination)?,
            current_hp: *current_hp,
            current_stamina: *current_stamina,
        },
    })
}

pub fn events(
    values: &[rules::ObservedEventV1],
) -> Result<Vec<wire::ObservedEvent>, wire::ProtocolError> {
    values
        .iter()
        .map(|value| match value {
            rules::ObservedEventV1::ActorMoved {
                actor_id: moved_actor_id,
                from,
                to,
                navigation: moved_navigation,
            } => Ok(wire::ObservedEvent::ActorMoved {
                actor_id: actor_id(moved_actor_id)?,
                from: position(from)?,
                to: position(to)?,
                navigation: navigation(*moved_navigation),
            }),
            rules::ObservedEventV1::Inspected {
                location,
                tile,
                tile_move_cost,
                exits,
                nearby_actors,
                ground_items,
            } => Ok(wire::ObservedEvent::Inspected {
                location: position(location)?,
                tile: label(tile)?,
                tile_move_cost: *tile_move_cost,
                exits: exits
                    .iter()
                    .map(|exit| {
                        Ok(wire::ObserverInspectExit {
                            direction: direction(exit.direction),
                            location: position(&exit.location)?,
                            terrain: exit.terrain.as_deref().map(label).transpose()?,
                            move_cost: exit.move_cost,
                            status: match &exit.status {
                                rules::ObserverInspectExitStatusV1::Walkable => {
                                    wire::ObserverInspectExitStatus::Walkable
                                }
                                rules::ObserverInspectExitStatusV1::BlockedTerrain => {
                                    wire::ObserverInspectExitStatus::BlockedTerrain
                                }
                                rules::ObserverInspectExitStatusV1::Door { open, target } => {
                                    wire::ObserverInspectExitStatus::Door {
                                        open: *open,
                                        target: position(target)?,
                                    }
                                }
                                rules::ObserverInspectExitStatusV1::OutOfBounds => {
                                    wire::ObserverInspectExitStatus::OutOfBounds
                                }
                            },
                        })
                    })
                    .collect::<Result<_, wire::ProtocolError>>()?,
                nearby_actors: nearby_actors
                    .iter()
                    .map(|actor| {
                        Ok(wire::ObserverInspectActor {
                            direction: direction(actor.direction),
                            actor_id: actor_id(&actor.actor_id)?,
                            actor: label(&actor.actor)?,
                            kind: match actor.kind {
                                rules::ActorKind::Player => wire::ActorKind::Player,
                                rules::ActorKind::Monster => wire::ActorKind::Monster,
                                rules::ActorKind::Npc => wire::ActorKind::Npc,
                            },
                            location: position(&actor.location)?,
                            hp: actor.hp,
                        })
                    })
                    .collect::<Result<_, wire::ProtocolError>>()?,
                ground_items: ground_items
                    .iter()
                    .map(|item| {
                        Ok(wire::ObserverInspectGroundItem {
                            item: observer_item(&item.item)?,
                            location: position(&item.location)?,
                            direction: item.direction.map(direction),
                        })
                    })
                    .collect::<Result<_, wire::ProtocolError>>()?,
            }),
            rules::ObservedEventV1::GroupChanged { group_id } => {
                Ok(wire::ObservedEvent::GroupChanged {
                    group_id: wire::DecimalU64::new(group_id.value()),
                })
            }
            rules::ObservedEventV1::GroupInvitationChanged { invitation_id } => {
                Ok(wire::ObservedEvent::GroupInvitationChanged {
                    invitation_id: wire::DecimalU64::new(invitation_id.value()),
                })
            }
            rules::ObservedEventV1::GroupPresenceChanged {
                group_id,
                character_id: changed_character_id,
                connected,
            } => Ok(wire::ObservedEvent::GroupPresenceChanged {
                group_id: wire::DecimalU64::new(group_id.value()),
                character_id: character_id(changed_character_id)?,
                connected: *connected,
            }),
            rules::ObservedEventV1::PlayerFollowChanged {
                follower_character_id,
                target_character_id,
            } => Ok(wire::ObservedEvent::PlayerFollowChanged {
                follower_character_id: character_id(follower_character_id)?,
                target_character_id: target_character_id.as_ref().map(character_id).transpose()?,
            }),
            rules::ObservedEventV1::CommunicationPreferencesChanged => {
                Ok(wire::ObservedEvent::CommunicationPreferencesChanged)
            }
            rules::ObservedEventV1::ItemOfferChanged { item_instance_id } => {
                Ok(wire::ObservedEvent::ItemOfferChanged {
                    item_instance_id: wire::ItemInstanceId::new(item_instance_id)?,
                })
            }
            rules::ObservedEventV1::DefeatRewardShare {
                character_id: recipient_character_id,
                amount,
            } => Ok(wire::ObservedEvent::DefeatRewardShare {
                character_id: character_id(recipient_character_id)?,
                amount: *amount,
            }),
            rules::ObservedEventV1::Feedback { cue } => Ok(wire::ObservedEvent::Feedback {
                cue: feedback_cue(cue)?,
            }),
        })
        .collect()
}

fn path_navigation(value: rules::TransitionKindViewV1) -> wire::NavigationKind {
    match value {
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
    }
}

pub fn path_preview(
    value: &rules::PathPreviewV1,
) -> Result<wire::PathPreview, wire::ProtocolError> {
    let preview = wire::PathPreview {
        contract_version: value.contract_version,
        actor_id: actor_id(&value.actor_id)?,
        start: position(&value.start)?,
        pace: match value.pace {
            rules::MovementPace::Walk => wire::MovementPace::Walk,
            rules::MovementPace::Run => wire::MovementPace::Run,
            rules::MovementPace::Sprint => wire::MovementPace::Sprint,
        },
        requested_path: value.requested_path.iter().copied().map(direction).collect(),
        available_path_points: value.available_path_points,
        accepted_steps: wire::DecimalU64::new(value.accepted_steps as u64),
        steps: value
            .steps
            .iter()
            .map(|step| {
                Ok(wire::PathPreviewStep {
                    index: wire::DecimalU64::new(step.index as u64),
                    direction: direction(step.direction),
                    from: position(&step.from)?,
                    attempted: position(&step.attempted)?,
                    opens_door: step.opens_door,
                    terrain_name: step.terrain_name.as_deref().map(label).transpose()?,
                    cost: step.cost,
                    remaining_points_after: step.remaining_points_after,
                    outcome: match &step.outcome {
                        rules::PathPreviewStepOutcomeV1::Moved { kind } => {
                            wire::PathPreviewStepOutcome::Moved {
                                navigation: path_navigation(*kind),
                            }
                        }
                        rules::PathPreviewStepOutcomeV1::Transitioned { kind, to } => {
                            wire::PathPreviewStepOutcome::Transitioned {
                                navigation: path_navigation(*kind),
                                to: position(to)?,
                            }
                        }
                        rules::PathPreviewStepOutcomeV1::Blocked { reason } => {
                            wire::PathPreviewStepOutcome::Blocked {
                                reason: match reason {
                                    rules::PathPreviewBlockedReasonV1::SuppressedByStatus => {
                                        wire::PathPreviewBlockedReason::SuppressedByStatus
                                    }
                                    rules::PathPreviewBlockedReasonV1::OutOfBounds => {
                                        wire::PathPreviewBlockedReason::OutOfBounds
                                    }
                                    rules::PathPreviewBlockedReasonV1::BlockedTerrain => {
                                        wire::PathPreviewBlockedReason::BlockedTerrain
                                    }
                                    rules::PathPreviewBlockedReasonV1::InsufficientMovementPoints => {
                                        wire::PathPreviewBlockedReason::InsufficientMovementPoints
                                    }
                                },
                            }
                        }
                    },
                })
            })
            .collect::<Result<_, wire::ProtocolError>>()?,
        stop_reason: match value.stop_reason {
            rules::MovementStopReason::FullPathAccepted => {
                wire::MovementStopReason::FullPathAccepted
            }
            rules::MovementStopReason::Blocked => wire::MovementStopReason::Blocked,
            rules::MovementStopReason::Transitioned => wire::MovementStopReason::Transitioned,
            rules::MovementStopReason::ZeroStaminaLimit => {
                wire::MovementStopReason::ZeroStaminaLimit
            }
        },
        final_position: position(&value.final_position)?,
        remaining_path_points: value.remaining_path_points,
        burden: wire::Burden {
            item_burden: wire::DecimalU64::new(value.burden.item_burden),
            coin_burden: wire::DecimalU64::new(value.burden.coin_burden),
            total_burden: wire::DecimalU64::new(value.burden.total_burden),
            lightly_loaded_limit: value
                .burden
                .lightly_loaded_limit
                .map(wire::DecimalU64::new),
            moderately_loaded_limit: value
                .burden
                .moderately_loaded_limit
                .map(wire::DecimalU64::new),
            heavily_loaded_limit: value
                .burden
                .heavily_loaded_limit
                .map(wire::DecimalU64::new),
            tier: value.burden.tier.map(|tier| match tier {
                rules::BurdenTier::LightlyLoaded => wire::BurdenTier::LightlyLoaded,
                rules::BurdenTier::ModeratelyLoaded => wire::BurdenTier::ModeratelyLoaded,
                rules::BurdenTier::HeavilyLoaded => wire::BurdenTier::HeavilyLoaded,
                rules::BurdenTier::VeryHeavilyLoaded => wire::BurdenTier::VeryHeavilyLoaded,
            }),
        },
        movement_exertion: match value.movement_exertion {
            rules::MovementExertion::None => wire::MovementExertion::None,
            rules::MovementExertion::Normal => wire::MovementExertion::Normal,
            rules::MovementExertion::Rapid => wire::MovementExertion::Rapid,
        },
        stamina_before: value.stamina_before,
        stamina_cost: value.stamina_cost,
        stamina_after: value.stamina_after,
    };
    preview.validate()?;
    Ok(preview)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesIntent {
    Gameplay(rules::PlayerIntent),
    Social(rules::SocialIntent),
}

pub fn intent(value: &wire::Intent) -> RulesIntent {
    match value {
        wire::Intent::MovePath { path } => RulesIntent::Gameplay(rules::PlayerIntent::MovePath(
            path.iter().map(rules_direction).collect(),
        )),
        wire::Intent::Traverse { traversal } => RulesIntent::Gameplay(
            rules::PlayerIntent::Traverse(rules_explicit_traversal(*traversal)),
        ),
        wire::Intent::Open { direction } => {
            RulesIntent::Gameplay(rules::PlayerIntent::Open(rules_direction(direction)))
        }
        wire::Intent::Close { direction } => {
            RulesIntent::Gameplay(rules::PlayerIntent::Close(rules_direction(direction)))
        }
        wire::Intent::Inspect => RulesIntent::Gameplay(rules::PlayerIntent::Inspect),
        wire::Intent::Hide => RulesIntent::Gameplay(rules::PlayerIntent::Hide),
        wire::Intent::ShowSack => RulesIntent::Gameplay(rules::PlayerIntent::ShowSack),
        wire::Intent::Wait => RulesIntent::Gameplay(rules::PlayerIntent::Wait),
        wire::Intent::Rest => RulesIntent::Gameplay(rules::PlayerIntent::Rest),
        wire::Intent::PhysicalAttack {
            mode,
            target_actor_id,
            authorization,
        } => RulesIntent::Gameplay(rules::PlayerIntent::PhysicalAttack {
            mode: rules_physical_mode(*mode),
            target_actor_id: rules_actor_id(target_actor_id),
            authorization: rules_authorization(*authorization),
        }),
        wire::Intent::Nock => RulesIntent::Gameplay(rules::PlayerIntent::Nock),
        wire::Intent::UnloadBow => RulesIntent::Gameplay(rules::PlayerIntent::UnloadBow),
        wire::Intent::WarmSpell { spell_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::WarmSpell {
                spell_id: spell_id.as_str().to_string(),
            })
        }
        wire::Intent::CastSpell {
            spell_id,
            target,
            authorization,
        } => RulesIntent::Gameplay(rules::PlayerIntent::CastSpell {
            spell_id: spell_id.as_str().to_string(),
            target: target.as_ref().map(rules_spell_target),
            authorization: rules_authorization(*authorization),
        }),
        wire::Intent::CastWarmedSpell {
            target,
            authorization,
        } => RulesIntent::Gameplay(rules::PlayerIntent::CastWarmedSpell {
            target: target.as_ref().map(rules_spell_target),
            authorization: rules_authorization(*authorization),
        }),
        wire::Intent::FizzleWarmedSpell => {
            RulesIntent::Gameplay(rules::PlayerIntent::FizzleWarmedSpell)
        }
        wire::Intent::SearchCorpse { corpse_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::SearchCorpse(
                rules::CorpseId::parse(corpse_id.as_str()).expect("validated wire corpse ID"),
            ))
        }
        wire::Intent::MoveItem {
            item_instance_id,
            destination,
        } => RulesIntent::Gameplay(rules::PlayerIntent::MoveItem {
            item_instance_id: item_instance_id.as_str().to_string(),
            destination: match destination {
                wire::ItemMoveDestination::GroundHere => rules::ItemMoveDestination::GroundHere,
                wire::ItemMoveDestination::Carried { position } => {
                    rules::ItemMoveDestination::Carried {
                        position: rules_carried_position(*position),
                    }
                }
            },
        }),
        wire::Intent::MoveGold {
            source,
            destination,
            quantity,
        } => RulesIntent::Gameplay(rules::PlayerIntent::MoveGold {
            source: match source {
                wire::GoldMoveSource::Carried { position } => rules::GoldMoveSource::Carried {
                    position: rules_gold_position(*position),
                },
                wire::GoldMoveSource::Ground { gold_pile_id } => rules::GoldMoveSource::Ground {
                    gold_pile_id: rules::GoldPileId::parse(gold_pile_id.as_str())
                        .expect("validated wire gold pile ID"),
                },
            },
            destination: match destination {
                wire::GoldMoveDestination::Carried { position } => {
                    rules::GoldMoveDestination::Carried {
                        position: rules_gold_position(*position),
                    }
                }
                wire::GoldMoveDestination::GroundHere => rules::GoldMoveDestination::GroundHere,
            },
            quantity: match quantity {
                wire::GoldMoveQuantity::All => rules::GoldMoveQuantity::All,
                wire::GoldMoveQuantity::Exact { amount } => rules::GoldMoveQuantity::Exact {
                    amount: amount.get(),
                },
            },
        }),
        wire::Intent::DepositBankGold {
            service_id,
            capability_id,
            gold_pile_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::DepositBankGold {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            gold_pile_id: rules::GoldPileId::parse(gold_pile_id.as_str())
                .expect("validated wire gold pile ID"),
        }),
        wire::Intent::WithdrawBankGold {
            service_id,
            capability_id,
            amount,
        } => RulesIntent::Gameplay(rules::PlayerIntent::WithdrawBankGold {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            amount: amount.get(),
        }),
        wire::Intent::DepositLockerItem {
            service_id,
            capability_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::DepositLockerItem {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::WithdrawLockerItem {
            service_id,
            capability_id,
            item_instance_id,
            destination,
        } => RulesIntent::Gameplay(rules::PlayerIntent::WithdrawLockerItem {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_id: item_instance_id.as_str().to_string(),
            destination: rules_carried_position(*destination),
        }),
        wire::Intent::DrinkItem { item_instance_id } => RulesIntent::Gameplay(
            rules::PlayerIntent::Drink(item_instance_id.as_str().to_string()),
        ),
        wire::Intent::Train {
            service_id,
            offered_gold,
        } => RulesIntent::Gameplay(rules::PlayerIntent::Train {
            service_id: service_id.as_str().to_string(),
            offered_gold: offered_gold.get(),
        }),
        wire::Intent::Critique {
            service_id,
            track_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::Critique {
            service_id: service_id.as_str().to_string(),
            track_id: track_id.as_str().to_string(),
        }),
        wire::Intent::PromoteClass { target_class_id } => RulesIntent::Gameplay(
            rules::PlayerIntent::PromoteClass(target_class_id.as_str().to_string()),
        ),
        wire::Intent::LearnSpell { spell_id } => RulesIntent::Gameplay(
            rules::PlayerIntent::LearnSpell(spell_id.as_str().to_string()),
        ),
        wire::Intent::CommitServiceTransaction {
            service_id,
            capability_id,
            transaction_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::CommitServiceTransaction {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            transaction_id: transaction_id.as_str().to_string(),
            item_instance_id: item_instance_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
        }),
        wire::Intent::BuyFromMerchant {
            service_id,
            capability_id,
            item_instance_ids,
        } => RulesIntent::Gameplay(rules::PlayerIntent::BuyFromMerchant {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_ids: item_instance_ids
                .iter()
                .map(|value| value.as_str().to_string())
                .collect(),
        }),
        wire::Intent::SellToMerchant {
            service_id,
            capability_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::SellToMerchant {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::UseItemService {
            service_id,
            capability_id,
            operation,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::UseItemService {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            operation: rules_item_service_operation(*operation),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::UseRestorationService {
            service_id,
            capability_id,
            operation_id,
            item_instance_id,
            corpse_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::UseRestorationService {
            service_id: service_id.as_str().to_string(),
            capability_id: capability_id.as_str().to_string(),
            operation_id: operation_id.as_str().to_string(),
            item_instance_id: item_instance_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
            corpse_id: corpse_id.as_ref().map(|value| {
                rules::CorpseId::parse(value.as_str()).expect("validated wire corpse ID")
            }),
        }),
        wire::Intent::InteractWithNpc {
            npc_actor_id,
            interaction_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::InteractWithNpc {
            npc_actor_id: rules_actor_id(npc_actor_id),
            interaction_id: interaction_id.as_str().to_string(),
            item_instance_id: item_instance_id
                .as_ref()
                .map(|value| value.as_str().to_string()),
        }),
        wire::Intent::ClearSelfDefense {
            attacker_character_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::ClearSelfDefense {
            attacker_character_id: rules_character_id(*attacker_character_id),
        }),
        wire::Intent::Invite {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::Invite {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::AcceptInvite { invitation_id } => {
            RulesIntent::Social(rules::SocialIntent::AcceptInvite {
                invitation_id: rules::GroupInviteId::new(invitation_id.get()),
            })
        }
        wire::Intent::DeclineInvite { invitation_id } => {
            RulesIntent::Social(rules::SocialIntent::DeclineInvite {
                invitation_id: rules::GroupInviteId::new(invitation_id.get()),
            })
        }
        wire::Intent::CancelInvite { invitation_id } => {
            RulesIntent::Social(rules::SocialIntent::CancelInvite {
                invitation_id: rules::GroupInviteId::new(invitation_id.get()),
            })
        }
        wire::Intent::LeaveGroup => RulesIntent::Social(rules::SocialIntent::LeaveGroup),
        wire::Intent::RemoveMember {
            member_character_id,
        } => RulesIntent::Social(rules::SocialIntent::RemoveMember {
            member_character_id: rules_character_id(*member_character_id),
        }),
        wire::Intent::DisbandGroup => RulesIntent::Social(rules::SocialIntent::DisbandGroup),
        wire::Intent::TransferLeadership {
            member_character_id,
        } => RulesIntent::Social(rules::SocialIntent::TransferLeadership {
            member_character_id: rules_character_id(*member_character_id),
        }),
        wire::Intent::BeginFollow {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::BeginFollow {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::EndFollow => RulesIntent::Social(rules::SocialIntent::EndFollow),
        wire::Intent::SetPagesEnabled { enabled } => {
            RulesIntent::Social(rules::SocialIntent::SetPagesEnabled { enabled: *enabled })
        }
        wire::Intent::Block {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::Block {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::Unblock {
            target_character_id,
        } => RulesIntent::Social(rules::SocialIntent::Unblock {
            target_character_id: rules_character_id(*target_character_id),
        }),
        wire::Intent::OfferItem {
            recipient_character_id,
            item_instance_id,
        } => RulesIntent::Gameplay(rules::PlayerIntent::OfferItem {
            recipient_character_id: rules_character_id(*recipient_character_id),
            item_instance_id: item_instance_id.as_str().to_string(),
        }),
        wire::Intent::AcceptItemOffer {
            item_instance_id,
            destination,
        } => RulesIntent::Gameplay(rules::PlayerIntent::AcceptItemOffer {
            item_instance_id: item_instance_id.as_str().to_string(),
            destination: rules_carried_position(*destination),
        }),
        wire::Intent::RefuseItemOffer { item_instance_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::RefuseItemOffer {
                item_instance_id: item_instance_id.as_str().to_string(),
            })
        }
        wire::Intent::WithdrawItemOffer { item_instance_id } => {
            RulesIntent::Gameplay(rules::PlayerIntent::WithdrawItemOffer {
                item_instance_id: item_instance_id.as_str().to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(id: &str, name: &str, kind: rules::ActorKind) -> rules::ObserverFeedbackActorV1 {
        rules::ObserverFeedbackActorV1 {
            actor_id: rules::ActorId::from(id),
            name: name.to_string(),
            kind,
        }
    }

    fn location() -> rules::WorldPosition {
        rules::WorldPosition::new("realm_0", "room_0", rules::Coord { x: 1, y: 1 })
    }

    #[test]
    fn protocol_v1_feedback_conversion_preserves_safe_nulls_wide_values_and_recovery() {
        let player = actor("player", "Wayfarer", rules::ActorKind::Player);
        let target = actor("mireling", "Mireling", rules::ActorKind::Monster);
        let values = vec![
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::PhysicalCombat {
                    source: None,
                    target: target.clone(),
                    location: None,
                    mode: rules::PhysicalAttackMode::Fight,
                    outcome: rules::ObserverPhysicalOutcomeV1::Missed,
                },
            },
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::Transaction {
                    actor: player.clone(),
                    source: rules::ObserverTransactionSourceV1::BankWithdrawal {
                        service_id: "bank".to_string(),
                        capability_id: "withdraw".to_string(),
                        bank_id: "bank_1".to_string(),
                        amount: i64::MAX,
                    },
                    costs: vec![],
                    rewards: vec![rules::ObserverTransactionRewardV1::CarriedGold {
                        amount: i64::MAX,
                        position: rules::CarriedGoldPosition::Sack,
                        before: 0,
                        after: i64::MAX,
                    }],
                },
            },
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::Defeat {
                    actor: target,
                    location: location(),
                    cause: rules::DeathCause::Physical,
                    credited_source: None,
                },
            },
            rules::ObservedEventV1::Feedback {
                cue: rules::ObserverFeedbackCueV1::Resurrection {
                    actor: player,
                    corpse_id: Some(rules::CorpseId::parse("corpse:1").expect("corpse ID")),
                    method: rules::ResurrectionMethod::Gods,
                    destination: location(),
                    current_hp: 1,
                    current_stamina: 1,
                },
            },
        ];

        let converted = events(&values).expect("feedback conversion");
        let encoded = serde_json::to_string(&converted).expect("serialize converted feedback");
        assert!(encoded.contains(r#""source":null"#));
        assert!(encoded.contains(r#""credited_source":null"#));
        assert!(encoded.contains(r#""9223372036854775807""#));
        assert!(encoded.contains(r#""corpse_id":"corpse:1""#));
        assert!(!encoded.contains("character_id"));
    }

    #[test]
    fn protocol_v1_feedback_conversion_rejects_invalid_text() {
        let value = rules::ObservedEventV1::Feedback {
            cue: rules::ObserverFeedbackCueV1::NpcMessage {
                npc_actor_id: rules::ActorId::from("guide"),
                npc_name: "Guide".to_string(),
                interaction_id: "speak".to_string(),
                response: "line\nbreak".to_string(),
            },
        };
        assert!(events(&[value]).is_err());
    }
}
