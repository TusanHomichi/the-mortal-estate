use std::collections::BTreeMap;

use serde::de::{self, Deserializer};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

const HIDE_BREAK_TRIGGERS: &[&str] = &["move", "attack", "active_item_move", "cast", "warm"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfessionActionDef {
    pub id: String,
    pub kind: String,
    pub class_ids: Vec<String>,
    pub hide: Option<HideActionDef>,
    pub martial_hand_block: Option<MartialHandBlockDef>,
    pub hide_field_present: bool,
    pub martial_hand_block_field_present: bool,
    hide_raw: Option<HideActionDefRaw>,
    martial_hand_block_raw: Option<MartialHandBlockDefRaw>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfessionActionDefRaw {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub class_ids: Vec<String>,
    #[serde(default)]
    pub hide: Option<serde_json::Value>,
    #[serde(default)]
    pub martial_hand_block: Option<serde_json::Value>,
}

impl<'de> Deserialize<'de> for ProfessionActionDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let hide_field_present = value
            .as_object()
            .is_some_and(|object| object.contains_key("hide"));
        let martial_hand_block_field_present = value
            .as_object()
            .is_some_and(|object| object.contains_key("martial_hand_block"));
        let raw =
            serde_json::from_value::<ProfessionActionDefRaw>(value).map_err(de::Error::custom)?;
        let hide_raw = match raw.hide.as_ref() {
            Some(serde_json::Value::Object(map)) => Some(
                serde_json::from_value(serde_json::Value::Object(map.clone()))
                    .map_err(de::Error::custom)?,
            ),
            _ => None,
        };
        let hide = hide_raw.as_ref().and_then(HideActionDefRaw::to_typed);
        let martial_hand_block_raw = match raw.martial_hand_block.as_ref() {
            Some(serde_json::Value::Object(map)) => Some(
                serde_json::from_value(serde_json::Value::Object(map.clone()))
                    .map_err(de::Error::custom)?,
            ),
            _ => None,
        };
        let martial_hand_block = martial_hand_block_raw
            .as_ref()
            .and_then(MartialHandBlockDefRaw::to_typed);

        Ok(Self {
            id: raw.id,
            kind: raw.kind,
            class_ids: raw.class_ids,
            hide,
            martial_hand_block,
            hide_field_present,
            martial_hand_block_field_present,
            hide_raw,
            martial_hand_block_raw,
        })
    }
}

impl Serialize for ProfessionActionDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field_count = 3
            + usize::from(self.hide_field_present)
            + usize::from(self.martial_hand_block_field_present);
        let mut state = serializer.serialize_struct("ProfessionActionDef", field_count)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("class_ids", &self.class_ids)?;
        if self.hide_field_present {
            state.serialize_field("hide", &self.hide)?;
        }
        if self.martial_hand_block_field_present {
            state.serialize_field("martial_hand_block", &self.martial_hand_block)?;
        }
        state.end()
    }
}

