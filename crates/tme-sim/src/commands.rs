use tme_rules::{
    CarriedGoldPosition, CarriedPosition, CharacterId, CorpseId, Direction, Engine,
    ExplicitTraversalKind, GoldMoveDestination, GoldMoveQuantity, GoldMoveSource, GoldPileId,
    ItemMoveDestination, PhysicalAttackMode, PlayerIntent, SpellTarget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParsedLine {
    Intent(PlayerIntent),
    CorpseSearch(CorpseSearchSelector),
    Meta(MetaCommand),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CorpseSearchSelector {
    Newest,
    PileIndex(usize),
    Exact(CorpseId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MetaCommand {
    Help,
    Quit,
}

pub(crate) fn parse_line(line: &str) -> Result<ParsedLine, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(ParsedLine::Empty);
    }

    let mut parts = trimmed.split_whitespace();
    let verb = parts
        .next()
        .expect("split_whitespace returned no verb")
        .to_ascii_lowercase();

    match verb.as_str() {
        "move" => {
            let direction = parts
                .next()
                .ok_or_else(|| "move requires a direction".to_string())?;
            if parts.next().is_some() {
                return Err("move accepts exactly one direction".to_string());
            }
            parse_direction(direction)
                .map(|direction| ParsedLine::Intent(PlayerIntent::MovePath(vec![direction])))
        }
        "run" | "sprint" => {
            let direction = parts
                .next()
                .ok_or_else(|| format!("{verb} requires a direction"))?;
            if parts.next().is_some() {
                return Err(format!("{verb} accepts exactly one direction"));
            }
            let count = if verb == "run" { 2 } else { 3 };
            parse_direction(direction)
                .map(|direction| ParsedLine::Intent(PlayerIntent::MovePath(vec![direction; count])))
        }
        "path" => {
            let directions = parts.map(parse_direction).collect::<Result<Vec<_>, _>>()?;
            if directions.is_empty() {
                return Err("path requires at least one direction".to_string());
            }
            if directions.len() > tme_rules::model::MAX_CONTROLLED_PATH_STEPS {
                return Err("path accepts at most three directions".to_string());
            }
            Ok(ParsedLine::Intent(PlayerIntent::MovePath(directions)))
        }
        "traverse" => {
            let kind = parts
                .next()
                .ok_or_else(|| "traverse requires a kind".to_string())?;
            if parts.next().is_some() {
                return Err("traverse accepts exactly one kind".to_string());
            }
            let kind = match kind.to_ascii_lowercase().as_str() {
                "stairs_up" => ExplicitTraversalKind::StairsUp,
                "stairs_down" => ExplicitTraversalKind::StairsDown,
                "climb_up" => ExplicitTraversalKind::ClimbUp,
                "climb_down" => ExplicitTraversalKind::ClimbDown,
                _ => return Err(format!("unknown traversal kind {kind:?}")),
            };
            Ok(ParsedLine::Intent(PlayerIntent::Traverse(kind)))
        }
        "hide" => {
            require_no_extra(parts, "hide")?;
            Ok(ParsedLine::Intent(PlayerIntent::Hide))
        }
        "nock" | "load" => {
            require_no_extra(parts, &verb)?;
            Ok(ParsedLine::Intent(PlayerIntent::Nock))
        }
        "unload" => {
            require_no_extra(parts, "unload")?;
            Ok(ParsedLine::Intent(PlayerIntent::UnloadBow))
        }
        "fight" | "kick" | "jumpkick" | "poke" | "shoot" | "throw" => {
            let mode = match verb.as_str() {
                "fight" => PhysicalAttackMode::Fight,
                "kick" => PhysicalAttackMode::Kick,
                "jumpkick" => PhysicalAttackMode::Jumpkick,
                "poke" => PhysicalAttackMode::Poke,
                "shoot" => PhysicalAttackMode::Shoot,
                "throw" => PhysicalAttackMode::Throw,
                _ => unreachable!("matched physical mode verb"),
            };
            parse_physical_target(trimmed, &verb).map(|(target_actor_id, authorization)| {
                ParsedLine::Intent(PlayerIntent::PhysicalAttack {
                    mode,
                    target_actor_id: target_actor_id.into(),
                    authorization,
                })
            })
        }
        "search" => parse_corpse_search(parts),
        "move_item" => {
            let item_instance_id = parts
                .next()
                .ok_or_else(|| "move_item requires an item instance id".to_string())?;
            if parts.next() != Some("to") {
                return Err("move_item requires 'to' before the destination".to_string());
            }
            let destination = parts
                .next()
                .ok_or_else(|| "move_item requires a destination".to_string())?;
            if parts.next().is_some() {
                return Err(
                    "move_item accepts exactly an item instance id and destination".to_string(),
                );
            }
            let destination = if destination == "ground_here" {
                ItemMoveDestination::GroundHere
            } else {
                let position = serde_json::from_value::<CarriedPosition>(
                    serde_json::Value::String(destination.to_string()),
                )
                .map_err(|_| format!("unknown carried position {destination:?}"))?;
                ItemMoveDestination::Carried { position }
            };
            Ok(ParsedLine::Intent(PlayerIntent::MoveItem {
                item_instance_id: item_instance_id.to_string(),
                destination,
            }))
        }
        "move_gold" => {
            let source = parts
                .next()
                .ok_or_else(|| "move_gold requires a source".to_string())?;
            if parts.next() != Some("to") {
                return Err("move_gold requires 'to' before the destination".to_string());
            }
            let destination = parts
                .next()
                .ok_or_else(|| "move_gold requires a destination".to_string())?;
            let quantity = parts
                .next()
                .ok_or_else(|| "move_gold requires all or a positive amount".to_string())?;
            if parts.next().is_some() {
                return Err(
                    "move_gold accepts exactly a source, destination, and quantity".to_string(),
                );
            }
            let source = match parse_carried_gold_position(source) {
                Ok(position) => GoldMoveSource::Carried { position },
                Err(_) => GoldMoveSource::Ground {
                    gold_pile_id: GoldPileId::parse(source)
                        .map_err(|_| format!("unknown gold source {source:?}"))?,
                },
            };
            let destination = if destination == "ground_here" {
                GoldMoveDestination::GroundHere
            } else {
                GoldMoveDestination::Carried {
                    position: parse_carried_gold_position(destination)?,
                }
            };
            let quantity = if quantity == "all" {
                GoldMoveQuantity::All
            } else {
                GoldMoveQuantity::Exact {
                    amount: parse_positive_i64(quantity, "move_gold amount")?,
                }
            };
            Ok(ParsedLine::Intent(PlayerIntent::MoveGold {
                source,
                destination,
                quantity,
            }))
        }
        "bank_deposit" => {
            let service_id = required_token(&mut parts, "bank_deposit requires a service id")?;
            let capability_id =
                required_token(&mut parts, "bank_deposit requires a capability id")?;
            let gold_pile_id = required_token(&mut parts, "bank_deposit requires a gold pile id")?;
            require_no_extra(parts, "bank_deposit")?;
            Ok(ParsedLine::Intent(PlayerIntent::DepositBankGold {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                gold_pile_id: GoldPileId::parse(gold_pile_id)
                    .map_err(|_| format!("invalid gold pile id {gold_pile_id:?}"))?,
            }))
        }
        "bank_withdraw" => {
            let service_id = required_token(&mut parts, "bank_withdraw requires a service id")?;
            let capability_id =
                required_token(&mut parts, "bank_withdraw requires a capability id")?;
            let amount = required_token(&mut parts, "bank_withdraw requires an amount")?;
            require_no_extra(parts, "bank_withdraw")?;
            Ok(ParsedLine::Intent(PlayerIntent::WithdrawBankGold {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                amount: parse_positive_i64(amount, "bank_withdraw amount")?,
            }))
        }
        "locker_deposit" => {
            let service_id = required_token(&mut parts, "locker_deposit requires a service id")?;
            let capability_id =
                required_token(&mut parts, "locker_deposit requires a capability id")?;
            let item_instance_id =
                required_token(&mut parts, "locker_deposit requires an item instance id")?;
            require_no_extra(parts, "locker_deposit")?;
            Ok(ParsedLine::Intent(PlayerIntent::DepositLockerItem {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                item_instance_id: item_instance_id.to_string(),
            }))
        }
        "locker_withdraw" => {
            let service_id = required_token(&mut parts, "locker_withdraw requires a service id")?;
            let capability_id =
                required_token(&mut parts, "locker_withdraw requires a capability id")?;
            let item_instance_id =
                required_token(&mut parts, "locker_withdraw requires an item instance id")?;
            let destination =
                required_token(&mut parts, "locker_withdraw requires a carried destination")?;
            require_no_extra(parts, "locker_withdraw")?;
            Ok(ParsedLine::Intent(PlayerIntent::WithdrawLockerItem {
                service_id: service_id.to_string(),
                capability_id: capability_id.to_string(),
                item_instance_id: item_instance_id.to_string(),
                destination: parse_carried_position(destination)?,
            }))
        }
        "offer_item" => {
            let recipient =
                required_token(&mut parts, "offer_item requires a recipient character id")?;
            let item_instance_id =
                required_token(&mut parts, "offer_item requires an item instance id")?;
            require_no_extra(parts, "offer_item")?;
            Ok(ParsedLine::Intent(PlayerIntent::OfferItem {
                recipient_character_id: parse_character_id(recipient)?,
                item_instance_id: item_instance_id.to_string(),
            }))
        }
        "accept_offer" => {
            let item_instance_id =
                required_token(&mut parts, "accept_offer requires an item instance id")?;
            let destination =
                required_token(&mut parts, "accept_offer requires a carried destination")?;
            require_no_extra(parts, "accept_offer")?;
            Ok(ParsedLine::Intent(PlayerIntent::AcceptItemOffer {
                item_instance_id: item_instance_id.to_string(),
                destination: parse_carried_position(destination)?,
            }))
        }
        "refuse_offer" => {
            let item_instance_id =
                required_token(&mut parts, "refuse_offer requires an item instance id")?;
            require_no_extra(parts, "refuse_offer")?;
            Ok(ParsedLine::Intent(PlayerIntent::RefuseItemOffer {
                item_instance_id: item_instance_id.to_string(),
            }))
        }
        "withdraw_offer" => {
            let item_instance_id =
                required_token(&mut parts, "withdraw_offer requires an item instance id")?;
            require_no_extra(parts, "withdraw_offer")?;
            Ok(ParsedLine::Intent(PlayerIntent::WithdrawItemOffer {
                item_instance_id: item_instance_id.to_string(),
            }))
        }
        "drink" => parse_name_argument(trimmed, "drink")
            .map(|item_instance_id| ParsedLine::Intent(PlayerIntent::Drink(item_instance_id))),
        "show" => {
            let next = parts.next();
            match next {
                None => Err("show requires sack".to_string()),
                Some(token) if !token.eq_ignore_ascii_case("sack") => {
                    Err("show accepts only sack".to_string())
                }
                Some(_) => {
                    if parts.next().is_some() {
                        Err("show sack accepts no extra arguments".to_string())
                    } else {
                        Ok(ParsedLine::Intent(PlayerIntent::ShowSack))
                    }
                }
            }
        }
        "wait" => {
            require_no_extra(parts, "wait")?;
            Ok(ParsedLine::Intent(PlayerIntent::Wait))
        }
        "rest" => {
            require_no_extra(parts, "rest")?;
            Ok(ParsedLine::Intent(PlayerIntent::Rest))
        }
        "open" => {
            let direction = parts
                .next()
                .ok_or_else(|| "open requires a direction".to_string())?;
            if parts.next().is_some() {
                return Err("open accepts exactly one direction".to_string());
            }
            parse_direction(direction)
                .map(|direction| ParsedLine::Intent(PlayerIntent::Open(direction)))
        }
        "close" => {
            let direction = parts
                .next()
                .ok_or_else(|| "close requires a direction".to_string())?;
            if parts.next().is_some() {
                return Err("close accepts exactly one direction".to_string());
            }
            parse_direction(direction)
                .map(|direction| ParsedLine::Intent(PlayerIntent::Close(direction)))
        }
        "inspect" => {
            require_no_extra(parts, "inspect")?;
            Ok(ParsedLine::Intent(PlayerIntent::Inspect))
        }
        "train" => {
            let service_id = parts
                .next()
                .ok_or_else(|| "train requires a service id and gold offer".to_string())?;
            let offered_gold = parts
                .next()
                .ok_or_else(|| "train requires a gold offer".to_string())?
                .parse::<i64>()
                .map_err(|_| "train gold offer must be a positive integer".to_string())?;
            if parts.next().is_some() {
                return Err("train accepts exactly a service id and gold offer".to_string());
            }
            if offered_gold <= 0 {
                return Err("train gold offer must be a positive integer".to_string());
            }
            Ok(ParsedLine::Intent(PlayerIntent::Train {
                service_id: service_id.to_string(),
                offered_gold,
            }))
        }
        "critique" => {
            let service_id = parts
                .next()
                .ok_or_else(|| "critique requires a service id and track id".to_string())?;
            let track_id = parts
                .next()
                .ok_or_else(|| "critique requires a track id".to_string())?;
            if parts.next().is_some() {
                return Err("critique accepts exactly a service id and track id".to_string());
            }
            Ok(ParsedLine::Intent(PlayerIntent::Critique {
                service_id: service_id.to_string(),
                track_id: track_id.to_string(),
            }))
        }
        "learn_spell" => {
            let spell_id = parts
                .next()
                .ok_or_else(|| format!("{verb} requires a spell name"))?;
            if parts.next().is_some() {
                return Err(format!("{verb} accepts exactly one spell name"));
            }
            Ok(ParsedLine::Intent(PlayerIntent::LearnSpell(
                spell_id.to_string(),
            )))
        }
        "cast" => {
            let first = parts
                .next()
                .ok_or_else(|| "cast requires a spell name or warmed".to_string())?;
            let (authorization, subject) = if first == "--unsafe" {
                (
                    tme_rules::HostilityAuthorization::ConfirmedUnsafe,
                    parts.next().ok_or_else(|| {
                        "cast --unsafe requires a spell name or warmed".to_string()
                    })?,
                )
            } else {
                (tme_rules::HostilityAuthorization::Safe, first)
            };
            let target = parse_spell_target_arguments(parts.collect())?;
            if subject.eq_ignore_ascii_case("warmed") {
                Ok(ParsedLine::Intent(PlayerIntent::CastWarmedSpell {
                    target,
                    authorization,
                }))
            } else {
                Ok(ParsedLine::Intent(PlayerIntent::CastSpell {
                    spell_id: subject.to_string(),
                    target,
                    authorization,
                }))
            }
        }
        "warm" => {
            let spell = parts
                .next()
                .ok_or_else(|| "warm requires a spell name".to_string())?;
            if parts.next().is_some() {
                return Err("warm accepts exactly one spell name".to_string());
            }
            Ok(ParsedLine::Intent(PlayerIntent::WarmSpell {
                spell_id: spell.to_string(),
            }))
        }
        "fizzle" => {
            require_no_extra(parts, "fizzle")?;
            Ok(ParsedLine::Intent(PlayerIntent::FizzleWarmedSpell))
        }
        "help" => {
            require_no_extra(parts, "help")?;
            Ok(ParsedLine::Meta(MetaCommand::Help))
        }
        "quit" => {
            require_no_extra(parts, "quit")?;
            Ok(ParsedLine::Meta(MetaCommand::Quit))
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_physical_target(
    line: &str,
    verb: &str,
) -> Result<(String, tme_rules::HostilityAuthorization), String> {
    let arguments = line.split_whitespace().skip(1).collect::<Vec<_>>();
    let unsafe_confirmed = arguments.first().is_some_and(|value| *value == "--unsafe");
    let target_start = usize::from(unsafe_confirmed);
    if arguments[target_start..].contains(&"--unsafe") {
        return Err(format!(
            "{verb} accepts --unsafe only before the target actor id"
        ));
    }
    let target_actor_id = arguments[target_start..].join(" ");
    if target_actor_id.is_empty() {
        return Err(format!("{verb} requires a target actor id"));
    }
    Ok((
        target_actor_id,
        if unsafe_confirmed {
            tme_rules::HostilityAuthorization::ConfirmedUnsafe
        } else {
            tme_rules::HostilityAuthorization::Safe
        },
    ))
}

fn parse_spell_target_arguments(arguments: Vec<&str>) -> Result<Option<SpellTarget>, String> {
    match arguments.as_slice() {
        [] => Ok(None),
        ["self"] => Ok(Some(SpellTarget::SelfTarget)),
        ["path"] => Err("cast path requires at least one direction".to_string()),
        ["path", directions @ ..] => Ok(Some(SpellTarget::Path {
            directions: directions
                .iter()
                .map(|direction| parse_direction(direction))
                .collect::<Result<Vec<_>, _>>()?,
        })),
        [actor_id] => Ok(Some(SpellTarget::Actor {
            actor_id: (*actor_id).into(),
        })),
        _ => Err("cast accepts one actor target or path followed by directions".to_string()),
    }
}

pub(crate) fn parse_line_for_engine(line: &str, engine: &Engine) -> Result<ParsedLine, String> {
    let actor_id = crate::session::scenario_player_actor_id(engine)?;
    match parse_line(line)? {
        ParsedLine::CorpseSearch(selector) => {
            let corpse_id = match selector {
                CorpseSearchSelector::Exact(corpse_id) => corpse_id,
                CorpseSearchSelector::Newest => engine
                    .actor_action_context(&actor_id)
                    .map_err(|error| error.to_string())?
                    .corpses_here
                    .first()
                    .map(|corpse| corpse.corpse_id.clone())
                    .ok_or_else(|| "no corpse is available here".to_string())?,
                CorpseSearchSelector::PileIndex(pile_index) => engine
                    .actor_action_context(&actor_id)
                    .map_err(|error| error.to_string())?
                    .corpses_here
                    .into_iter()
                    .find(|corpse| corpse.pile_index == pile_index)
                    .map(|corpse| corpse.corpse_id)
                    .ok_or_else(|| format!("no corpse at pile index {pile_index}"))?,
            };
            Ok(ParsedLine::Intent(PlayerIntent::SearchCorpse(corpse_id)))
        }
        parsed => Ok(parsed),
    }
}

fn parse_corpse_search<'a>(mut parts: impl Iterator<Item = &'a str>) -> Result<ParsedLine, String> {
    let first = parts
        .next()
        .ok_or_else(|| "search requires corpse, N corpse, or a canonical corpse id".to_string())?;
    let second = parts.next();
    if parts.next().is_some() {
        return Err("search accepts only corpse, N corpse, or a canonical corpse id".to_string());
    }

    match second {
        None if first.eq_ignore_ascii_case("corpse") => {
            Ok(ParsedLine::CorpseSearch(CorpseSearchSelector::Newest))
        }
        None => CorpseId::parse(first)
            .map(CorpseSearchSelector::Exact)
            .map(ParsedLine::CorpseSearch)
            .map_err(|_| format!("invalid corpse id {first:?}")),
        Some(second) if second.eq_ignore_ascii_case("corpse") => {
            if first.is_empty()
                || !first.bytes().all(|byte| byte.is_ascii_digit())
                || first.starts_with('0')
            {
                return Err(format!("invalid corpse pile index {first:?}"));
            }
            let pile_index = first
                .parse::<usize>()
                .map_err(|_| format!("invalid corpse pile index {first:?}"))?;
            Ok(ParsedLine::CorpseSearch(CorpseSearchSelector::PileIndex(
                pile_index,
            )))
        }
        Some(_) => {
            Err("search accepts only corpse, N corpse, or a canonical corpse id".to_string())
        }
    }
}

fn parse_direction(value: &str) -> Result<Direction, String> {
    match value.to_ascii_lowercase().as_str() {
        "n" | "north" => Ok(Direction::North),
        "ne" | "northeast" => Ok(Direction::Northeast),
        "e" | "east" => Ok(Direction::East),
        "se" | "southeast" => Ok(Direction::Southeast),
        "s" | "south" => Ok(Direction::South),
        "sw" | "southwest" => Ok(Direction::Southwest),
        "w" | "west" => Ok(Direction::West),
        "nw" | "northwest" => Ok(Direction::Northwest),
        other => Err(format!("unknown direction: {other}")),
    }
}

fn parse_carried_gold_position(value: &str) -> Result<CarriedGoldPosition, String> {
    match value {
        "left_hand" => Ok(CarriedGoldPosition::LeftHand),
        "right_hand" => Ok(CarriedGoldPosition::RightHand),
        "sack" => Ok(CarriedGoldPosition::Sack),
        _ => Err(format!("unknown carried gold position {value:?}")),
    }
}

fn parse_carried_position(value: &str) -> Result<CarriedPosition, String> {
    serde_json::from_value::<CarriedPosition>(serde_json::Value::String(value.to_string()))
        .map_err(|_| format!("unknown carried position {value:?}"))
}

fn parse_character_id(value: &str) -> Result<CharacterId, String> {
    serde_json::from_value::<CharacterId>(serde_json::Value::String(value.to_string()))
        .map_err(|_| format!("invalid character id {value:?}"))
}

fn parse_positive_i64(value: &str, label: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .ok()
        .filter(|amount| *amount > 0)
        .ok_or_else(|| format!("{label} must be a positive integer"))
}

fn required_token<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    message: &str,
) -> Result<&'a str, String> {
    parts.next().ok_or_else(|| message.to_string())
}

fn parse_name_argument(trimmed: &str, verb: &str) -> Result<String, String> {
    let name = trimmed[verb.len()..].trim();
    if name.is_empty() {
        Err(format!(
            "{verb} requires {}",
            if matches!(
                verb,
                "fight" | "kick" | "jumpkick" | "poke" | "shoot" | "throw"
            ) {
                "a target name"
            } else {
                "an item instance id"
            }
        ))
    } else {
        Ok(name.to_string())
    }
}

fn require_no_extra<'a>(
    mut parts: impl Iterator<Item = &'a str>,
    verb: &str,
) -> Result<(), String> {
    if parts.next().is_some() {
        Err(format!("{verb} accepts no arguments"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tme_rules::{Direction, PlayerIntent};

    fn fixture_engine(name: &str) -> Engine {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/test-corpus")
            .join(name);
        let loaded = crate::loading::load_simulation(&path).expect("fixture graph validates");
        Engine::new(loaded.world_seed, 7).expect("fixture starts")
    }

    fn stacked_corpse_engine() -> Engine {
        let mut engine = fixture_engine("death_corpse.json");
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::PhysicalAttack {
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "scavenger".into(),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect("first corpse");
        engine
            .apply_actor_intent(
                &tme_rules::ActorId::from("player"),
                PlayerIntent::PhysicalAttack {
                    mode: tme_rules::PhysicalAttackMode::Fight,
                    target_actor_id: "lookout".into(),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                },
            )
            .expect("second corpse");
        engine
    }

    #[test]
    fn parses_move_directions_with_short_and_long_names() {
        let cases = [
            ("move n", Direction::North),
            ("move NE", Direction::Northeast),
            ("MOVE east", Direction::East),
            ("move se", Direction::Southeast),
            ("move S", Direction::South),
            ("move southwest", Direction::Southwest),
            ("move w", Direction::West),
            ("move NORTHWEST", Direction::Northwest),
        ];

        for (line, direction) in cases {
            assert_eq!(
                parse_line(line).expect(line),
                ParsedLine::Intent(PlayerIntent::MovePath(vec![direction])),
                "{line}"
            );
        }
    }

    #[test]
    fn parses_path_with_multiple_directions() {
        assert_eq!(
            parse_line("path e northeast SW").expect("path should parse"),
            ParsedLine::Intent(PlayerIntent::MovePath(vec![
                Direction::East,
                Direction::Northeast,
                Direction::Southwest,
            ]))
        );
    }

    #[test]
    fn move_run_sprint_and_mixed_path_map_only_to_one_through_three_directions() {
        for (line, expected) in [
            ("move east", vec![Direction::East]),
            ("run east", vec![Direction::East, Direction::East]),
            (
                "sprint east",
                vec![Direction::East, Direction::East, Direction::East],
            ),
            (
                "path east south west",
                vec![Direction::East, Direction::South, Direction::West],
            ),
        ] {
            assert_eq!(
                parse_line(line).expect(line),
                ParsedLine::Intent(PlayerIntent::MovePath(expected)),
                "{line}"
            );
        }

        for (line, expected) in [
            ("move", "move requires a direction"),
            ("move east west", "move accepts exactly one direction"),
            ("run", "run requires a direction"),
            ("run east west", "run accepts exactly one direction"),
            ("sprint", "sprint requires a direction"),
            ("sprint east west", "sprint accepts exactly one direction"),
            ("path", "path requires at least one direction"),
            (
                "path east east east east",
                "path accepts at most three directions",
            ),
        ] {
            assert_eq!(parse_line(line).expect_err(line), expected, "{line}");
        }
    }

    #[test]
    fn parses_only_explicit_traversal_commands() {
        for (line, kind) in [
            ("traverse stairs_up", ExplicitTraversalKind::StairsUp),
            ("TRAVERSE STAIRS_DOWN", ExplicitTraversalKind::StairsDown),
            ("traverse climb_up", ExplicitTraversalKind::ClimbUp),
            ("traverse climb_down", ExplicitTraversalKind::ClimbDown),
        ] {
            assert_eq!(
                parse_line(line).expect(line),
                ParsedLine::Intent(PlayerIntent::Traverse(kind))
            );
        }
        assert_eq!(
            parse_line("traverse").expect_err("missing kind"),
            "traverse requires a kind"
        );
        assert_eq!(
            parse_line("traverse stairs_up now").expect_err("extra argument"),
            "traverse accepts exactly one kind"
        );
        assert!(parse_line("up").is_err());
        assert!(parse_line("use_stairs down").is_err());
    }

    #[test]
    fn parses_every_physical_mode_with_multi_word_names_case_sensitively() {
        for (verb, mode) in [
            ("fight", PhysicalAttackMode::Fight),
            ("kick", PhysicalAttackMode::Kick),
            ("jumpkick", PhysicalAttackMode::Jumpkick),
            ("poke", PhysicalAttackMode::Poke),
            ("shoot", PhysicalAttackMode::Shoot),
            ("throw", PhysicalAttackMode::Throw),
        ] {
            assert_eq!(
                parse_line(&format!("{verb} Ancient Ogre")).expect("physical mode should parse"),
                ParsedLine::Intent(PlayerIntent::PhysicalAttack {
                    mode,
                    target_actor_id: "Ancient Ogre".into(),
                    authorization: tme_rules::HostilityAuthorization::Safe,
                })
            );
            assert_eq!(
                parse_line(&format!("{verb} --unsafe Lawful Guard"))
                    .expect("unsafe physical mode should parse"),
                ParsedLine::Intent(PlayerIntent::PhysicalAttack {
                    mode,
                    target_actor_id: "Lawful Guard".into(),
                    authorization: tme_rules::HostilityAuthorization::ConfirmedUnsafe,
                })
            );
        }
        assert_eq!(
            parse_line("fight Lawful Guard --unsafe").expect_err("misplaced unsafe flag must fail"),
            "fight accepts --unsafe only before the target actor id"
        );
    }

    #[test]
    fn corpse_search_forms_resolve_newest_first_to_one_typed_id_intent() {
        let engine = stacked_corpse_engine();

        for (line, corpse_id) in [
            ("search corpse", "corpse:2"),
            ("search 2 corpse", "corpse:1"),
            ("search corpse:2", "corpse:2"),
        ] {
            assert_eq!(
                parse_line_for_engine(line, &engine).expect(line),
                ParsedLine::Intent(PlayerIntent::SearchCorpse(
                    CorpseId::parse(corpse_id).unwrap()
                )),
                "{line}"
            );
        }
    }

    #[test]
    fn corpse_search_rejects_missing_out_of_range_and_extra_tokens() {
        let engine = stacked_corpse_engine();
        for line in [
            "search",
            "search 0 corpse",
            "search 03 corpse",
            "search corpse:0",
            "search corpse:01",
            "search corpse extra",
            "search 1 corpse extra",
        ] {
            assert!(parse_line_for_engine(line, &engine).is_err(), "{line}");
        }
        assert_eq!(
            parse_line_for_engine("search 3 corpse", &engine).unwrap_err(),
            "no corpse at pile index 3"
        );

        let empty = fixture_engine("first_room.json");
        assert_eq!(
            parse_line_for_engine("search corpse", &empty).unwrap_err(),
            "no corpse is available here"
        );
    }

    #[test]
    fn parses_wait_inspect_help_quit_and_empty_lines() {
        assert_eq!(
            parse_line("wait").expect("wait should parse"),
            ParsedLine::Intent(PlayerIntent::Wait)
        );
        assert_eq!(
            parse_line(" INSPECT ").expect("inspect should parse"),
            ParsedLine::Intent(PlayerIntent::Inspect)
        );
        assert_eq!(
            parse_line("help").expect("help should parse"),
            ParsedLine::Meta(MetaCommand::Help)
        );
        assert_eq!(
            parse_line("QUIT").expect("quit should parse"),
            ParsedLine::Meta(MetaCommand::Quit)
        );
        assert_eq!(
            parse_line("   ").expect("empty should parse"),
            ParsedLine::Empty
        );
    }

    #[test]
    fn parses_item_move_commands() {
        assert_eq!(
            parse_line("move_item hemp_rope to sack_item_1").expect("carried move should parse"),
            ParsedLine::Intent(PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: ItemMoveDestination::Carried {
                    position: CarriedPosition::SackItem1,
                },
            })
        );
        assert_eq!(
            parse_line("move_item hemp_rope to ground_here").expect("ground move should parse"),
            ParsedLine::Intent(PlayerIntent::MoveItem {
                item_instance_id: "hemp_rope".to_string(),
                destination: ItemMoveDestination::GroundHere,
            })
        );
        assert_eq!(
            parse_line("show sack").expect("show sack should parse"),
            ParsedLine::Intent(PlayerIntent::ShowSack)
        );
        assert_eq!(
            parse_line("SHOW SACK").expect("SHOW SACK should parse"),
            ParsedLine::Intent(PlayerIntent::ShowSack)
        );
    }

    #[test]
    fn parses_every_positioned_gold_storage_and_offer_command() {
        assert_eq!(
            parse_line("move_gold sack to left_hand 30").unwrap(),
            ParsedLine::Intent(PlayerIntent::MoveGold {
                source: GoldMoveSource::Carried {
                    position: CarriedGoldPosition::Sack,
                },
                destination: GoldMoveDestination::Carried {
                    position: CarriedGoldPosition::LeftHand,
                },
                quantity: GoldMoveQuantity::Exact { amount: 30 },
            })
        );
        assert_eq!(
            parse_line("move_gold gold:2 to sack all").unwrap(),
            ParsedLine::Intent(PlayerIntent::MoveGold {
                source: GoldMoveSource::Ground {
                    gold_pile_id: GoldPileId::parse("gold:2").unwrap(),
                },
                destination: GoldMoveDestination::Carried {
                    position: CarriedGoldPosition::Sack,
                },
                quantity: GoldMoveQuantity::All,
            })
        );
        assert_eq!(
            parse_line("bank_deposit west_counter bank_access gold:1").unwrap(),
            ParsedLine::Intent(PlayerIntent::DepositBankGold {
                service_id: "west_counter".to_string(),
                capability_id: "bank_access".to_string(),
                gold_pile_id: GoldPileId::parse("gold:1").unwrap(),
            })
        );
        assert_eq!(
            parse_line("bank_withdraw east_counter bank_access 18").unwrap(),
            ParsedLine::Intent(PlayerIntent::WithdrawBankGold {
                service_id: "east_counter".to_string(),
                capability_id: "bank_access".to_string(),
                amount: 18,
            })
        );
        assert_eq!(
            parse_line("locker_deposit west_counter locker_access field_case").unwrap(),
            ParsedLine::Intent(PlayerIntent::DepositLockerItem {
                service_id: "west_counter".to_string(),
                capability_id: "locker_access".to_string(),
                item_instance_id: "field_case".to_string(),
            })
        );
        assert_eq!(
            parse_line("locker_withdraw east_counter locker_access field_case sack_item_1")
                .unwrap(),
            ParsedLine::Intent(PlayerIntent::WithdrawLockerItem {
                service_id: "east_counter".to_string(),
                capability_id: "locker_access".to_string(),
                item_instance_id: "field_case".to_string(),
                destination: CarriedPosition::SackItem1,
            })
        );
        assert_eq!(
            parse_line("offer_item character:party:recipient field_case").unwrap(),
            ParsedLine::Intent(PlayerIntent::OfferItem {
                recipient_character_id: parse_character_id("character:party:recipient").unwrap(),
                item_instance_id: "field_case".to_string(),
            })
        );
        assert_eq!(
            parse_line("accept_offer field_case left_hand").unwrap(),
            ParsedLine::Intent(PlayerIntent::AcceptItemOffer {
                item_instance_id: "field_case".to_string(),
                destination: CarriedPosition::LeftHand,
            })
        );
        assert_eq!(
            parse_line("refuse_offer field_case").unwrap(),
            ParsedLine::Intent(PlayerIntent::RefuseItemOffer {
                item_instance_id: "field_case".to_string(),
            })
        );
        assert_eq!(
            parse_line("withdraw_offer field_case").unwrap(),
            ParsedLine::Intent(PlayerIntent::WithdrawItemOffer {
                item_instance_id: "field_case".to_string(),
            })
        );

        for malformed in [
            "move_gold",
            "move_gold sack left_hand 1",
            "move_gold sack to nowhere 1",
            "move_gold gold:0 to sack all",
            "move_gold sack to left_hand 0",
            "bank_deposit west_counter bank_access gold:0",
            "bank_withdraw east_counter bank_access -1",
            "locker_deposit west_counter locker_access",
            "locker_withdraw east_counter locker_access field_case nowhere",
            "offer_item character:party:recipient",
            "accept_offer field_case nowhere",
            "refuse_offer field_case extra",
            "withdraw_offer",
        ] {
            assert!(parse_line(malformed).is_err(), "{malformed}");
        }
    }

    #[test]
    fn rejects_removed_item_verbs() {
        for verb in ["take", "retrieve", "drop", "equip", "unequip"] {
            assert_eq!(
                parse_line(&format!("{verb} old_item"))
                    .expect_err("removed item verb must be rejected"),
                format!("unknown command: {verb}")
            );
        }
    }

    #[test]
    fn parses_open_and_close_commands() {
        assert_eq!(
            parse_line("open east").expect("open should parse"),
            ParsedLine::Intent(PlayerIntent::Open(Direction::East))
        );
        assert_eq!(
            parse_line("close n").expect("close should parse"),
            ParsedLine::Intent(PlayerIntent::Close(Direction::North))
        );
        assert_eq!(
            parse_line("OPEN W").expect("OPEN should parse"),
            ParsedLine::Intent(PlayerIntent::Open(Direction::West))
        );
    }

    #[test]
    fn rejects_open_close_without_direction() {
        assert!(parse_line("open").is_err());
        assert!(parse_line("close").is_err());
        assert!(parse_line("open east north").is_err());
        assert!(parse_line("close w s").is_err());
    }

    #[test]
    fn parses_drink_commands() {
        assert_eq!(
            parse_line("drink healing_balm").expect("drink should parse"),
            ParsedLine::Intent(PlayerIntent::Drink("healing_balm".to_string()))
        );
        assert_eq!(
            parse_line("drink Healing Balm").expect("drink multi-word should parse"),
            ParsedLine::Intent(PlayerIntent::Drink("Healing Balm".to_string()))
        );
    }

    #[test]
    fn learn_alias_is_not_a_command() {
        assert_eq!(
            parse_line("learn spark"),
            Err("unknown command: learn".to_string())
        );
    }

    #[test]
    fn parses_learn_spell_commands() {
        assert_eq!(
            parse_line("learn_spell spark").expect("learn_spell should parse"),
            ParsedLine::Intent(PlayerIntent::LearnSpell("spark".to_string()))
        );
    }

    #[test]
    fn parses_training_and_critique_commands_with_exact_shapes() {
        assert_eq!(
            parse_line("train west_mentor 21").expect("train should parse"),
            ParsedLine::Intent(PlayerIntent::Train {
                service_id: "west_mentor".to_string(),
                offered_gold: 21,
            })
        );
        assert_eq!(
            parse_line("critique west_mentor mace").expect("critique should parse"),
            ParsedLine::Intent(PlayerIntent::Critique {
                service_id: "west_mentor".to_string(),
                track_id: "mace".to_string(),
            })
        );

        for line in [
            "train",
            "train west_mentor",
            "train west_mentor 0",
            "train west_mentor -1",
            "train west_mentor coins",
            "train west_mentor 9223372036854775808",
            "train west_mentor 7 extra",
            "critique",
            "critique west_mentor",
            "critique west_mentor mace extra",
        ] {
            assert!(parse_line(line).is_err(), "{line}");
        }
    }

    #[test]
    fn parses_hide_command() {
        assert_eq!(
            parse_line("hide").expect("hide should parse"),
            ParsedLine::Intent(PlayerIntent::Hide)
        );
        assert_eq!(
            parse_line("HIDE").expect("HIDE should parse"),
            ParsedLine::Intent(PlayerIntent::Hide)
        );
        assert_eq!(
            parse_line("hide now").expect_err("hide takes no args"),
            "hide accepts no arguments"
        );
    }

    #[test]
    fn reports_specific_parse_errors() {
        let cases = [
            ("move", "move requires a direction"),
            ("move upward", "unknown direction: upward"),
            ("path", "path requires at least one direction"),
            ("path e sideways", "unknown direction: sideways"),
            ("fight", "fight requires a target actor id"),
            ("kick", "kick requires a target actor id"),
            ("jumpkick", "jumpkick requires a target actor id"),
            ("poke", "poke requires a target actor id"),
            ("shoot", "shoot requires a target actor id"),
            ("throw", "throw requires a target actor id"),
            ("attack", "unknown command: attack"),
            ("move_item", "move_item requires an item instance id"),
            (
                "move_item rope",
                "move_item requires 'to' before the destination",
            ),
            ("move_item rope to", "move_item requires a destination"),
            (
                "move_item rope to nowhere",
                "unknown carried position \"nowhere\"",
            ),
            ("drink", "drink requires an item instance id"),
            ("show", "show requires sack"),
            ("show bag", "show accepts only sack"),
            ("show sack now", "show sack accepts no extra arguments"),
            ("open", "open requires a direction"),
            ("open east north", "open accepts exactly one direction"),
            ("close", "close requires a direction"),
            ("close w s", "close accepts exactly one direction"),
            ("learn_spell", "learn_spell requires a spell name"),
            (
                "learn_spell spark extra",
                "learn_spell accepts exactly one spell name",
            ),
            ("dance", "unknown command: dance"),
        ];

        for (line, expected) in cases {
            let error = parse_line(line).expect_err(line);
            assert_eq!(error, expected, "{line}");
        }
    }

    #[test]
    fn parses_typed_direct_warm_cast_fizzle_and_rest_commands() {
        assert_eq!(
            parse_line("cast spark watcher").expect("cast should parse"),
            ParsedLine::Intent(PlayerIntent::CastSpell {
                spell_id: "spark".to_string(),
                target: Some(SpellTarget::Actor {
                    actor_id: "watcher".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            })
        );
        assert_eq!(
            parse_line("cast ward").expect("cast should parse"),
            ParsedLine::Intent(PlayerIntent::CastSpell {
                spell_id: "ward".to_string(),
                target: None,
                authorization: tme_rules::HostilityAuthorization::Safe,
            })
        );
        assert_eq!(
            parse_line("warm charged_spark").expect("warm should parse"),
            ParsedLine::Intent(PlayerIntent::WarmSpell {
                spell_id: "charged_spark".to_string(),
            })
        );
        assert_eq!(
            parse_line("cast warmed watcher").expect("warmed cast should parse"),
            ParsedLine::Intent(PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Actor {
                    actor_id: "watcher".into(),
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            })
        );
        assert_eq!(
            parse_line("cast warmed path north east").expect("path should parse"),
            ParsedLine::Intent(PlayerIntent::CastWarmedSpell {
                target: Some(SpellTarget::Path {
                    directions: vec![Direction::North, Direction::East],
                }),
                authorization: tme_rules::HostilityAuthorization::Safe,
            })
        );
        assert_eq!(
            parse_line("fizzle").expect("fizzle should parse"),
            ParsedLine::Intent(PlayerIntent::FizzleWarmedSpell)
        );
        assert_eq!(
            parse_line("rest").expect("rest should parse"),
            ParsedLine::Intent(PlayerIntent::Rest)
        );
        assert_eq!(
            parse_line("prepare charged_spark").expect_err("old command"),
            "unknown command: prepare"
        );
        assert_eq!(
            parse_line("release charged_spark").expect_err("old command"),
            "unknown command: release"
        );
    }
}
