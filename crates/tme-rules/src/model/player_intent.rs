use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerIntent {
    MovePath(Vec<Direction>),
    Traverse(ExplicitTraversalKind),
    Hide,
    Nock,
    UnloadBow,
    PhysicalAttack {
        mode: PhysicalAttackMode,
        target_actor_id: ActorId,
        authorization: HostilityAuthorization,
    },
    SearchCorpse(CorpseId),
    MoveItem {
        item_instance_id: String,
        destination: ItemMoveDestination,
    },
    MoveGold {
        source: GoldMoveSource,
        destination: GoldMoveDestination,
        quantity: GoldMoveQuantity,
    },
    DepositBankGold {
        service_id: String,
        capability_id: String,
        gold_pile_id: GoldPileId,
    },
    WithdrawBankGold {
        service_id: String,
        capability_id: String,
        amount: i64,
    },
    DepositLockerItem {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
    },
    WithdrawLockerItem {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
        destination: CarriedPosition,
    },
    OfferItem {
        recipient_character_id: CharacterId,
        item_instance_id: String,
    },
    AcceptItemOffer {
        item_instance_id: String,
        destination: CarriedPosition,
    },
    RefuseItemOffer {
        item_instance_id: String,
    },
    WithdrawItemOffer {
        item_instance_id: String,
    },
    Drink(String),
    Open(Direction),
    Close(Direction),
    ShowSack,
    Wait,
    Inspect,
    Train {
        service_id: String,
        offered_gold: i64,
    },
    Critique {
        service_id: String,
        track_id: String,
    },
    PromoteClass(String),
    LearnSpell(String),
    CommitServiceTransaction {
        service_id: String,
        capability_id: String,
        transaction_id: String,
        item_instance_id: Option<String>,
    },
    BuyFromMerchant {
        service_id: String,
        capability_id: String,
        item_instance_ids: Vec<String>,
    },
    SellToMerchant {
        service_id: String,
        capability_id: String,
        item_instance_id: String,
    },
    UseItemService {
        service_id: String,
        capability_id: String,
        operation: ItemServiceOperationKind,
        item_instance_id: String,
    },
    UseRestorationService {
        service_id: String,
        capability_id: String,
        operation_id: String,
        item_instance_id: Option<String>,
        corpse_id: Option<CorpseId>,
    },
    InteractWithNpc {
        npc_actor_id: ActorId,
        interaction_id: String,
        item_instance_id: Option<String>,
    },
    CastSpell {
        spell_id: String,
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    WarmSpell {
        spell_id: String,
    },
    CastWarmedSpell {
        target: Option<SpellTarget>,
        authorization: HostilityAuthorization,
    },
    ClearSelfDefense {
        attacker_character_id: CharacterId,
    },
    FizzleWarmedSpell,
    Rest,
}

impl PlayerIntent {
    /// Returns the sole movement path, or `None` for non-movement intents.
    pub fn movement_path(&self) -> Option<Vec<Direction>> {
        match self {
            Self::MovePath(p) => Some(p.clone()),
            _ => None,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::MovePath(path) => {
                let pace = MovementPace::from_step_count(path.len())
                    .map(MovementPace::label)
                    .unwrap_or("invalid_path");
                format!(
                    "{pace} {}",
                    path.iter()
                        .map(|direction| direction.label())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
            Self::Traverse(kind) => format!("traverse {}", kind.label()),
            Self::Hide => "hide".to_string(),
            Self::Nock => "nock".to_string(),
            Self::UnloadBow => "unload_bow".to_string(),
            Self::PhysicalAttack {
                mode,
                target_actor_id,
                authorization,
            } => format!(
                "{}{} {target_actor_id}",
                mode.label(),
                if matches!(authorization, HostilityAuthorization::ConfirmedUnsafe) {
                    " --unsafe"
                } else {
                    ""
                }
            ),
            Self::SearchCorpse(corpse_id) => format!("search corpse {corpse_id}"),
            Self::MoveItem {
                item_instance_id,
                destination,
            } => match destination {
                ItemMoveDestination::GroundHere => {
                    format!("move_item {item_instance_id} to ground_here")
                }
                ItemMoveDestination::Carried { position } => {
                    format!("move_item {item_instance_id} to {}", position.label())
                }
            },
            Self::MoveGold {
                source,
                destination,
                quantity,
            } => format!("move_gold {source:?} {destination:?} {quantity:?}"),
            Self::DepositBankGold {
                service_id,
                capability_id,
                gold_pile_id,
            } => format!("deposit_bank_gold {service_id} {capability_id} {gold_pile_id}"),
            Self::WithdrawBankGold {
                service_id,
                capability_id,
                amount,
            } => format!("withdraw_bank_gold {service_id} {capability_id} {amount}"),
            Self::DepositLockerItem {
                service_id,
                capability_id,
                item_instance_id,
            } => format!("deposit_locker_item {service_id} {capability_id} {item_instance_id}"),
            Self::WithdrawLockerItem {
                service_id,
                capability_id,
                item_instance_id,
                destination,
            } => format!(
                "withdraw_locker_item {service_id} {capability_id} {item_instance_id} {}",
                destination.label()
            ),
            Self::OfferItem {
                recipient_character_id,
                item_instance_id,
            } => format!(
                "offer_item {item_instance_id} to {}",
                recipient_character_id.as_str()
            ),
            Self::AcceptItemOffer {
                item_instance_id,
                destination,
            } => format!(
                "accept_item_offer {item_instance_id} to {}",
                destination.label()
            ),
            Self::RefuseItemOffer { item_instance_id } => {
                format!("refuse_item_offer {item_instance_id}")
            }
            Self::WithdrawItemOffer { item_instance_id } => {
                format!("withdraw_item_offer {item_instance_id}")
            }
            Self::Drink(item) => format!("drink {item}"),
            Self::Open(direction) => format!("open {}", direction.label()),
            Self::Close(direction) => format!("close {}", direction.label()),
            Self::ShowSack => "show_sack".to_string(),
            Self::Wait => "wait".to_string(),
            Self::Train {
                service_id,
                offered_gold,
            } => format!("train {service_id} {offered_gold}"),
            Self::Critique {
                service_id,
                track_id,
            } => format!("critique {service_id} {track_id}"),
            Self::PromoteClass(target) => format!("promote {target}"),
            Self::LearnSpell(spell_id) => format!("learn_spell {spell_id}"),
            Self::CommitServiceTransaction {
                service_id,
                capability_id,
                transaction_id,
                item_instance_id,
            } => format!(
                "commit_service_transaction {service_id} {capability_id} {transaction_id} {}",
                item_instance_id.as_deref().unwrap_or("none")
            ),
            Self::BuyFromMerchant {
                service_id,
                capability_id,
                item_instance_ids,
            } => format!(
                "buy_from_merchant {service_id} {capability_id} {}",
                item_instance_ids.join(",")
            ),
            Self::SellToMerchant {
                service_id,
                capability_id,
                item_instance_id,
            } => format!("sell_to_merchant {service_id} {capability_id} {item_instance_id}"),
            Self::UseItemService {
                service_id,
                capability_id,
                operation,
                item_instance_id,
            } => format!(
                "use_item_service {service_id} {capability_id} {} {item_instance_id}",
                operation.label()
            ),
            Self::UseRestorationService {
                service_id,
                capability_id,
                operation_id,
                item_instance_id,
                corpse_id,
            } => format!(
                "use_restoration_service {service_id} {capability_id} {operation_id} {} {}",
                item_instance_id.as_deref().unwrap_or("none"),
                corpse_id
                    .as_ref()
                    .map_or("none", |corpse_id| corpse_id.as_str())
            ),
            Self::InteractWithNpc {
                npc_actor_id,
                interaction_id,
                item_instance_id,
            } => format!(
                "interact_with_npc {npc_actor_id} {interaction_id} {}",
                item_instance_id.as_deref().unwrap_or("none")
            ),
            Self::Inspect => "inspect".to_string(),
            Self::CastSpell {
                spell_id,
                target,
                authorization,
            } => {
                let mut label = if let Some(target) = target {
                    format!("cast {} on {}", spell_id, target.label())
                } else {
                    format!("cast {}", spell_id)
                };
                if matches!(authorization, HostilityAuthorization::ConfirmedUnsafe) {
                    label.push_str(" --unsafe");
                }
                label
            }
            Self::WarmSpell { spell_id } => format!("warm_spell {}", spell_id),
            Self::CastWarmedSpell {
                target,
                authorization,
            } => {
                let mut label = if let Some(target) = target {
                    format!("cast_warmed_spell on {}", target.label())
                } else {
                    "cast_warmed_spell".to_string()
                };
                if matches!(authorization, HostilityAuthorization::ConfirmedUnsafe) {
                    label.push_str(" --unsafe");
                }
                label
            }
            Self::ClearSelfDefense {
                attacker_character_id,
            } => format!("clear_self_defense {}", attacker_character_id.as_str()),
            Self::FizzleWarmedSpell => "fizzle_warmed_spell".to_string(),
            Self::Rest => "rest".to_string(),
        }
    }
}
