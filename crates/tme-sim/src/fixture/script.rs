use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use tme_rules::{
    CarriedPosition, CorpseId, Direction, ExplicitTraversalKind, GoldMoveDestination,
    GoldMoveQuantity, GoldMoveSource, GoldPileId, HostilityAuthorization, ItemMoveDestination,
    MAX_CONTROLLED_PATH_STEPS, PhysicalAttackMode, PlayerIntent, SpellTarget,
    TransactionRequirementDef, ValidatedWorldSeed,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptValidationError {
    pub messages: Vec<String>,
}

impl ScriptValidationError {
    fn new(messages: Vec<String>) -> Self {
        Self { messages }
    }
}

impl std::fmt::Display for ScriptValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.messages.join("; "))
    }
}

impl std::error::Error for ScriptValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptCastSpellStep {
    pub spell_id: String,
    #[serde(default)]
    pub target: Option<SpellTarget>,
    pub authorization: HostilityAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptCastWarmedSpellStep {
    #[serde(default)]
    pub target: Option<SpellTarget>,
    pub authorization: HostilityAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMoveItemStep {
    pub item_instance_id: String,
    pub destination: ItemMoveDestination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMoveGoldStep {
    pub source: GoldMoveSource,
    pub destination: GoldMoveDestination,
    pub quantity: GoldMoveQuantity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptBankDepositStep {
    pub service_id: String,
    pub capability_id: String,
    pub gold_pile_id: GoldPileId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptBankWithdrawalStep {
    pub service_id: String,
    pub capability_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptLockerDepositStep {
    pub service_id: String,
    pub capability_id: String,
    pub item_instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptLockerWithdrawalStep {
    pub service_id: String,
    pub capability_id: String,
    pub item_instance_id: String,
    pub destination: CarriedPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTrainStep {
    pub service_id: String,
    pub offered_gold: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptCritiqueStep {
    pub service_id: String,
    pub track_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptServiceTransactionStep {
    pub service_id: String,
    pub capability_id: String,
    pub transaction_id: String,
    #[serde(deserialize_with = "deserialize_required_nullable_string")]
    pub item_instance_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMerchantPurchaseStep {
    pub service_id: String,
    pub capability_id: String,
    pub item_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMerchantSaleStep {
    pub service_id: String,
    pub capability_id: String,
    pub item_instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptItemServiceStep {
    pub service_id: String,
    pub capability_id: String,
    pub operation: tme_rules::ItemServiceOperationKind,
    pub item_instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRestorationServiceStep {
    pub service_id: String,
    pub capability_id: String,
    pub operation_id: String,
    #[serde(deserialize_with = "deserialize_required_nullable_string")]
    pub item_instance_id: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable_corpse_id")]
    pub corpse_id: Option<CorpseId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptNpcInteractionStep {
    pub npc_actor_id: String,
    pub interaction_id: String,
    #[serde(deserialize_with = "deserialize_required_nullable_string")]
    pub item_instance_id: Option<String>,
}

fn deserialize_required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

fn deserialize_required_nullable_corpse_id<'de, D>(
    deserializer: D,
) -> Result<Option<CorpseId>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<CorpseId>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptPhysicalAttackStep {
    pub mode: PhysicalAttackMode,
    pub target_actor_id: String,
    pub authorization: HostilityAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScriptStep {
    pub move_path: Option<Vec<Direction>>,
    pub traverse: Option<ExplicitTraversalKind>,
    pub hide: Option<bool>,
    #[serde(skip)]
    pub hide_field_present: bool,
    pub nock: Option<bool>,
    pub unload_bow: Option<bool>,
    pub physical_attack: Option<ScriptPhysicalAttackStep>,
    pub search_corpse: Option<CorpseId>,
    pub move_item: Option<ScriptMoveItemStep>,
    pub move_gold: Option<ScriptMoveGoldStep>,
    pub deposit_bank_gold: Option<ScriptBankDepositStep>,
    pub withdraw_bank_gold: Option<ScriptBankWithdrawalStep>,
    pub deposit_locker_item: Option<ScriptLockerDepositStep>,
    pub withdraw_locker_item: Option<ScriptLockerWithdrawalStep>,
    pub drink: Option<String>,
    pub open: Option<Direction>,
    pub close: Option<Direction>,
    pub show_sack: Option<bool>,
    pub wait: Option<bool>,
    pub inspect: Option<bool>,
    pub train: Option<ScriptTrainStep>,
    pub critique: Option<ScriptCritiqueStep>,
    pub promote: Option<String>,
    pub learn_spell: Option<String>,
    pub commit_service_transaction: Option<ScriptServiceTransactionStep>,
    pub buy_from_merchant: Option<ScriptMerchantPurchaseStep>,
    pub sell_to_merchant: Option<ScriptMerchantSaleStep>,
    pub use_item_service: Option<ScriptItemServiceStep>,
    pub use_restoration_service: Option<ScriptRestorationServiceStep>,
    pub interact_with_npc: Option<ScriptNpcInteractionStep>,
    pub warm_spell: Option<String>,
    pub cast_warmed_spell: Option<ScriptCastWarmedSpellStep>,
    pub fizzle_warmed_spell: Option<bool>,
    pub rest: Option<bool>,
    pub cast_spell: Option<ScriptCastSpellStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptStepRaw {
    pub move_path: Option<Vec<Direction>>,
    pub traverse: Option<ExplicitTraversalKind>,
    #[serde(default)]
    pub hide: Option<serde_json::Value>,
    pub nock: Option<bool>,
    pub unload_bow: Option<bool>,
    pub physical_attack: Option<ScriptPhysicalAttackStep>,
    pub search_corpse: Option<CorpseId>,
    pub move_item: Option<ScriptMoveItemStep>,
    pub move_gold: Option<ScriptMoveGoldStep>,
    pub deposit_bank_gold: Option<ScriptBankDepositStep>,
    pub withdraw_bank_gold: Option<ScriptBankWithdrawalStep>,
    pub deposit_locker_item: Option<ScriptLockerDepositStep>,
    pub withdraw_locker_item: Option<ScriptLockerWithdrawalStep>,
    pub drink: Option<String>,
    pub open: Option<Direction>,
    pub close: Option<Direction>,
    pub show_sack: Option<bool>,
    pub wait: Option<bool>,
    pub inspect: Option<bool>,
    pub train: Option<ScriptTrainStep>,
    pub critique: Option<ScriptCritiqueStep>,
    pub promote: Option<String>,
    pub learn_spell: Option<String>,
    pub commit_service_transaction: Option<ScriptServiceTransactionStep>,
    pub buy_from_merchant: Option<ScriptMerchantPurchaseStep>,
    pub sell_to_merchant: Option<ScriptMerchantSaleStep>,
    pub use_item_service: Option<ScriptItemServiceStep>,
    pub use_restoration_service: Option<ScriptRestorationServiceStep>,
    pub interact_with_npc: Option<ScriptNpcInteractionStep>,
    pub warm_spell: Option<String>,
    pub cast_warmed_spell: Option<ScriptCastWarmedSpellStep>,
    pub fizzle_warmed_spell: Option<bool>,
    pub rest: Option<bool>,
    pub cast_spell: Option<ScriptCastSpellStep>,
}

impl<'de> Deserialize<'de> for ScriptStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let hide_field_present = value
            .as_object()
            .is_some_and(|object| object.contains_key("hide"));
        let raw = serde_json::from_value::<ScriptStepRaw>(value).map_err(de::Error::custom)?;
        let hide = raw.hide.and_then(|value| value.as_bool());

        Ok(Self {
            move_path: raw.move_path,
            traverse: raw.traverse,
            hide,
            hide_field_present,
            nock: raw.nock,
            unload_bow: raw.unload_bow,
            physical_attack: raw.physical_attack,
            search_corpse: raw.search_corpse,
            move_item: raw.move_item,
            move_gold: raw.move_gold,
            deposit_bank_gold: raw.deposit_bank_gold,
            withdraw_bank_gold: raw.withdraw_bank_gold,
            deposit_locker_item: raw.deposit_locker_item,
            withdraw_locker_item: raw.withdraw_locker_item,
            drink: raw.drink,
            open: raw.open,
            close: raw.close,
            show_sack: raw.show_sack,
            wait: raw.wait,
            inspect: raw.inspect,
            train: raw.train,
            critique: raw.critique,
            promote: raw.promote,
            learn_spell: raw.learn_spell,
            commit_service_transaction: raw.commit_service_transaction,
            buy_from_merchant: raw.buy_from_merchant,
            sell_to_merchant: raw.sell_to_merchant,
            use_item_service: raw.use_item_service,
            use_restoration_service: raw.use_restoration_service,
            interact_with_npc: raw.interact_with_npc,
            warm_spell: raw.warm_spell,
            cast_warmed_spell: raw.cast_warmed_spell,
            fizzle_warmed_spell: raw.fizzle_warmed_spell,
            rest: raw.rest,
            cast_spell: raw.cast_spell,
        })
    }
}

impl ScriptStep {
    pub(crate) fn has_no_actions(&self) -> bool {
        self.move_path.is_none()
            && self.traverse.is_none()
            && !self.hide_field_present
            && self.nock.is_none()
            && self.unload_bow.is_none()
            && self.physical_attack.is_none()
            && self.search_corpse.is_none()
            && self.move_item.is_none()
            && self.move_gold.is_none()
            && self.deposit_bank_gold.is_none()
            && self.withdraw_bank_gold.is_none()
            && self.deposit_locker_item.is_none()
            && self.withdraw_locker_item.is_none()
            && self.drink.is_none()
            && self.open.is_none()
            && self.close.is_none()
            && self.show_sack.is_none()
            && self.wait.is_none()
            && self.inspect.is_none()
            && self.train.is_none()
            && self.critique.is_none()
            && self.promote.is_none()
            && self.learn_spell.is_none()
            && self.commit_service_transaction.is_none()
            && self.buy_from_merchant.is_none()
            && self.sell_to_merchant.is_none()
            && self.use_item_service.is_none()
            && self.use_restoration_service.is_none()
            && self.interact_with_npc.is_none()
            && self.warm_spell.is_none()
            && self.cast_warmed_spell.is_none()
            && self.fizzle_warmed_spell.is_none()
            && self.rest.is_none()
            && self.cast_spell.is_none()
    }

    pub fn to_intent(&self) -> Result<PlayerIntent, ScriptValidationError> {
        let action_count = usize::from(self.move_path.is_some())
            + usize::from(self.traverse.is_some())
            + usize::from(self.hide_field_present)
            + usize::from(self.nock.is_some())
            + usize::from(self.unload_bow.is_some())
            + usize::from(self.physical_attack.is_some())
            + usize::from(self.search_corpse.is_some())
            + usize::from(self.move_item.is_some())
            + usize::from(self.move_gold.is_some())
            + usize::from(self.deposit_bank_gold.is_some())
            + usize::from(self.withdraw_bank_gold.is_some())
            + usize::from(self.deposit_locker_item.is_some())
            + usize::from(self.withdraw_locker_item.is_some())
            + usize::from(self.drink.is_some())
            + usize::from(self.open.is_some())
            + usize::from(self.close.is_some())
            + usize::from(self.show_sack.is_some())
            + usize::from(self.wait.is_some())
            + usize::from(self.inspect.is_some())
            + usize::from(self.train.is_some())
            + usize::from(self.critique.is_some())
            + usize::from(self.promote.is_some())
            + usize::from(self.learn_spell.is_some())
            + usize::from(self.commit_service_transaction.is_some())
            + usize::from(self.buy_from_merchant.is_some())
            + usize::from(self.sell_to_merchant.is_some())
            + usize::from(self.use_item_service.is_some())
            + usize::from(self.use_restoration_service.is_some())
            + usize::from(self.interact_with_npc.is_some())
            + usize::from(self.warm_spell.is_some())
            + usize::from(self.cast_warmed_spell.is_some())
            + usize::from(self.fizzle_warmed_spell.is_some())
            + usize::from(self.rest.is_some())
            + usize::from(self.cast_spell.is_some());
        if action_count != 1 {
            return Err(ScriptValidationError::new(vec![
                if self.hide_field_present {
                    "must specify exactly one action".to_string()
                } else {
                    "must contain exactly one of move_path, traverse, hide, nock, unload_bow, wait, rest, inspect, physical_attack, search_corpse, move_item, move_gold, deposit_bank_gold, withdraw_bank_gold, deposit_locker_item, withdraw_locker_item, drink, open, close, show_sack, train, critique, promote, learn_spell, commit_service_transaction, buy_from_merchant, sell_to_merchant, use_item_service, use_restoration_service, interact_with_npc, warm_spell, cast_warmed_spell, fizzle_warmed_spell, or cast_spell"
                    .to_string()
                },
            ]));
        }
        if self.hide_field_present && self.hide != Some(true) {
            return Err(ScriptValidationError::new(vec![
                ".hide must be true".to_string(),
            ]));
        }
        if self.nock == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "nock must be true".to_string(),
            ]));
        }
        if self.unload_bow == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "unload_bow must be true".to_string(),
            ]));
        }
        if let Some(path) = &self.move_path {
            if path.is_empty() {
                return Err(ScriptValidationError::new(vec![
                    "move_path must be non-empty".to_string(),
                ]));
            }
            if path.len() > MAX_CONTROLLED_PATH_STEPS {
                return Err(ScriptValidationError::new(vec![format!(
                    "move_path must contain at most {MAX_CONTROLLED_PATH_STEPS} directions"
                )]));
            }
        }
        if let Some(physical_attack) = &self.physical_attack
            && physical_attack.target_actor_id.trim().is_empty()
        {
            return Err(ScriptValidationError::new(vec![
                "physical_attack.target_actor_id must be a non-empty string".to_string(),
            ]));
        }
        if let Some(step) = &self.move_item
            && step.item_instance_id.trim().is_empty()
        {
            return Err(ScriptValidationError::new(vec![
                "move_item.item_instance_id must be a non-empty string".to_string(),
            ]));
        }
        if let Some(ScriptMoveGoldStep {
            quantity: GoldMoveQuantity::Exact { amount },
            ..
        }) = &self.move_gold
            && *amount <= 0
        {
            return Err(ScriptValidationError::new(vec![
                "move_gold.quantity.amount must be positive".to_string(),
            ]));
        }
        if let Some(step) = &self.deposit_bank_gold {
            for (field, value) in [
                ("service_id", step.service_id.as_str()),
                ("capability_id", step.capability_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".deposit_bank_gold.{field} must be a non-empty string"
                    )]));
                }
            }
        }
        if let Some(step) = &self.withdraw_bank_gold {
            for (field, value) in [
                ("service_id", step.service_id.as_str()),
                ("capability_id", step.capability_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".withdraw_bank_gold.{field} must be a non-empty string"
                    )]));
                }
            }
            if step.amount <= 0 {
                return Err(ScriptValidationError::new(vec![
                    ".withdraw_bank_gold.amount must be positive".to_string(),
                ]));
            }
        }
        for (action, service_id, capability_id, item_instance_id) in self
            .deposit_locker_item
            .as_ref()
            .map(|step| {
                (
                    "deposit_locker_item",
                    step.service_id.as_str(),
                    step.capability_id.as_str(),
                    step.item_instance_id.as_str(),
                )
            })
            .into_iter()
            .chain(self.withdraw_locker_item.as_ref().map(|step| {
                (
                    "withdraw_locker_item",
                    step.service_id.as_str(),
                    step.capability_id.as_str(),
                    step.item_instance_id.as_str(),
                )
            }))
        {
            for (field, value) in [
                ("service_id", service_id),
                ("capability_id", capability_id),
                ("item_instance_id", item_instance_id),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".{action}.{field} must be a non-empty string"
                    )]));
                }
            }
        }
        if let Some(item) = &self.drink
            && item.trim().is_empty()
        {
            return Err(ScriptValidationError::new(vec![
                "drink must be a non-empty string".to_string(),
            ]));
        }
        if let Some(cast_spell) = &self.cast_spell
            && cast_spell.spell_id.trim().is_empty()
        {
            return Err(ScriptValidationError::new(vec![
                "cast_spell.spell_id must be a non-empty string".to_string(),
            ]));
        }
        if let Some(spell) = &self.warm_spell
            && spell.trim().is_empty()
        {
            return Err(ScriptValidationError::new(vec![
                "warm_spell must be a non-empty string".to_string(),
            ]));
        }
        if self.fizzle_warmed_spell == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "fizzle_warmed_spell must be true".to_string(),
            ]));
        }
        if self.rest == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "rest must be true".to_string(),
            ]));
        }
        if let Some(spell) = &self.learn_spell
            && spell.trim().is_empty()
        {
            return Err(ScriptValidationError::new(vec![
                "learn_spell must be a non-empty string".to_string(),
            ]));
        }
        if self.show_sack == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "show_sack must be true".to_string(),
            ]));
        }
        if self.wait == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "wait must be true".to_string(),
            ]));
        }
        if self.inspect == Some(false) {
            return Err(ScriptValidationError::new(vec![
                "inspect must be true".to_string(),
            ]));
        }
        if let Some(train) = &self.train {
            if train.service_id.trim().is_empty() {
                return Err(ScriptValidationError::new(vec![
                    ".train.service_id must be a non-empty string".to_string(),
                ]));
            }
            if train.offered_gold <= 0 {
                return Err(ScriptValidationError::new(vec![
                    ".train.offered_gold must be positive".to_string(),
                ]));
            }
        }
        if let Some(critique) = &self.critique {
            if critique.service_id.trim().is_empty() {
                return Err(ScriptValidationError::new(vec![
                    ".critique.service_id must be a non-empty string".to_string(),
                ]));
            }
            if critique.track_id.trim().is_empty() {
                return Err(ScriptValidationError::new(vec![
                    ".critique.track_id must be a non-empty string".to_string(),
                ]));
            }
        }
        if let Some(transaction) = &self.commit_service_transaction {
            for (field, value) in [
                ("service_id", transaction.service_id.as_str()),
                ("capability_id", transaction.capability_id.as_str()),
                ("transaction_id", transaction.transaction_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".commit_service_transaction.{field} must be a non-empty string"
                    )]));
                }
            }
            if transaction
                .item_instance_id
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(ScriptValidationError::new(vec![
                    ".commit_service_transaction.item_instance_id must be null or a non-empty string"
                        .to_string(),
                ]));
            }
        }
        if let Some(purchase) = &self.buy_from_merchant {
            for (field, value) in [
                ("service_id", purchase.service_id.as_str()),
                ("capability_id", purchase.capability_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".buy_from_merchant.{field} must be a non-empty string"
                    )]));
                }
            }
            if purchase.item_instance_ids.is_empty()
                || purchase
                    .item_instance_ids
                    .iter()
                    .any(|value| value.trim().is_empty())
            {
                return Err(ScriptValidationError::new(vec![
                    ".buy_from_merchant.item_instance_ids must be a non-empty list of non-empty strings"
                        .to_string(),
                ]));
            }
            let unique = purchase
                .item_instance_ids
                .iter()
                .collect::<std::collections::HashSet<_>>();
            if unique.len() != purchase.item_instance_ids.len() {
                return Err(ScriptValidationError::new(vec![
                    ".buy_from_merchant.item_instance_ids must be unique".to_string(),
                ]));
            }
        }
        if let Some(sale) = &self.sell_to_merchant {
            for (field, value) in [
                ("service_id", sale.service_id.as_str()),
                ("capability_id", sale.capability_id.as_str()),
                ("item_instance_id", sale.item_instance_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".sell_to_merchant.{field} must be a non-empty string"
                    )]));
                }
            }
        }
        if let Some(item_service) = &self.use_item_service {
            for (field, value) in [
                ("service_id", item_service.service_id.as_str()),
                ("capability_id", item_service.capability_id.as_str()),
                ("item_instance_id", item_service.item_instance_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".use_item_service.{field} must be a non-empty string"
                    )]));
                }
            }
        }
        if let Some(restoration) = &self.use_restoration_service {
            for (field, value) in [
                ("service_id", restoration.service_id.as_str()),
                ("capability_id", restoration.capability_id.as_str()),
                ("operation_id", restoration.operation_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".use_restoration_service.{field} must be a non-empty string"
                    )]));
                }
            }
            if restoration
                .item_instance_id
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(ScriptValidationError::new(vec![
                    ".use_restoration_service.item_instance_id must be null or a non-empty string"
                        .to_string(),
                ]));
            }
        }
        if let Some(interaction) = &self.interact_with_npc {
            for (field, value) in [
                ("npc_actor_id", interaction.npc_actor_id.as_str()),
                ("interaction_id", interaction.interaction_id.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(ScriptValidationError::new(vec![format!(
                        ".interact_with_npc.{field} must be a non-empty string"
                    )]));
                }
            }
            if interaction
                .item_instance_id
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(ScriptValidationError::new(vec![
                    ".interact_with_npc.item_instance_id must be null or a non-empty string"
                        .to_string(),
                ]));
            }
        }

        if let Some(path) = &self.move_path {
            Ok(PlayerIntent::MovePath(path.clone()))
        } else if let Some(kind) = self.traverse {
            Ok(PlayerIntent::Traverse(kind))
        } else if self.hide == Some(true) {
            Ok(PlayerIntent::Hide)
        } else if self.nock == Some(true) {
            Ok(PlayerIntent::Nock)
        } else if self.unload_bow == Some(true) {
            Ok(PlayerIntent::UnloadBow)
        } else if let Some(physical_attack) = &self.physical_attack {
            Ok(PlayerIntent::PhysicalAttack {
                mode: physical_attack.mode,
                target_actor_id: physical_attack.target_actor_id.clone().into(),
                authorization: physical_attack.authorization,
            })
        } else if let Some(corpse_id) = &self.search_corpse {
            Ok(PlayerIntent::SearchCorpse(corpse_id.clone()))
        } else if let Some(step) = &self.move_item {
            Ok(PlayerIntent::MoveItem {
                item_instance_id: step.item_instance_id.clone(),
                destination: step.destination.clone(),
            })
        } else if let Some(step) = &self.move_gold {
            Ok(PlayerIntent::MoveGold {
                source: step.source.clone(),
                destination: step.destination.clone(),
                quantity: step.quantity.clone(),
            })
        } else if let Some(step) = &self.deposit_bank_gold {
            Ok(PlayerIntent::DepositBankGold {
                service_id: step.service_id.clone(),
                capability_id: step.capability_id.clone(),
                gold_pile_id: step.gold_pile_id.clone(),
            })
        } else if let Some(step) = &self.withdraw_bank_gold {
            Ok(PlayerIntent::WithdrawBankGold {
                service_id: step.service_id.clone(),
                capability_id: step.capability_id.clone(),
                amount: step.amount,
            })
        } else if let Some(step) = &self.deposit_locker_item {
            Ok(PlayerIntent::DepositLockerItem {
                service_id: step.service_id.clone(),
                capability_id: step.capability_id.clone(),
                item_instance_id: step.item_instance_id.clone(),
            })
        } else if let Some(step) = &self.withdraw_locker_item {
            Ok(PlayerIntent::WithdrawLockerItem {
                service_id: step.service_id.clone(),
                capability_id: step.capability_id.clone(),
                item_instance_id: step.item_instance_id.clone(),
                destination: step.destination,
            })
        } else if let Some(item) = &self.drink {
            Ok(PlayerIntent::Drink(item.clone()))
        } else if let Some(direction) = self.open {
            Ok(PlayerIntent::Open(direction))
        } else if let Some(direction) = self.close {
            Ok(PlayerIntent::Close(direction))
        } else if self.show_sack == Some(true) {
            Ok(PlayerIntent::ShowSack)
        } else if self.wait == Some(true) {
            Ok(PlayerIntent::Wait)
        } else if self.inspect == Some(true) {
            Ok(PlayerIntent::Inspect)
        } else if let Some(train) = &self.train {
            Ok(PlayerIntent::Train {
                service_id: train.service_id.clone(),
                offered_gold: train.offered_gold,
            })
        } else if let Some(critique) = &self.critique {
            Ok(PlayerIntent::Critique {
                service_id: critique.service_id.clone(),
                track_id: critique.track_id.clone(),
            })
        } else if let Some(target) = &self.promote {
            Ok(PlayerIntent::PromoteClass(target.clone()))
        } else if let Some(spell) = &self.learn_spell {
            Ok(PlayerIntent::LearnSpell(spell.clone()))
        } else if let Some(transaction) = &self.commit_service_transaction {
            Ok(PlayerIntent::CommitServiceTransaction {
                service_id: transaction.service_id.clone(),
                capability_id: transaction.capability_id.clone(),
                transaction_id: transaction.transaction_id.clone(),
                item_instance_id: transaction.item_instance_id.clone(),
            })
        } else if let Some(purchase) = &self.buy_from_merchant {
            Ok(PlayerIntent::BuyFromMerchant {
                service_id: purchase.service_id.clone(),
                capability_id: purchase.capability_id.clone(),
                item_instance_ids: purchase.item_instance_ids.clone(),
            })
        } else if let Some(sale) = &self.sell_to_merchant {
            Ok(PlayerIntent::SellToMerchant {
                service_id: sale.service_id.clone(),
                capability_id: sale.capability_id.clone(),
                item_instance_id: sale.item_instance_id.clone(),
            })
        } else if let Some(item_service) = &self.use_item_service {
            Ok(PlayerIntent::UseItemService {
                service_id: item_service.service_id.clone(),
                capability_id: item_service.capability_id.clone(),
                operation: item_service.operation,
                item_instance_id: item_service.item_instance_id.clone(),
            })
        } else if let Some(restoration) = &self.use_restoration_service {
            Ok(PlayerIntent::UseRestorationService {
                service_id: restoration.service_id.clone(),
                capability_id: restoration.capability_id.clone(),
                operation_id: restoration.operation_id.clone(),
                item_instance_id: restoration.item_instance_id.clone(),
                corpse_id: restoration.corpse_id.clone(),
            })
        } else if let Some(interaction) = &self.interact_with_npc {
            Ok(PlayerIntent::InteractWithNpc {
                npc_actor_id: interaction.npc_actor_id.clone().into(),
                interaction_id: interaction.interaction_id.clone(),
                item_instance_id: interaction.item_instance_id.clone(),
            })
        } else if let Some(spell) = &self.warm_spell {
            Ok(PlayerIntent::WarmSpell {
                spell_id: spell.clone(),
            })
        } else if let Some(cast_warmed_spell) = &self.cast_warmed_spell {
            Ok(PlayerIntent::CastWarmedSpell {
                target: cast_warmed_spell.target.clone(),
                authorization: cast_warmed_spell.authorization,
            })
        } else if self.fizzle_warmed_spell == Some(true) {
            Ok(PlayerIntent::FizzleWarmedSpell)
        } else if self.rest == Some(true) {
            Ok(PlayerIntent::Rest)
        } else if let Some(cast_spell) = &self.cast_spell {
            Ok(PlayerIntent::CastSpell {
                spell_id: cast_spell.spell_id.clone(),
                target: cast_spell.target.clone(),
                authorization: cast_spell.authorization,
            })
        } else {
            Err(ScriptValidationError::new(vec![
                "must specify exactly one action".to_string(),
            ]))
        }
    }
}

