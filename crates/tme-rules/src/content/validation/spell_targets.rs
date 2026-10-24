use super::*;

fn requires_item_target(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::WeaponEnchant
    )
}

fn requires_bu_target(family: SpellEffectFamily) -> bool {
    matches!(
        family,
        SpellEffectFamily::DoorControl
            | SpellEffectFamily::SecretDetection
            | SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::WeaponEnchant
            | SpellEffectFamily::Locate
            | SpellEffectFamily::Portal
            | SpellEffectFamily::Scry
    )
}

impl ValidationBundle {
    pub(super) fn validate_spell_target(
        &self,
        index: usize,
        spell: &SpellDef,
        errors: &mut Vec<String>,
    ) {
        if let Some(effect) = &spell.effect
            && requires_bu_target(effect.family)
            && spell.target.is_none()
        {
            errors.push(format!(
                "spells[{index}].target must be present for {} spells",
                effect.family
            ));
            return;
        }

        if let Some(effect) = &spell.effect
            && requires_item_target(effect.family)
            && spell.target.is_none()
        {
            errors.push(format!(
                "spells[{index}].target must be present for {} spells",
                effect.family
            ));
            return;
        }

        if let Some(effect) = &spell.effect
            && effect.family == SpellEffectFamily::Summon
            && spell.target.is_none()
        {
            errors.push(format!(
                "spells[{index}].target must be present for summon spells"
            ));
            return;
        }

        let Some(target) = &spell.target else {
            if let Some(effect) = &spell.effect
                && matches!(
                    effect.family,
                    SpellEffectFamily::RaiseDead | SpellEffectFamily::TurnUndead
                )
            {
                errors.push(format!(
                    "spells[{index}].target.kind must be none for {} spells",
                    effect.family
                ));
            }
            return;
        };

        if target.kind != SpellTargetKind::Item && target.item_location.is_some() {
            errors.push(format!(
                "spells[{index}].target.item_location is only valid for item targets"
            ));
        }

        if let Some(effect) = &spell.effect {
            let expected = match effect.family {
                SpellEffectFamily::Banish | SpellEffectFamily::InstantDeath => {
                    Some(SpellTargetKind::Actor)
                }
                SpellEffectFamily::FallProtection
                | SpellEffectFamily::Speed
                | SpellEffectFamily::Vision => Some(SpellTargetKind::SelfTarget),
                SpellEffectFamily::RaiseDead | SpellEffectFamily::TurnUndead => {
                    Some(SpellTargetKind::None)
                }
                SpellEffectFamily::WaterBreathing => Some(SpellTargetKind::Actor),
                SpellEffectFamily::Concealment if effect.door_control.is_some() => {
                    Some(SpellTargetKind::Door)
                }
                SpellEffectFamily::Concealment => Some(SpellTargetKind::SelfTarget),
                _ => None,
            };
            if expected.is_some_and(|expected| target.kind != expected) {
                errors.push(format!(
                    "spells[{index}].target.kind must be {} for {} spells",
                    expected.expect("checked expected target").label(),
                    effect.family
                ));
            }
        }

        if target.kind == SpellTargetKind::SelfTarget {
            // A self target names the caster, and the direct-damage family
            // resolves against an actor target rather than the caster's own
            // identity. Authoring that pair would admit a cast whose execution
            // can only stub after consuming resources, so refuse the content
            // here. No self-damage mechanic is created by this refusal.
            if let Some(effect) = &spell.effect
                && effect.family == SpellEffectFamily::DirectDamage
            {
                errors.push(format!(
                    "spells[{index}].target.kind cannot be self for direct_damage spells"
                ));
            }
            if target.range.is_some() {
                errors.push(format!(
                    "spells[{index}].target.range is invalid for self target"
                ));
            }
            if target.requires_visible.is_some() {
                errors.push(format!(
                    "spells[{index}].target.requires_visible is invalid for self target"
                ));
            }
            if target.area.is_some() {
                errors.push(format!(
                    "spells[{index}].target.area is invalid for self target"
                ));
            }
        }

        if target.kind == SpellTargetKind::Area {
            match &target.area {
                Some(area) => {
                    if area.shape.trim().is_empty() {
                        errors.push(format!(
                            "spells[{index}].target.area.shape must be non-empty for area targets"
                        ));
                    }
                    match area.radius {
                        Some(radius) if radius > 0 => {}
                        Some(_) => errors.push(format!(
                            "spells[{index}].target.area.radius must be positive for area targets"
                        )),
                        None => errors.push(format!(
                            "spells[{index}].target.area.radius must be present for area targets"
                        )),
                    }
                }
                None if spell.catalog_entry.is_some() => {}
                None => {
                    errors.push(format!(
                        "spells[{index}].target.area.shape must be non-empty for area targets"
                    ));
                    errors.push(format!(
                        "spells[{index}].target.area.radius must be present for area targets"
                    ));
                }
            }
        }

        if spell
            .effect
            .as_ref()
            .is_some_and(|effect| requires_item_target(effect.family))
            && target.kind != SpellTargetKind::Item
        {
            errors.push(format!(
                "spells[{index}].target.kind must be item for {} spells",
                spell.effect.as_ref().unwrap().family
            ));
        }

        if let Some(effect) = &spell.effect {
            if effect.family == SpellEffectFamily::Summon
                && target.kind != SpellTargetKind::Coordinate
            {
                errors.push(format!(
                    "spells[{index}].target.kind must be coordinate for summon spells"
                ));
            }
            self.validate_bu_spell_target_family(index, effect, target, errors);
        }
    }

