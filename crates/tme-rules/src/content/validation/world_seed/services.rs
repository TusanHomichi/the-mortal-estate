//! Mutable seed validation: services.
use super::*;

pub(super) fn validate_services(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let mut instance_ids = HashMap::new();
    let mut inventories = HashMap::new();
    let mut promotion_keys = HashMap::new();
    let mut service_grant_ids = HashMap::new();
    let mut spell_teaching_keys = HashMap::new();
    for (index, inventory) in seed.merchant_inventories.iter().enumerate() {
        let key = (
            inventory.service_instance_id.as_str(),
            inventory.capability_id.as_str(),
        );
        if let Some(previous) = inventories.insert(key, index) {
            errors.push(format!(
                "merchant_inventories[{index}] duplicates merchant_inventories[{previous}] service/capability"
            ));
        }
    }
    for (index, instance) in seed.service_instances.iter().enumerate() {
        let label = format!("service_instances[{index}]");
        if instance.id.trim().is_empty() {
            errors.push(format!("{label}.id must be non-empty"));
        } else if let Some(previous) = instance_ids.insert(instance.id.as_str(), index) {
            errors.push(format!(
                "{label}.id duplicates service_instances[{previous}].id"
            ));
        }
        if !context.service_definition_exists(&instance.service_definition_id) {
            errors.push(format!(
                "{label}.service_definition_id references unknown selected service definition {:?}",
                instance.service_definition_id
            ));
        }
        let location = match &instance.placement {
            crate::model::ServicePlacement::Fixed { location } => location,
            crate::model::ServicePlacement::Actor { actor_id } => {
                let Some(provider) = seed.actors.iter().find(|actor| {
                    &actor.id == actor_id
                        && context.actor_definition_kind(&actor.actor_definition_id)
                            == Some(ActorKind::Npc)
                }) else {
                    errors.push(format!("{label}.placement references unknown NPC provider"));
                    continue;
                };
                &provider.location
            }
        };
        validate_world_position(context, location, &format!("{label}.placement"), errors);
        for promotion in context.promotion_capabilities(&instance.service_definition_id) {
            let key = (location.clone(), promotion.target_class_id.to_string());
            if let Some((previous_instance, previous_capability)) =
                promotion_keys.insert(key, (index, promotion.id.to_string()))
            {
                errors.push(format!(
                    "{label} promotion capability {:?} duplicates service_instances[{previous_instance}] promotion capability {previous_capability:?} room/position/target",
                    promotion.id
                ));
            }
        }
        for teaching in context.spell_teaching_pairs(&instance.service_definition_id) {
            let key = (
                location.clone(),
                teaching.class_id.to_string(),
                teaching.spell_id.to_string(),
            );
            if let Some((previous_instance, previous_capability)) =
                spell_teaching_keys.insert(key, (index, teaching.capability_id.to_string()))
            {
                errors.push(format!(
                    "{label} spell teaching capability {:?} duplicates service_instances[{previous_instance}] spell teaching capability {previous_capability:?} room/position/class/spell",
                    teaching.capability_id
                ));
            }
        }
        for item_instance_id in
            context.service_grant_item_instance_ids(&instance.service_definition_id)
        {
            if seed.item_instances.contains_key(item_instance_id) {
                errors.push(format!(
                    "{label} service definition item grant {item_instance_id:?} must not already be registered"
                ));
            }
            if let Some(previous_instance) =
                service_grant_ids.insert(item_instance_id.to_string(), index)
            {
                errors.push(format!(
                    "{label} service definition item grant {item_instance_id:?} duplicates service_instances[{previous_instance}] service definition item grant"
                ));
            }
        }
        for merchant in context.merchant_capabilities(&instance.service_definition_id) {
            if !inventories.contains_key(&(instance.id.as_str(), merchant.id)) {
                errors.push(format!(
                    "{label} merchant capability {:?} requires exactly one merchant inventory seed",
                    merchant.id
                ));
            }
            if let Some(multiplier) = context
                .merchant_pawn_listing_multiplier(&instance.service_definition_id, merchant.id)
            {
                for (item_instance_id, item_instance) in &seed.item_instances {
                    let Some(item) = context.item(&item_instance.definition_id) else {
                        continue;
                    };
                    let Some(unit_value) = item.economy.unit_value_gold else {
                        continue;
                    };
                    let total = unit_value
                        .checked_mul(u64::from(item_instance.quantity))
                        .and_then(|value| value.checked_mul(u64::from(multiplier)));
                    if total.is_none_or(|value| value > i64::MAX as u64) {
                        errors.push(format!(
                            "{label} merchant capability {:?} player_sales cannot price item instance {item_instance_id:?} within signed carried gold",
                            merchant.id
                        ));
                    }
                }
            }
        }
    }
    for (index, inventory) in seed.merchant_inventories.iter().enumerate() {
        let label = format!("merchant_inventories[{index}]");
        let Some(instance) = seed
            .service_instances
            .iter()
            .find(|instance| instance.id == inventory.service_instance_id)
        else {
            errors.push(format!(
                "{label}.service_instance_id references unknown service instance {:?}",
                inventory.service_instance_id
            ));
            continue;
        };
        if context
            .service_capability_kind(&instance.service_definition_id, &inventory.capability_id)
            != Some(SeedServiceCapabilityKind::Merchant)
        {
            errors.push(format!(
                "{label}.capability_id must reference a merchant capability"
            ));
            continue;
        }
        let accepts_sales = context
            .merchant_capabilities(&instance.service_definition_id)
            .into_iter()
            .find(|merchant| merchant.id == inventory.capability_id)
            .is_some_and(|merchant| merchant.accepts_player_sales);
        if inventory.stock.is_empty() && !accepts_sales {
            errors.push(format!(
                "{label}.stock must be non-empty when player_sales is null"
            ));
        }
        let mut stock_ids = HashSet::new();
        for (stock_index, stock) in inventory.stock.iter().enumerate() {
            let stock_label = format!("{label}.stock[{stock_index}]");
            if stock.item_instance_id.trim().is_empty() {
                errors.push(format!("{stock_label}.item_instance_id must be non-empty"));
            } else if !stock_ids.insert(stock.item_instance_id.as_str()) {
                errors.push(format!(
                    "{stock_label}.item_instance_id must be unique within the inventory"
                ));
            }
            if stock.price_gold <= 0 {
                errors.push(format!("{stock_label}.price_gold must be positive"));
            }
            if let Some(item_instance) = seed.item_instances.get(&stock.item_instance_id) {
                if !matches!(item_instance.binding, ItemBindingState::Unrestricted) {
                    errors.push(format!(
                        "{stock_label}.item_instance_id must reference an unrestricted item"
                    ));
                }
                if context
                    .item(&item_instance.definition_id)
                    .is_some_and(|item| !item.valid_placements.contains(&ItemPlacementKind::Sack))
                {
                    errors.push(format!(
                        "{stock_label}.item_instance_id definition must permit sack placement"
                    ));
                }
            } else {
                errors.push(format!(
                    "{stock_label}.item_instance_id references unknown item instance {:?}",
                    stock.item_instance_id
                ));
            }
        }
    }
}
