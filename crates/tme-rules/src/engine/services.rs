//! Read-only typed service lookup and capability resolution.

use crate::model::{
    BankCapability, ClassPromotionCapability, LockerCapability, ResolvedService, ServiceCapability,
    ServiceTransactionCapability, SkillCritiqueCapability, SkillTrainingCapability,
    SpellTeachingCapability,
};

use super::Engine;

fn capability_id(capability: &ServiceCapability) -> &str {
    match capability {
        ServiceCapability::SkillTraining(capability) => &capability.id,
        ServiceCapability::SkillCritique(capability) => &capability.id,
        ServiceCapability::SpellTeaching(capability) => &capability.id,
        ServiceCapability::ClassPromotion(capability) => &capability.id,
        ServiceCapability::ServiceTransaction(capability) => &capability.id,
        ServiceCapability::Merchant(capability) => &capability.id,
        ServiceCapability::ItemService(capability) => &capability.id,
        ServiceCapability::Restoration(capability) => &capability.id,
        ServiceCapability::Bank(capability) => &capability.id,
        ServiceCapability::Locker(capability) => &capability.id,
    }
}

impl Engine {
    fn service_definition_by_id(
        &self,
        definition_id: &str,
    ) -> Option<&crate::model::ServiceDefinition> {
        self.definition
            .catalog
            .service_definitions
            .iter()
            .find(|definition| definition.id == definition_id)
    }

    pub(super) fn service_by_id(&self, service_id: &str) -> Option<ResolvedService<'_>> {
        let instance = self
            .world
            .service_instances
            .iter()
            .find(|instance| instance.id == service_id)?;
        let definition = self.service_definition_by_id(&instance.definition_id)?;
        Some(ResolvedService::new(instance, definition))
    }

    pub(super) fn services_at_actor(&self, actor_index: usize) -> Vec<ResolvedService<'_>> {
        let Some(actor) = self.world.actors.get(actor_index) else {
            return Vec::new();
        };
        self.world
            .service_instances
            .iter()
            .filter(|instance| {
                instance.position.level == actor.location.level
                    && instance.position.position == actor.location.position
            })
            .filter_map(|instance| {
                self.service_definition_by_id(&instance.definition_id)
                    .map(|definition| ResolvedService::new(instance, definition))
            })
            .collect()
    }

    pub(super) fn bank_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        capability_id: &str,
    ) -> Option<&'a BankCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::Bank(capability) if capability.id == capability_id => {
                    Some(capability)
                }
                _ => None,
            })
    }

    pub(super) fn locker_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        capability_id: &str,
    ) -> Option<&'a LockerCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::Locker(capability) if capability.id == capability_id => {
                    Some(capability)
                }
                _ => None,
            })
    }

    pub(super) fn skill_training_capability<'a>(
        &self,
        service: ResolvedService<'a>,
    ) -> Option<&'a SkillTrainingCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::SkillTraining(capability) => Some(capability),
                _ => None,
            })
    }

    pub(super) fn skill_critique_capability<'a>(
        &self,
        service: ResolvedService<'a>,
    ) -> Option<&'a SkillCritiqueCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::SkillCritique(capability) => Some(capability),
                _ => None,
            })
    }

    pub(super) fn referenced_training_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        teaching: &'a SpellTeachingCapability,
    ) -> Option<&'a SkillTrainingCapability> {
        match service.capabilities().iter().find(|capability| {
            capability_id(capability) == teaching.training_capability_id.as_str()
        }) {
            Some(ServiceCapability::SkillTraining(capability)) => Some(capability),
            _ => None,
        }
    }

    pub(super) fn spell_teachers_for(
        &self,
        spell_id: &str,
    ) -> Vec<(ResolvedService<'_>, &SpellTeachingCapability)> {
        self.world
            .service_instances
            .iter()
            .filter_map(|instance| {
                self.service_definition_by_id(&instance.definition_id)
                    .map(|definition| ResolvedService::new(instance, definition))
            })
            .flat_map(|service| {
                service.capabilities().iter().filter_map(move |capability| {
                    let ServiceCapability::SpellTeaching(teaching) = capability else {
                        return None;
                    };
                    teaching
                        .teachings
                        .iter()
                        .any(|row| row.spell_id == spell_id)
                        .then_some((service, teaching))
                })
            })
            .collect()
    }

    pub(super) fn promotion_capabilities_at_actor(
        &self,
        actor_index: usize,
        target_class_id: &str,
    ) -> Vec<(ResolvedService<'_>, &ClassPromotionCapability)> {
        self.services_at_actor(actor_index)
            .into_iter()
            .flat_map(|service| {
                service.capabilities().iter().filter_map(move |capability| {
                    let ServiceCapability::ClassPromotion(promotion) = capability else {
                        return None;
                    };
                    promotion
                        .transaction
                        .rewards
                        .iter()
                        .any(|reward| {
                            matches!(
                                reward,
                                crate::model::TransactionReward::Class { to_class_id, .. }
                                    if to_class_id == target_class_id
                            )
                        })
                        .then_some((service, promotion))
                })
            })
            .collect()
    }

    pub(super) fn service_transaction_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        capability_id: &str,
    ) -> Option<&'a ServiceTransactionCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::ServiceTransaction(capability)
                    if capability.id == capability_id =>
                {
                    Some(capability)
                }
                _ => None,
            })
    }

    pub(super) fn merchant_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        capability_id: &str,
    ) -> Option<&'a crate::model::MerchantCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::Merchant(capability) if capability.id == capability_id => {
                    Some(capability)
                }
                _ => None,
            })
    }

    pub(super) fn item_service_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        capability_id: &str,
    ) -> Option<&'a crate::model::ItemServiceCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::ItemService(capability) if capability.id == capability_id => {
                    Some(capability)
                }
                _ => None,
            })
    }

    pub(super) fn restoration_capability<'a>(
        &self,
        service: ResolvedService<'a>,
        capability_id: &str,
    ) -> Option<&'a crate::model::RestorationCapability> {
        service
            .capabilities()
            .iter()
            .find_map(|capability| match capability {
                ServiceCapability::Restoration(capability) if capability.id == capability_id => {
                    Some(capability)
                }
                _ => None,
            })
    }
}
