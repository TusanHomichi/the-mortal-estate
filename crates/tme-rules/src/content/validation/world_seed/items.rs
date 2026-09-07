//! Mutable seed validation: items.
use super::*;

pub(super) fn validate_ground_items(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    for (index, ground) in seed.ground_items.iter().enumerate() {
        validate_world_position(
            context,
            &ground.location,
            &format!("ground_items[{index}].location"),
            errors,
        );
    }
}

pub(super) fn validate_item_instances(
    seed: &WorldSeedDef,
    context: &impl WorldSeedValidationContext,
    errors: &mut Vec<String>,
) {
    let character_ids = seed
        .actors
        .iter()
        .filter_map(|actor| actor.character_id.as_ref())
        .collect::<HashSet<_>>();
    for (instance_id, instance) in &seed.item_instances {
        let label = format!("item_instances[{instance_id:?}]");
        if instance_id.trim().is_empty() {
            errors.push("item_instances keys must be non-empty".to_string());
        }
        if instance_id.starts_with("summon:") {
            errors.push(format!(
                "item instance {instance_id:?} must not use reserved prefix \"summon:\""
            ));
        }
        if instance.quantity == 0 {
            errors.push(format!("{label}.quantity must be positive"));
        }
        if !matches!(instance.binding, ItemBindingState::Unrestricted) && instance.quantity != 1 {
            errors.push(format!(
                "{label}.quantity must be 1 for a tied item instance"
            ));
        }
        if let ItemBindingState::Bound { character_id } = &instance.binding
            && character_id.as_str().trim().is_empty()
        {
            errors.push(format!("{label}.binding.character_id must be non-empty"));
        }
        let Some(item) = context.item(&instance.definition_id) else {
            errors.push(format!(
                "{label}.definition_id references unknown item definition {:?}",
                instance.definition_id
            ));
            continue;
        };
        if item
            .capability
            .and_then(|capability| capability.spell_book_for.as_ref())
            .is_some()
        {
            if instance.quantity != 1 {
                errors.push(format!("{label}.quantity must be 1 for a Spell Book"));
            }
            match &instance.binding {
                ItemBindingState::Bound { character_id }
                    if character_ids.contains(character_id) => {}
                ItemBindingState::Bound { .. } => errors.push(format!(
                    "{label}.binding.character_id references no scenario character"
                )),
                _ => errors.push(format!("{label}.binding must be bound for a Spell Book")),
            }
        }
        if item
            .economy
            .unit_value_gold
            .is_some_and(|unit| unit.checked_mul(u64::from(instance.quantity)).is_none())
        {
            errors.push(format!(
                "{label}.quantity * unit_value_gold must not overflow"
            ));
        }
        if item
            .economy
            .unit_burden
            .checked_mul(u64::from(instance.quantity))
            .is_none()
        {
            errors.push(format!("{label}.quantity * unit_burden must not overflow"));
        }
    }

    let mut owners = HashMap::<&str, String>::new();
    for (actor_index, actor) in seed.actors.iter().enumerate() {
        if actor.carried.gold.left_hand < 0
            || actor.carried.gold.right_hand < 0
            || actor.carried.gold.sack < 0
        {
            errors.push(format!(
                "actors[{actor_index}].carried.gold values must be non-negative"
            ));
        }
        if actor.carried.gold.checked_total().is_none() {
            errors.push(format!(
                "actors[{actor_index}].carried.gold total must fit a signed 64-bit integer"
            ));
        }
        let mut positions = HashMap::new();
        for (item_index, positioned) in actor.carried.items.iter().enumerate() {
            let label =
                format!("actors[{actor_index}].carried.items[{item_index}].item_instance_id");
            record_owner(
                &seed.item_instances,
                &mut owners,
                &positioned.item_instance_id,
                &label,
                errors,
            );
            if let Some(previous) = positions.insert(positioned.position, item_index) {
                errors.push(format!(
                    "actors[{actor_index}].carried.items[{item_index}].position duplicates actors[{actor_index}].carried.items[{previous}].position"
                ));
            }
            if let Some(instance) = seed.item_instances.get(&positioned.item_instance_id)
                && let Some(item) = context.item(&instance.definition_id)
                && !item
                    .valid_placements
                    .contains(&positioned.position.placement_kind())
            {
                errors.push(format!(
                    "{label} cannot occupy carried position {:?}",
                    positioned.position.label()
                ));
            }
            if let Some(instance) = seed.item_instances.get(&positioned.item_instance_id)
                && !positioned.position.is_sack_item()
                && instance.quantity != 1
            {
                errors.push(format!("{label} must have quantity 1 outside the sack"));
            }
        }
        for (position, amount) in [
            (CarriedPosition::LeftHand, actor.carried.gold.left_hand),
            (CarriedPosition::RightHand, actor.carried.gold.right_hand),
        ] {
            if amount > 0 && positions.contains_key(&position) {
                errors.push(format!(
                    "actors[{actor_index}].carried cannot place an item and gold in {}",
                    position.label()
                ));
            }
        }
    }
    for (index, ground) in seed.ground_items.iter().enumerate() {
        record_owner(
            &seed.item_instances,
            &mut owners,
            &ground.item_instance_id,
            &format!("ground_items[{index}].item_instance_id"),
            errors,
        );
    }
    for (inventory_index, inventory) in seed.merchant_inventories.iter().enumerate() {
        for (stock_index, stock) in inventory.stock.iter().enumerate() {
            record_owner(
                &seed.item_instances,
                &mut owners,
                &stock.item_instance_id,
                &format!(
                    "merchant_inventories[{inventory_index}].stock[{stock_index}].item_instance_id"
                ),
                errors,
            );
        }
    }
    for instance_id in seed.item_instances.keys() {
        if !owners.contains_key(instance_id.as_str()) {
            errors.push(format!(
                "item instance {instance_id:?} has no owner or location"
            ));
        }
    }
}

pub(super) fn record_owner<'a>(
    registry: &'a BTreeMap<String, ItemInstanceSeedDef>,
    owners: &mut HashMap<&'a str, String>,
    instance_id: &'a str,
    label: &str,
    errors: &mut Vec<String>,
) {
    if instance_id.trim().is_empty() {
        errors.push(format!("{label} must be non-empty"));
    } else if !registry.contains_key(instance_id) {
        errors.push(format!(
            "{label} references unknown item instance {instance_id:?}"
        ));
    } else if let Some(previous) = owners.insert(instance_id, label.to_string()) {
        errors.push(format!(
            "item instance {instance_id:?} is referenced more than once ({previous} and {label})"
        ));
    }
}
