use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::MAX_SKILL_LEVEL;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingRulesDef {
    pub gold_per_learning_rate: i64,
    pub experience_per_learning_rate: i32,
    pub maximum_learning_rates: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRulesDef {
    pub base_learning_rate: u64,
    pub practice_thresholds: Vec<u64>,
    pub training: TrainingRulesDef,
}

impl SkillRulesDef {
    pub(crate) fn validate_intrinsic(&self, prefix: &str, errors: &mut Vec<String>) {
        let expected_len = usize::from(MAX_SKILL_LEVEL) + 1;
        if self.base_learning_rate == 0 {
            errors.push(format!("{prefix}.base_learning_rate must be positive"));
        }
        if self.practice_thresholds.len() != expected_len {
            errors.push(format!(
                "{prefix}.practice_thresholds must contain exactly {expected_len} level-ordered values"
            ));
        }
        for (index, threshold) in self.practice_thresholds.iter().enumerate() {
            if *threshold == 0 {
                errors.push(format!(
                    "{prefix}.practice_thresholds[{index}] must be positive"
                ));
            }
        }

        let training = &self.training;
        if training.gold_per_learning_rate <= 0 {
            errors.push(format!(
                "{prefix}.training.gold_per_learning_rate must be positive"
            ));
        }
        if training.experience_per_learning_rate <= 0 {
            errors.push(format!(
                "{prefix}.training.experience_per_learning_rate must be positive"
            ));
        }
        if training.maximum_learning_rates.len() != expected_len {
            errors.push(format!(
                "{prefix}.training.maximum_learning_rates must contain exactly {expected_len} level-ordered values"
            ));
        }
        for (index, maximum) in training.maximum_learning_rates.iter().enumerate() {
            if *maximum == 0 {
                errors.push(format!(
                    "{prefix}.training.maximum_learning_rates[{index}] must be positive"
                ));
            }
            if *maximum < self.base_learning_rate {
                errors.push(format!(
                    "{prefix}.training.maximum_learning_rates[{index}] must be at least base_learning_rate"
                ));
            }
            if index > 0 && *maximum <= training.maximum_learning_rates[index - 1] {
                errors.push(format!(
                    "{prefix}.training.maximum_learning_rates[{index}] must be greater than the previous level"
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillTrackKind {
    Weapon,
    MartialArts,
    Thievery,
    Magic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillLevelTitleDef {
    pub level: u8,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillLadderDef {
    pub id: String,
    pub titles: Vec<SkillLevelTitleDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillTrackDef {
    pub id: String,
    pub display: String,
    pub kind: SkillTrackKind,
    pub ladder_id: String,
    #[serde(default)]
    pub eligible_class_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillCatalogDef {
    pub ladders: Vec<SkillLadderDef>,
    pub tracks: Vec<SkillTrackDef>,
}

impl SkillCatalogDef {
    pub(crate) fn validate_intrinsic(&self, prefix: &str, errors: &mut Vec<String>) {
        let mut ladder_ids = HashSet::new();
        for (index, ladder) in self.ladders.iter().enumerate() {
            let label = format!("{prefix}.ladders[{index}]");
            if ladder.id.trim().is_empty() {
                errors.push(format!("{label}.id must be non-empty"));
            }
            if !ladder_ids.insert(ladder.id.as_str()) {
                errors.push(format!("{label}.id must be unique"));
            }
            let expected_len = usize::from(MAX_SKILL_LEVEL) + 1;
            if ladder.titles.len() != expected_len {
                errors.push(format!(
                    "{label}.titles must contain exactly {expected_len} ordered levels"
                ));
            }
            for (title_index, title) in ladder.titles.iter().enumerate() {
                let title_label = format!("{label}.titles[{title_index}]");
                if usize::from(title.level) != title_index {
                    errors.push(format!(
                        "{title_label}.level must equal its ordered index {title_index}"
                    ));
                }
                if title.title.trim().is_empty() {
                    errors.push(format!("{title_label}.title must be non-empty"));
                }
            }
        }
        if self.ladders.is_empty() {
            errors.push(format!("{prefix}.ladders must be non-empty"));
        }

        let mut track_ids = HashSet::new();
        for (index, track) in self.tracks.iter().enumerate() {
            let label = format!("{prefix}.tracks[{index}]");
            if track.id.trim().is_empty() {
                errors.push(format!("{label}.id must be non-empty"));
            }
            if !track_ids.insert(track.id.as_str()) {
                errors.push(format!("{label}.id must be unique"));
            }
            if track.display.trim().is_empty() {
                errors.push(format!("{label}.display must be non-empty"));
            }
            if !ladder_ids.contains(track.ladder_id.as_str()) {
                errors.push(format!(
                    "{label}.ladder_id references unknown ladder {:?}",
                    track.ladder_id
                ));
            }
            let mut class_ids = HashSet::new();
            for (class_index, class_id) in track.eligible_class_ids.iter().enumerate() {
                if class_id.trim().is_empty() {
                    errors.push(format!(
                        "{label}.eligible_class_ids[{class_index}] must be non-empty"
                    ));
                }
                if !class_ids.insert(class_id.as_str()) {
                    errors.push(format!(
                        "{label}.eligible_class_ids must not contain duplicates"
                    ));
                }
            }
            if track.kind == SkillTrackKind::Magic {
                if track.eligible_class_ids.is_empty() {
                    errors.push(format!(
                        "{label}.eligible_class_ids must be non-empty for a magic track"
                    ));
                }
                if track
                    .eligible_class_ids
                    .iter()
                    .any(|class_id| class_id == "knight")
                {
                    errors.push(format!(
                        "{label}.eligible_class_ids must not grant magic skill to knight"
                    ));
                }
            }
        }
        if self.tracks.is_empty() {
            errors.push(format!("{prefix}.tracks must be non-empty"));
        }
    }

    pub fn track(&self, track_id: &str) -> Option<&SkillTrackDef> {
        self.tracks.iter().find(|track| track.id == track_id)
    }

    pub fn track_is_eligible_for_class(&self, track_id: &str, class_id: &str) -> bool {
        self.track(track_id).is_some_and(|track| {
            track.eligible_class_ids.is_empty()
                || track
                    .eligible_class_ids
                    .iter()
                    .any(|eligible| eligible == class_id)
        })
    }

    pub fn track_display(&self, track_id: &str) -> Option<&str> {
        self.track(track_id).map(|track| track.display.as_str())
    }

    pub fn level_title(&self, track_id: &str, level: u8) -> Option<&str> {
        let ladder_id = &self.track(track_id)?.ladder_id;
        self.ladders
            .iter()
            .find(|ladder| ladder.id == *ladder_id)?
            .titles
            .get(usize::from(level))
            .map(|title| title.title.as_str())
    }
}
