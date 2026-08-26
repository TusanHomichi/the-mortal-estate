use serde::{Deserialize, Serialize};

pub const MAX_SKILL_LEVEL: u8 = 19;
pub const MAX_CRITIQUE_RANK: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterAlignment {
    Lawful,
    Neutral,
    Chaotic,
    Evil,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterAlignmentState {
    pub alignment: CharacterAlignment,
    pub karma_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterAttributes {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterResources {
    pub hp: i32,
    pub max_hp: i32,
    pub peak_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub stamina: i32,
    pub max_stamina: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterProgression {
    pub level: i32,
    pub experience: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalAttributeAdds {
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterIdentity {
    pub base_class_id: String,
    pub current_class_id: String,
    pub display_class: String,
    pub nationality_id: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex_or_gender_display: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionEntry {
    pub from_class_id: String,
    pub to_class_id: String,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillEntry {
    pub track_id: String,
    pub level: u8,
    pub critique_rank: u8,
    pub practice_points: u64,
    pub learning_rate: u64,
}

impl SkillEntry {
    pub fn untrained(track_id: impl Into<String>, base_learning_rate: u64) -> Self {
        Self {
            track_id: track_id.into(),
            level: 0,
            critique_rank: 0,
            practice_points: 0,
            learning_rate: base_learning_rate,
        }
    }

    pub const fn has_valid_learning_rate(&self) -> bool {
        self.learning_rate > 0
    }

    pub const fn is_valid_position(&self) -> bool {
        if self.level == 0 {
            self.critique_rank == 0
        } else {
            self.level <= MAX_SKILL_LEVEL && self.critique_rank <= MAX_CRITIQUE_RANK
        }
    }

    pub const fn is_maximum(&self) -> bool {
        self.level == MAX_SKILL_LEVEL && self.critique_rank == MAX_CRITIQUE_RANK
    }

    pub fn advance_position(&mut self) -> bool {
        if !self.is_valid_position() || self.is_maximum() {
            return false;
        }
        if self.level == 0 {
            self.level = 1;
            self.critique_rank = 1;
        } else if self.critique_rank < MAX_CRITIQUE_RANK {
            self.critique_rank += 1;
        } else {
            self.level += 1;
            self.critique_rank = 1;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownSpell {
    pub spell_id: String,
    pub lane: String,
    pub learned_at_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterSheetV1 {
    pub identity: CharacterIdentity,
    pub alignment_state: CharacterAlignmentState,
    pub attributes: CharacterAttributes,
    pub resources: CharacterResources,
    pub progression: CharacterProgression,
    pub physical_attribute_adds: PhysicalAttributeAdds,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub promotion_history: Vec<PromotionEntry>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skill_ledger: Vec<SkillEntry>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub known_spells: Vec<KnownSpell>,
}