pub(crate) fn validate_script_shape(script: &[ScriptStep]) -> Vec<(String, String)> {
    const SCRIPT_ACTION_ERROR: &str = "must contain exactly one of move_path, traverse, hide, nock, unload_bow, wait, rest, inspect, physical_attack, search_corpse, move_item, move_gold, deposit_bank_gold, withdraw_bank_gold, deposit_locker_item, withdraw_locker_item, drink, open, close, show_sack, train, critique, promote, learn_spell, commit_service_transaction, buy_from_merchant, sell_to_merchant, use_item_service, use_restoration_service, interact_with_npc, warm_spell, cast_warmed_spell, fizzle_warmed_spell, or cast_spell";

    let mut diagnostics = Vec::new();
    for (index, step) in script.iter().enumerate() {
        let Err(error) = step.to_intent() else {
            continue;
        };
        for message in error.messages {
            let message = if message == "must specify exactly one action"
                || (message == SCRIPT_ACTION_ERROR && step.has_no_actions())
            {
                message
            } else if let Some(suffix) = message.strip_prefix('.') {
                suffix.to_string()
            } else {
                message
            };
            diagnostics.push((format!("/script/{index}"), message));
        }
    }
    diagnostics
}

pub(crate) fn validate_script_references(
    script: &[ScriptStep],
    world_seed: &ValidatedWorldSeed,
) -> Vec<(String, String)> {
    let seed = world_seed.seed();
    let mut diagnostics = Vec::new();
    for (index, step) in script.iter().enumerate() {
        let Some(script_interaction) = step.interact_with_npc.as_ref() else {
            continue;
        };
        let pointer = format!("/script/{index}/interact_with_npc");
        let Some(actor) = seed
            .actors
            .iter()
            .find(|actor| actor.id == script_interaction.npc_actor_id && actor.npc.is_some())
        else {
            diagnostics.push((
                format!("{pointer}/npc_actor_id"),
                format!(
                    "references unknown NPC {:?}",
                    script_interaction.npc_actor_id
                ),
            ));
            continue;
        };
        let Some(interaction) = actor.npc.as_ref().and_then(|npc| {
            npc.interactions
                .iter()
                .find(|interaction| interaction.transaction.id == script_interaction.interaction_id)
        }) else {
            diagnostics.push((
                format!("{pointer}/interaction_id"),
                format!(
                    "references unknown interaction {:?}",
                    script_interaction.interaction_id
                ),
            ));
            continue;
        };
        let required_item = interaction
            .transaction
            .requirements
            .iter()
            .find_map(|requirement| match requirement {
                TransactionRequirementDef::CarriedItem {
                    item_definition_id,
                    quantity,
                } => Some((item_definition_id.as_str(), *quantity)),
                _ => None,
            });
        match (
            required_item,
            script_interaction.item_instance_id.as_deref(),
        ) {
            (None, Some(_)) => diagnostics.push((
                format!("{pointer}/item_instance_id"),
                "must be null when the interaction has no carried_item requirement".to_string(),
            )),
            (Some(_), None) => diagnostics.push((
                format!("{pointer}/item_instance_id"),
                "must select the required carried item".to_string(),
            )),
            (Some((definition_id, quantity)), Some(instance_id)) => {
                match seed.item_instances.get(instance_id) {
                    Some(instance)
                        if instance.definition_id == definition_id
                            && instance.quantity >= quantity => {}
                    Some(_) => diagnostics.push((
                        format!("{pointer}/item_instance_id"),
                        "does not match the interaction carried_item requirement".to_string(),
                    )),
                    None => diagnostics.push((
                        format!("{pointer}/item_instance_id"),
                        format!("references unknown item instance {instance_id:?}"),
                    )),
                }
            }
            (None, None) => {}
        }
    }
    diagnostics
}

