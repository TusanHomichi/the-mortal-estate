use std::collections::HashMap;

use super::ValidationBundle;

impl ValidationBundle {
    pub(super) fn validate_quests(&self, errors: &mut Vec<String>) {
        let mut quest_ids = HashMap::new();
        let mut titles = HashMap::new();
        for (index, quest) in self.quests.iter().enumerate() {
            let label = format!("quests[{index}]");
            if quest.id.trim().is_empty() {
                errors.push(format!("{label}.id must be non-empty"));
            } else if let Some(previous) = quest_ids.insert(quest.id.as_str(), index) {
                errors.push(format!("{label}.id duplicates quests[{previous}].id"));
            }
            if quest.title.trim().is_empty() {
                errors.push(format!("{label}.title must be non-empty"));
            } else if let Some(previous) = titles.insert(quest.title.as_str(), index) {
                errors.push(format!("{label}.title duplicates quests[{previous}].title"));
            }
            if quest.stages.is_empty() {
                errors.push(format!("{label}.stages must be non-empty"));
            }
            let mut stage_ids = HashMap::new();
            let mut terminal_count = 0;
            for (stage_index, stage) in quest.stages.iter().enumerate() {
                let stage_label = format!("{label}.stages[{stage_index}]");
                if stage.id.trim().is_empty() {
                    errors.push(format!("{stage_label}.id must be non-empty"));
                } else if let Some(previous) = stage_ids.insert(stage.id.as_str(), stage_index) {
                    errors.push(format!(
                        "{stage_label}.id duplicates {label}.stages[{previous}].id"
                    ));
                }
                if stage.label.trim().is_empty() {
                    errors.push(format!("{stage_label}.label must be non-empty"));
                }
                terminal_count += usize::from(stage.terminal);
            }
            if !quest.stages.is_empty() && terminal_count == 0 {
                errors.push(format!("{label}.stages must contain a terminal stage"));
            }
        }
    }
}