impl ProfessionActionDef {
    pub(super) fn validate_kind_fields(&self, prefix: &str, errors: &mut Vec<String>) {
        match self.kind.as_str() {
            "hide" => {
                let Some(hide) = &self.hide_raw else {
                    errors.push(format!("{prefix}.hide must be present for hide actions"));
                    if self.martial_hand_block_field_present {
                        errors.push(format!(
                            "{prefix}.martial_hand_block is only valid for martial_hand_block actions"
                        ));
                    }
                    return;
                };
                if self.martial_hand_block_field_present {
                    errors.push(format!(
                        "{prefix}.martial_hand_block is only valid for martial_hand_block actions"
                    ));
                }
                hide.validate(&format!("{prefix}.hide"), errors);
            }
            "martial_hand_block" => {
                let Some(block) = &self.martial_hand_block_raw else {
                    errors.push(format!(
                        "{prefix}.martial_hand_block must be present for martial_hand_block actions"
                    ));
                    if self.hide_field_present {
                        errors.push(format!("{prefix}.hide is only valid for hide actions"));
                    }
                    return;
                };
                if self.hide_field_present {
                    errors.push(format!("{prefix}.hide is only valid for hide actions"));
                }
                block.validate(&format!("{prefix}.martial_hand_block"), errors);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HideActionDefRaw {
    #[serde(default)]
    effect_id: Option<serde_json::Value>,
    #[serde(default)]
    duration_rounds: Option<serde_json::Value>,
    #[serde(default)]
    requires_cover_or_darkness: Option<serde_json::Value>,
    #[serde(default)]
    break_on: Option<serde_json::Value>,
    #[serde(default)]
    disallow_two_handed: Option<serde_json::Value>,
    #[serde(default, flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

impl HideActionDefRaw {
    fn to_typed(&self) -> Option<HideActionDef> {
        let effect_id = self.effect_id.as_ref()?.as_str()?.trim().to_string();
        if effect_id.is_empty() {
            return None;
        }

        let duration_rounds = self.duration_rounds.as_ref()?.as_u64()?;
        if duration_rounds == 0 || duration_rounds > u32::MAX as u64 {
            return None;
        }

        let requires_cover_or_darkness = self.requires_cover_or_darkness.as_ref()?.as_bool()?;
        let disallow_two_handed = self.disallow_two_handed.as_ref()?.as_bool()?;
        let break_on = self.break_on.as_ref()?.as_array()?;
        let break_on = break_on
            .iter()
            .map(|value| {
                let trigger = value.as_str()?;
                HIDE_BREAK_TRIGGERS
                    .contains(&trigger)
                    .then(|| trigger.to_string())
            })
            .collect::<Option<Vec<_>>>()?;

        Some(HideActionDef {
            effect_id,
            duration_rounds: duration_rounds as u32,
            requires_cover_or_darkness,
            break_on,
            disallow_two_handed,
        })
    }

    fn validate(&self, prefix: &str, errors: &mut Vec<String>) {
        for key in self.extra.keys() {
            errors.push(format!("{prefix} has unknown field: {key}"));
        }

        let effect_id = self
            .effect_id
            .as_ref()
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if effect_id.is_none() {
            errors.push(format!("{prefix}.effect_id must be non-empty"));
        }

        let duration_rounds = self
            .duration_rounds
            .as_ref()
            .and_then(|value| value.as_u64())
            .filter(|value| *value > 0 && *value <= u32::MAX as u64);
        if duration_rounds.is_none() {
            errors.push(format!("{prefix}.duration_rounds must be positive"));
        }

        for (field, value) in [
            (
                "requires_cover_or_darkness",
                self.requires_cover_or_darkness.as_ref(),
            ),
            ("disallow_two_handed", self.disallow_two_handed.as_ref()),
        ] {
            if !matches!(value, Some(serde_json::Value::Bool(_))) {
                errors.push(format!("{prefix}.{field} must be a boolean"));
            }
        }

        match self.break_on.as_ref() {
            Some(serde_json::Value::Array(values)) => {
                for (break_index, value) in values.iter().enumerate() {
                    let Some(trigger) = value.as_str() else {
                        errors.push(format!(
                            "{prefix}.break_on[{break_index}] must be one of active_item_move, attack, cast, move, warm"
                        ));
                        continue;
                    };
                    if !HIDE_BREAK_TRIGGERS.contains(&trigger) {
                        errors.push(format!(
                            "{prefix}.break_on[{break_index}] must be one of active_item_move, attack, cast, move, warm"
                        ));
                    }
                }
            }
            Some(_) | None => {
                errors.push(format!("{prefix}.break_on must be a list"));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HideActionDef {
    pub effect_id: String,
    pub duration_rounds: u32,
    pub requires_cover_or_darkness: bool,
    pub break_on: Vec<String>,
    pub disallow_two_handed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MartialHandBlockDef {
    pub min_hand_level: i32,
    pub level_divisor: i32,
    pub max_chance_percent: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MartialHandBlockDefRaw {
    #[serde(default)]
    min_hand_level: Option<serde_json::Value>,
    #[serde(default)]
    level_divisor: Option<serde_json::Value>,
    #[serde(default)]
    max_chance_percent: Option<serde_json::Value>,
    #[serde(default, flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

impl MartialHandBlockDefRaw {
    fn to_typed(&self) -> Option<MartialHandBlockDef> {
        let min_hand_level = self.min_hand_level.as_ref()?.as_i64()?;
        if !(0..=19).contains(&min_hand_level) {
            return None;
        }
        let level_divisor = self.level_divisor.as_ref()?.as_i64()?;
        if level_divisor <= 0 || level_divisor > i32::MAX as i64 {
            return None;
        }
        let max_chance_percent = self.max_chance_percent.as_ref()?.as_i64()?;
        if !(1..=100).contains(&max_chance_percent) {
            return None;
        }
        Some(MartialHandBlockDef {
            min_hand_level: min_hand_level as i32,
            level_divisor: level_divisor as i32,
            max_chance_percent: max_chance_percent as i32,
        })
    }

    fn validate(&self, prefix: &str, errors: &mut Vec<String>) {
        for key in self.extra.keys() {
            errors.push(format!("{prefix} has unknown field: {key}"));
        }

        let min_hand_level = self
            .min_hand_level
            .as_ref()
            .and_then(|value| value.as_i64())
            .filter(|value| (0..=19).contains(value));
        if min_hand_level.is_none() {
            errors.push(format!("{prefix}.min_hand_level must be between 0 and 19"));
        }

        let level_divisor = self
            .level_divisor
            .as_ref()
            .and_then(|value| value.as_i64())
            .filter(|value| *value > 0 && *value <= i32::MAX as i64);
        if level_divisor.is_none() {
            errors.push(format!("{prefix}.level_divisor must be positive"));
        }

        let max_chance_percent = self
            .max_chance_percent
            .as_ref()
            .and_then(|value| value.as_i64())
            .filter(|value| (1..=100).contains(value));
        if max_chance_percent.is_none() {
            errors.push(format!(
                "{prefix}.max_chance_percent must be between 1 and 100"
            ));
        }
    }
}
