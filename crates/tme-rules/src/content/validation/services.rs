use std::collections::HashSet;

use crate::model::MAX_SKILL_LEVEL;

use super::ValidationBundle;
use crate::content::{ServiceCapabilityDef, SkillTrackKind, TrainingOfferDef};

const TRAINING_CLASS_IDS: &[&str] = &[
    "fighter",
    "knight",
    "martial_artist",
    "thaumaturge",
    "thief",
    "wizard",
];

pub(super) fn capability_id(capability: &ServiceCapabilityDef) -> &str {
    match capability {
        ServiceCapabilityDef::SkillTraining { id, .. }
        | ServiceCapabilityDef::SkillCritique { id }
        | ServiceCapabilityDef::SpellTeaching { id, .. }
        | ServiceCapabilityDef::ClassPromotion { id, .. }
        | ServiceCapabilityDef::ServiceTransaction { id, .. }
        | ServiceCapabilityDef::Merchant { id, .. }
        | ServiceCapabilityDef::ItemService { id, .. }
        | ServiceCapabilityDef::Restoration { id, .. }
        | ServiceCapabilityDef::Bank { id, .. }
        | ServiceCapabilityDef::Locker { id, .. } => id,
    }
}

impl ValidationBundle {
    pub(super) fn validate_training_offers(
        &self,
        capability_label: &str,
        offers: &[TrainingOfferDef],
        errors: &mut Vec<String>,
    ) {
        if offers.is_empty() {
            errors.push(format!(
                "{capability_label}.offers must be a non-empty list"
            ));
            return;
        }
        let Some(catalog) = self.skill_catalog.as_ref() else {
            errors.push("skill_catalog is required when skill_training exists".to_string());
            return;
        };

        let mut covered_pairs = HashSet::new();
        for (offer_index, offer) in offers.iter().enumerate() {
            let label = format!("{capability_label}.offers[{offer_index}]");
            let Some(track) = catalog.track(&offer.track_id) else {
                errors.push(format!(
                    "{label}.track_id references unknown skill catalog track {:?}",
                    offer.track_id
                ));
                continue;
            };
            if offer.eligible_class_ids.is_empty() {
                errors.push(format!(
                    "{label}.eligible_class_ids must be a non-empty list"
                ));
            }
            let mut class_ids = HashSet::new();
            for (class_index, class_id) in offer.eligible_class_ids.iter().enumerate() {
                if class_id.trim().is_empty() {
                    errors.push(format!(
                        "{label}.eligible_class_ids[{class_index}] must be non-empty"
                    ));
                    continue;
                }
                if !class_ids.insert(class_id.as_str()) {
                    errors.push(format!(
                        "{label}.eligible_class_ids must not contain duplicates"
                    ));
                }
                if !TRAINING_CLASS_IDS.contains(&class_id.as_str()) {
                    errors.push(format!(
                        "{label}.eligible_class_ids[{class_index}] references unknown class {class_id:?}"
                    ));
                }
                if !catalog.track_is_eligible_for_class(&offer.track_id, class_id) {
                    errors.push(format!(
                        "{label}.eligible_class_ids[{class_index}] is not eligible for track {:?}",
                        offer.track_id
                    ));
                }
                if !covered_pairs.insert((offer.track_id.as_str(), class_id.as_str())) {
                    errors.push(format!(
                        "{capability_label}.offers must not duplicate a track/class pair"
                    ));
                }
            }
            if track.kind == SkillTrackKind::Magic
                && class_ids
                    != track
                        .eligible_class_ids
                        .iter()
                        .map(String::as_str)
                        .collect::<HashSet<_>>()
            {
                errors.push(format!(
                    "{label}.eligible_class_ids must exactly match the magic track eligibility"
                ));
            }
            if offer.minimum_category_level > MAX_SKILL_LEVEL {
                errors.push(format!(
                    "{label}.minimum_category_level must be between 0 and 19"
                ));
            }
            if offer.maximum_category_level > MAX_SKILL_LEVEL {
                errors.push(format!(
                    "{label}.maximum_category_level must be between 0 and 19"
                ));
            }
            if offer.minimum_category_level > offer.maximum_category_level {
                errors.push(format!(
                    "{label}.minimum_category_level must not exceed maximum_category_level"
                ));
            }
        }
    }
}
