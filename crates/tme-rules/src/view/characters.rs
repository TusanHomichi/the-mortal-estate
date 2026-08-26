use serde::{Deserialize, Serialize};

use crate::model::{
    AttributeBonus, CharacterAlignment, CharacterAlignmentState, CharacterAttributes,
    CharacterIdentity, CharacterResources, PhysicalAttributeAdds, PromotionEntry,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeBonusViewV1 {
    pub stat: String,
    pub value: i32,
}

// --- Character sheet view types (Slice AQ) ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterIdentityViewV1 {
    pub base_class_id: String,
    pub current_class_id: String,
    pub display_class: String,
    pub nationality_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sex_or_gender_display: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterAttributesViewV1 {
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub charisma: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterResourcesViewV1 {
    pub hp: i32,
    pub max_hp: i32,
    pub peak_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub stamina: i32,
    pub max_stamina: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterProgressionViewV1 {
    pub level: i32,
    pub experience: i64,
    pub pending_target_level: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PhysicalAttributeAddsViewV1 {
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterAlignmentStateViewV1 {
    pub alignment: CharacterAlignment,
    pub karma_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PromotionEntryViewV1 {
    pub from_class_id: String,
    pub to_class_id: String,
    pub level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterSheetViewV1 {
    pub identity: CharacterIdentityViewV1,
    pub alignment_state: CharacterAlignmentStateViewV1,
    pub attributes: CharacterAttributesViewV1,
    pub resources: CharacterResourcesViewV1,
    pub progression: CharacterProgressionViewV1,
    pub physical_attribute_adds: PhysicalAttributeAddsViewV1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub promotion_history: Vec<PromotionEntryViewV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub known_spells: Vec<KnownSpellViewV1>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skill_ledger: Vec<SkillEntryViewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnownSpellViewV1 {
    pub spell_id: String,
    pub lane: String,
    pub learned_at_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SkillEntryViewV1 {
    pub track_id: String,
    pub level: u8,
    pub critique_rank: u8,
    pub practice_points: u64,
    pub learning_rate: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_title: Option<String>,
}

// --- Character sheet view From impls (Slice AQ) ---

impl From<&CharacterIdentity> for CharacterIdentityViewV1 {
    fn from(id: &CharacterIdentity) -> Self {
        Self {
            base_class_id: id.base_class_id.clone(),
            current_class_id: id.current_class_id.clone(),
            display_class: id.display_class.clone(),
            nationality_id: id.nationality_id.clone(),
            sex_or_gender_display: id.sex_or_gender_display.clone(),
        }
    }
}

impl From<&CharacterAttributes> for CharacterAttributesViewV1 {
    fn from(a: &CharacterAttributes) -> Self {
        Self {
            strength: a.strength,
            dexterity: a.dexterity,
            constitution: a.constitution,
            intelligence: a.intelligence,
            wisdom: a.wisdom,
            charisma: a.charisma,
        }
    }
}

impl From<&CharacterResources> for CharacterResourcesViewV1 {
    fn from(r: &CharacterResources) -> Self {
        Self {
            hp: r.hp,
            max_hp: r.max_hp,
            peak_hp: r.peak_hp,
            mp: r.mp,
            max_mp: r.max_mp,
            stamina: r.stamina,
            max_stamina: r.max_stamina,
        }
    }
}

impl From<&PhysicalAttributeAdds> for PhysicalAttributeAddsViewV1 {
    fn from(c: &PhysicalAttributeAdds) -> Self {
        Self {
            strength_adds: c.strength_adds,
            dexterity_adds: c.dexterity_adds,
        }
    }
}

impl From<&CharacterAlignmentState> for CharacterAlignmentStateViewV1 {
    fn from(state: &CharacterAlignmentState) -> Self {
        Self {
            alignment: state.alignment,
            karma_points: state.karma_points,
        }
    }
}

impl From<&PromotionEntry> for PromotionEntryViewV1 {
    fn from(p: &PromotionEntry) -> Self {
        Self {
            from_class_id: p.from_class_id.clone(),
            to_class_id: p.to_class_id.clone(),
            level: p.level,
        }
    }
}

// --- Item capability view From impls (Slice BC) ---

impl From<&AttributeBonus> for AttributeBonusViewV1 {
    fn from(b: &AttributeBonus) -> Self {
        Self {
            stat: b.stat.clone(),
            value: b.value,
        }
    }
}
