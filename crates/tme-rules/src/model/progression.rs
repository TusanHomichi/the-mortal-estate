use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressionRules {
    pub level_thresholds: Vec<LevelThreshold>,
    pub growth_profiles: BTreeMap<String, ProgressionGrowthProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelThreshold {
    pub level: i32,
    pub cumulative_experience: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressionGrowthProfile {
    pub class_id: String,
    pub hit_points: GrowthRule,
    pub magic_points: Option<GrowthRule>,
    pub stamina_points: GrowthRule,
    pub physical_attribute_adds_by_level: Vec<CombatAddGrowth>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrowthRule {
    Fixed {
        outcomes: Vec<WeightedGrowthOutcome>,
    },
    AttributeBands {
        attribute: GrowthAttribute,
        bands: Vec<AttributeGrowthBand>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthAttribute {
    Strength,
    Constitution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeGrowthBand {
    pub minimum_attribute: i32,
    pub outcomes: Vec<WeightedGrowthOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedGrowthOutcome {
    pub amount: i32,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatAddGrowth {
    pub level: i32,
    pub strength_adds: i32,
    pub dexterity_adds: i32,
}