pub(crate) struct ScriptedIntentSource {
    script: std::vec::IntoIter<ScriptStep>,
}

impl ScriptedIntentSource {
    pub(crate) fn new(script: Vec<ScriptStep>) -> Self {
        Self {
            script: script.into_iter(),
        }
    }
}

impl crate::session::IntentSource for ScriptedIntentSource {
    fn next_intent<W: std::io::Write>(
        &mut self,
        _engine: &tme_rules::Engine,
        _transcript: &mut crate::session::TranscriptWriter<W>,
    ) -> Result<crate::session::IntentAction, String> {
        match self.script.next() {
            Some(step) => step
                .to_intent()
                .map(crate::session::IntentAction::Step)
                .map_err(|error| error.to_string()),
            None => Ok(crate::session::IntentAction::Stop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn step(value: serde_json::Value) -> Result<PlayerIntent, String> {
        serde_json::from_value::<ScriptStep>(value)
            .map_err(|error| error.to_string())?
            .to_intent()
            .map_err(|error| error.to_string())
    }

    #[test]
    fn current_script_action_shapes_deserialize_to_typed_intents() {
        let actions = [
            json!({"move_path": ["east", "north"]}),
            json!({"traverse": "stairs_up"}),
            json!({"hide": true}),
            json!({"nock": true}),
            json!({"unload_bow": true}),
            json!({"wait": true}),
            json!({"rest": true}),
            json!({"inspect": true}),
            json!({"physical_attack": {"mode": "fight", "target_actor_id": "target", "authorization": "safe"}}),
            json!({"search_corpse": "corpse:1"}),
            json!({"move_item": {"item_instance_id": "item", "destination": {"kind": "carried", "position": "sack_item_1"}}}),
            json!({"move_gold": {"source": {"kind": "carried", "position": "sack"}, "destination": {"kind": "carried", "position": "left_hand"}, "quantity": {"kind": "exact", "amount": 1}}}),
            json!({"deposit_bank_gold": {"service_id": "bank", "capability_id": "access", "gold_pile_id": "gold:1"}}),
            json!({"withdraw_bank_gold": {"service_id": "bank", "capability_id": "access", "amount": 1}}),
            json!({"deposit_locker_item": {"service_id": "locker", "capability_id": "access", "item_instance_id": "item"}}),
            json!({"withdraw_locker_item": {"service_id": "locker", "capability_id": "access", "item_instance_id": "item", "destination": "sack_item_1"}}),
            json!({"drink": "potion"}),
            json!({"open": "east"}),
            json!({"close": "west"}),
            json!({"show_sack": true}),
            json!({"learn_spell": "spark"}),
            json!({"promote": "knight"}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "spark", "target": {"actor": {"actor_id": "target"}}}}),
            json!({"warm_spell": "spark"}),
            json!({"cast_warmed_spell": {"authorization": "safe", "target": null}}),
            json!({"fizzle_warmed_spell": true}),
            json!({"train": {"service_id": "mentor", "offered_gold": 1}}),
            json!({"critique": {"service_id": "mentor", "track_id": "mace"}}),
            json!({"interact_with_npc": {"npc_actor_id": "guide", "interaction_id": "speak", "item_instance_id": null}}),
            json!({"commit_service_transaction": {"service_id": "desk", "capability_id": "work", "transaction_id": "job", "item_instance_id": null}}),
            json!({"buy_from_merchant": {"service_id": "shop", "capability_id": "stock", "item_instance_ids": ["one", "two"]}}),
            json!({"sell_to_merchant": {"service_id": "shop", "capability_id": "stock", "item_instance_id": "one"}}),
            json!({"use_item_service": {"service_id": "shop", "capability_id": "care", "operation": "appraise", "item_instance_id": "one"}}),
            json!({"use_restoration_service": {"service_id": "shrine", "capability_id": "restore", "operation_id": "cure", "item_instance_id": null, "corpse_id": null}}),
        ];
        for action in actions {
            assert!(
                step(action.clone()).is_ok(),
                "action must validate: {action}"
            );
        }
    }

    #[test]
    fn script_actions_reject_empty_mixed_false_and_removed_shapes() {
        let too_long = vec!["east"; MAX_CONTROLLED_PATH_STEPS + 1];
        let invalid = [
            json!({}),
            json!({"wait": false}),
            json!({"inspect": false}),
            json!({"hide": false}),
            json!({"hide": null}),
            json!({"rest": false}),
            json!({"show_sack": false}),
            json!({"move": "east"}),
            json!({"wait": true, "inspect": true}),
            json!({"move_path": []}),
            json!({"move_path": too_long}),
            json!({"physical_attack": {"mode": "fight", "target_actor_id": "", "authorization": "safe"}}),
            json!({"move_item": {"item_instance_id": "", "destination": {"kind": "ground_here"}}}),
            json!({"move_gold": {"source": {"kind": "carried", "position": "sack"}, "destination": {"kind": "carried", "position": "left_hand"}, "quantity": {"kind": "exact", "amount": 0}}}),
            json!({"deposit_bank_gold": {"service_id": "", "capability_id": "access", "gold_pile_id": "gold:1"}}),
            json!({"withdraw_bank_gold": {"service_id": "bank", "capability_id": "access", "amount": 0}}),
            json!({"deposit_locker_item": {"service_id": "locker", "capability_id": "", "item_instance_id": "item"}}),
            json!({"withdraw_locker_item": {"service_id": "locker", "capability_id": "access", "item_instance_id": "", "destination": "sack_item_1"}}),
            json!({"train": {"service_id": "mentor", "offered_gold": 0}}),
            json!({"critique": {"service_id": "", "track_id": "mace"}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "", "target": null}}),
            json!({"learn_spell": ""}),
            json!({"buy_from_merchant": {"service_id": "shop", "capability_id": "stock", "item_instance_ids": []}}),
            json!({"buy_from_merchant": {"service_id": "shop", "capability_id": "stock", "item_instance_ids": ["one", "one"]}}),
            json!({"sell_to_merchant": {"service_id": "shop", "capability_id": "stock", "item_instance_id": ""}}),
            json!({"use_item_service": {"service_id": "shop", "capability_id": "", "operation": "appraise", "item_instance_id": "one"}}),
            json!({"use_restoration_service": {"service_id": "shrine", "capability_id": "restore", "operation_id": "", "item_instance_id": null, "corpse_id": null}}),
            json!({"attack": "target"}),
        ];
        for action in invalid {
            assert!(step(action.clone()).is_err(), "action must fail: {action}");
        }
    }

