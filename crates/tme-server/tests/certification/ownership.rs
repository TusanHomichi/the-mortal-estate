// Private ownership evidence for the EV certification target.
async fn facet_baseline(pool: &sqlx::PgPool, facet_id: wire::FacetId) -> FacetBaseline {
    let row = sqlx::query(
        "SELECT checkpoint_bytes,checkpoint_sha256,content_digest FROM tme.facets WHERE facet_id=$1",
    )
    .bind(facet_id.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    assert_eq!(
        Sha256::digest(&checkpoint).as_slice(),
        row.get::<Vec<u8>, _>("checkpoint_sha256")
    );
    FacetBaseline {
        ownership: checkpoint_ownership(&checkpoint),
        checkpoint,
        content_digest: row.get("content_digest"),
    }
}

fn checkpoint_ownership(checkpoint: &[u8]) -> CheckpointOwnership {
    let payload: serde_json::Value = serde_json::from_slice(checkpoint).unwrap();
    let actors = payload["world"]["actors"]
        .as_array()
        .expect("checkpoint world actors");
    let actor_ids = actors
        .iter()
        .map(|actor| {
            actor["id"]
                .as_str()
                .expect("checkpoint actor ID")
                .to_string()
        })
        .collect();
    let character_ids = actors
        .iter()
        .filter_map(|actor| actor["character_id"].as_str().map(str::to_string))
        .collect();
    let mut item_ids = payload["world"]["item_instances"]
        .as_object()
        .expect("checkpoint item instances")
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    collect_string_fields(&payload, "item_instance_id", &mut item_ids);
    let mut item_state = Vec::new();
    collect_item_state(&payload, "$", &mut item_state);
    CheckpointOwnership {
        actor_ids,
        character_ids,
        item_ids,
        item_state,
    }
}

fn collect_string_fields(
    value: &serde_json::Value,
    selected_key: &str,
    result: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                if key == selected_key
                    && let Some(value) = value.as_str()
                {
                    result.insert(value.to_string());
                }
                collect_string_fields(value, selected_key, result);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_string_fields(value, selected_key, result);
            }
        }
        _ => {}
    }
}

fn collect_item_state(
    value: &serde_json::Value,
    path: &str,
    result: &mut Vec<(String, serde_json::Value)>,
) {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                let child_path = format!("{path}/{key}");
                if key.contains("item") {
                    result.push((child_path, value.clone()));
                } else {
                    collect_item_state(value, &child_path, result);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                collect_item_state(value, &format!("{path}/{index}"), result);
            }
        }
        _ => {}
    }
}

fn assert_observer_ownership(clients: &[support::Client], baseline: &FacetBaseline) {
    for client in clients {
        // D4: the wire carries no world identity at all, so an observer must
        // never see one. An empty set here is the regression guard: if a world
        // id reappears in any envelope field named *facet_id*, this fails.
        assert!(
            client.observed_facet_ids.is_empty(),
            "an envelope leaked a world identity to an observer: {:?}",
            client.observed_facet_ids
        );
        let ownership = &baseline.ownership;
        assert!(
            client.observed_actor_ids.is_subset(&ownership.actor_ids),
            "observer received an actor absent from its facet checkpoint"
        );
        assert!(
            client
                .observed_character_ids
                .is_subset(&ownership.character_ids),
            "observer received a character absent from its facet checkpoint"
        );
        assert!(
            client.observed_item_ids.is_subset(&ownership.item_ids),
            "observer received an item absent from its facet checkpoint"
        );
    }
}

async fn assert_packet_rows(
    pool: &sqlx::PgPool,
    fixtures: &[CharacterFixture],
    request_digests: &BTreeMap<Uuid, [u8; 32]>,
) {
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(256, total);
    assert_eq!(256, request_digests.len());
    let digest_rows = sqlx::query("SELECT command_id,request_digest FROM tme.command_receipts")
        .fetch_all(pool)
        .await
        .unwrap();
    assert_eq!(256, digest_rows.len());
    for row in digest_rows {
        let command_id: Uuid = row.get("command_id");
        assert_eq!(
            request_digests[&command_id].as_slice(),
            row.get::<Vec<u8>, _>("request_digest"),
            "durable receipt request digest was not canonical"
        );
    }
    let accepted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE disposition='accepted' AND after_revision=before_revision+1",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let rejected: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts WHERE disposition='rejected' AND after_revision=before_revision",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(192, accepted);
    assert_eq!(64, rejected);
    let world_count: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.command_receipts")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(256, world_count);
    let distinct: i64 =
        sqlx::query_scalar("SELECT count(DISTINCT server_sequence) FROM tme.command_receipts")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(256, distinct);
    let crossed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tme.command_receipts r JOIN tme.characters c USING(account_id) \
         WHERE r.actor_id<>c.actor_id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(0, crossed);
    for fixture in fixtures {
        let rows = sqlx::query(
            "SELECT client_sequence,disposition FROM tme.command_receipts WHERE account_id=$1 ORDER BY server_sequence",
        )
        .bind(fixture.account_id.as_uuid())
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(32, rows.len());
        for (index, row) in rows.iter().enumerate() {
            let expected_sequence = if index < 24 { index as i64 + 1 } else { 25 };
            assert_eq!(expected_sequence, row.get::<i64, _>("client_sequence"));
            assert_eq!(
                if index < 24 { "accepted" } else { "rejected" },
                row.get::<String, _>("disposition")
            );
        }
    }
}

async fn assert_final_checkpoints(
    pool: &sqlx::PgPool,
    world_facet: wire::FacetId,
    baseline: &FacetBaseline,
    fixtures: &[CharacterFixture],
) {
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM tme.facets")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(1, rows, "the durable store grew a second world");
    let row = sqlx::query(
        "SELECT checkpoint_bytes,checkpoint_sha256,content_digest FROM tme.facets WHERE facet_id=$1",
    )
    .bind(world_facet.as_uuid())
    .fetch_one(pool)
    .await
    .unwrap();
    let checkpoint: Vec<u8> = row.get("checkpoint_bytes");
    assert_ne!(baseline.checkpoint, checkpoint);
    assert_eq!(
        baseline.content_digest,
        row.get::<Vec<u8>, _>("content_digest")
    );
    assert_eq!(
        Sha256::digest(&checkpoint).as_slice(),
        row.get::<Vec<u8>, _>("checkpoint_sha256")
    );
    let final_ownership = checkpoint_ownership(&checkpoint);
    assert_eq!(
        baseline.ownership.item_state, final_ownership.item_state,
        "toggle/wait work changed world-owned item state"
    );
    let own_actor_ids = fixtures
        .iter()
        .map(|fixture| fixture.actor_id.as_str().to_string())
        .collect::<BTreeSet<_>>();
    let own_character_ids = fixtures
        .iter()
        .map(|fixture| fixture.character_id.as_uuid().to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(8, own_actor_ids.len());
    assert_eq!(8, own_character_ids.len());
    assert!(own_actor_ids.is_subset(&final_ownership.actor_ids));
    assert!(own_character_ids.is_subset(&final_ownership.character_ids));
}