    fn validate_bu_spell_target_family(
        &self,
        index: usize,
        effect: &SpellEffectDef,
        target: &SpellTargetDef,
        errors: &mut Vec<String>,
    ) {
        match effect.family {
            SpellEffectFamily::DoorControl => {
                let action = effect
                    .door_control
                    .as_ref()
                    .map(|door| door.action.as_str());
                match action {
                    Some("open" | "close")
                        if target.kind != SpellTargetKind::Coordinate
                            && target.kind != SpellTargetKind::Door =>
                    {
                        errors.push(format!(
                            "spells[{index}].target.kind must be coordinate or door for door_control {} spells",
                            action.unwrap()
                        ));
                    }
                    Some("reveal_secret" | "hide_secret")
                        if target.kind != SpellTargetKind::None
                            && target.kind != SpellTargetKind::Coordinate =>
                    {
                        errors.push(format!(
                            "spells[{index}].target.kind must be none or coordinate for door_control {} spells",
                            action.unwrap()
                        ));
                    }
                    _ => {}
                }
            }
            SpellEffectFamily::SecretDetection => {
                if target.kind != SpellTargetKind::None
                    && target.kind != SpellTargetKind::Coordinate
                {
                    errors.push(format!(
                        "spells[{index}].target.kind must be none or coordinate for secret_detection spells"
                    ));
                }
            }
            SpellEffectFamily::ItemIdentify
            | SpellEffectFamily::ItemEnchant
            | SpellEffectFamily::WeaponEnchant => {
                if target.kind != SpellTargetKind::Item {
                    errors.push(format!(
                        "spells[{index}].target.kind must be item for {} spells",
                        effect.family
                    ));
                }
            }
            SpellEffectFamily::Locate => {
                if target.kind != SpellTargetKind::None {
                    errors.push(format!(
                        "spells[{index}].target.kind must be none for locate spells"
                    ));
                }
            }
            SpellEffectFamily::Portal => {
                if target.kind != SpellTargetKind::Coordinate {
                    errors.push(format!(
                        "spells[{index}].target.kind must be coordinate for portal spells"
                    ));
                }
            }
            SpellEffectFamily::Scry if target.kind != SpellTargetKind::None => {
                errors.push(format!(
                    "spells[{index}].target.kind must be none for scry spells"
                ));
            }
            _ => {}
        }
    }
}