    #[test]
    fn nullable_script_selections_are_required_and_strict() {
        for action in [
            json!({"interact_with_npc": {"npc_actor_id": "guide", "interaction_id": "speak"}}),
            json!({"commit_service_transaction": {"service_id": "desk", "capability_id": "work", "transaction_id": "job"}}),
            json!({"use_restoration_service": {"service_id": "shrine", "capability_id": "restore", "operation_id": "cure", "corpse_id": null}}),
            json!({"use_restoration_service": {"service_id": "shrine", "capability_id": "restore", "operation_id": "cure", "item_instance_id": null}}),
        ] {
            assert!(step(action.clone()).is_err(), "action must fail: {action}");
        }
    }

    #[test]
    fn legacy_python_script_contract_matrix_is_preserved_in_sim() {
        // This is the deletion-gate crosswalk for the script-shaped cases that
        // previously lived in the Python gameplay validator.  Keep the rows
        // literal: the coverage ledger names this test for those retired cases.
        let accepted = [
            json!({"search_corpse": "corpse:1"}),
            json!({"move_path": ["east", "southeast", "south"]}),
            json!({"traverse": "stairs_down"}),
            json!({"physical_attack": {"mode": "fight", "target_actor_id": "target", "authorization": "safe"}}),
            json!({"move_item": {"item_instance_id": "item", "destination": {"kind": "carried", "position": "right_hand"}}}),
            json!({"move_gold": {"source": {"kind": "ground", "gold_pile_id": "gold:1"}, "destination": {"kind": "carried", "position": "sack"}, "quantity": {"kind": "all"}}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "wind", "target": {"direction": {"direction": "north"}}}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "ward", "target": {"door": {"direction": "east"}}}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "key", "target": {"item": {"item_instance_id": "iron_key", "location": "sack"}}}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "stillness", "target": "none"}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "mend", "target": "self_target"}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "step", "target": {"coordinate": {"position": {"realm": "realm_0", "level": "room_0", "position": {"x": 2, "y": 1}}}}}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "field", "target": {"area": {"center": {"realm": "realm_0", "level": "room_0", "position": {"x": 2, "y": 1}}}}}}),
            json!({"cast_spell": {"authorization": "safe", "spell_id": "trail", "target": {"path": {"directions": ["east", "north"]}}}}),
            json!({"train": {"service_id": "trainer", "offered_gold": 40}}),
            json!({"critique": {"service_id": "trainer", "track_id": "sword"}}),
        ];
        for action in accepted {
            assert!(
                step(action.clone()).is_ok(),
                "legacy accepted action must remain valid: {action}"
            );
        }

        let invalid = [
            json!({"search_corpse": null}),
            json!({"search_corpse": ""}),
            json!({"search_corpse": "corpse"}),
            json!({"search_corpse": "corpse:0"}),
            json!({"search_corpse": "corpse:01"}),
            json!({"search_corpse": "corpse:-1"}),
            json!({"search_corpse": "corpse:1", "wait": true}),
            json!({"search": "corpse:1"}),
            json!({"move_path": ["east", "sideways"]}),
            json!({"move_path": ["east", []]}),
            json!({"move_path": ["east"], "wait": true}),
            json!({"traverse": "sideways"}),
            json!({"traverse": "stairs_down", "move_path": ["east"]}),
            json!({"physical_attack": {"mode": "future", "target_actor_id": "target", "authorization": "safe"}}),
            json!({"physical_attack": {"mode": "fight", "target_actor_id": "target"}}),
            json!({"physical_attack": {"mode": "fight", "target_actor_id": "target", "force": 1}}),
            json!({"physical_attack": {"mode": "fight", "target_actor_id": "target", "authorization": "safe"}, "wait": true}),
            json!({"move_item": {"item_instance_id": "item", "destination": {"kind": "carried", "position": "pack"}}}),
            json!({"move_item": {"item_instance_id": "item"}}),
            json!({"move_item": {"item_instance_id": "item", "destination": {"kind": "ground_here"}}, "wait": true}),
            json!({"move_gold": {"source": {"kind": "carried", "position": "belt"}, "destination": {"kind": "ground"}, "quantity": {"kind": "exact", "amount": 0}, "legacy": true}}),
            json!({"deposit_bank_gold": {"service_id": "bank", "capability_id": "access", "gold_pile_id": "gold:01"}}),
            json!({"withdraw_bank_gold": {"service_id": "bank", "capability_id": "", "amount": true}}),
            json!({"deposit_locker_item": {"service_id": "locker", "capability_id": "access", "item_instance_id": ""}}),
            json!({"withdraw_locker_item": {"service_id": "locker", "capability_id": "access", "item_instance_id": "item", "destination": "pack"}}),
            json!({"drink": "  "}),
            json!({"drink": "balm", "wait": true}),
            json!({"learn_spell": "spark", "wait": true}),
            json!({"take": "item"}),
            json!({"retrieve": "item"}),
            json!({"drop": "item"}),
            json!({"equip": "item"}),
            json!({"unequip": "hands"}),
            json!({"buy_from_merchant": {"service_id": "", "capability_id": "stock", "item_instance_ids": ["one", "one"]}}),
            json!({"sell_to_merchant": {"service_id": "shop", "capability_id": "", "item_instance_id": ""}}),
            json!({"use_item_service": {"service_id": "shop", "capability_id": "care", "operation": "repair", "item_instance_id": "one"}}),
            json!({"buy_from_merchant": {"service_id": "shop", "capability_id": "stock", "item_instance_ids": []}, "wait": true}),
            json!({"use_restoration_service": {"service_id": "", "capability_id": "restore", "operation_id": "", "item_instance_id": null}}),
            json!({"use_restoration_service": {"service_id": "shrine", "capability_id": "", "operation_id": "cure", "item_instance_id": 1, "corpse_id": "", "legacy": true}}),
            json!({"interact_with_npc": {"interaction_id": "speak", "item_instance_id": null}}),
            json!({"interact_with_npc": {"npc_actor_id": "guide", "item_instance_id": null}}),
            json!({"interact_with_npc": {"npc_actor_id": "guide", "interaction_id": "speak"}}),
            json!({"interact_with_npc": {"npc_actor_id": "guide", "interaction_id": "speak", "item_instance_id": null, "legacy": true}}),
            json!({"train": "sword"}),
            json!({"train": {"service_id": "", "offered_gold": 0, "track_id": "sword"}}),
            json!({"critique": {"service_id": "", "track_id": "", "gold": 1}}),
            json!({"train": {"service_id": "trainer", "offered_gold": 9_223_372_036_854_775_808_u64}, "wait": true}),
        ];
        for action in invalid {
            assert!(
                step(action.clone()).is_err(),
                "legacy rejected action must remain invalid: {action}"
            );
        }
    }
}
