use super::*;

impl PostgresState {
    pub async fn character_creation_options(
        &self,
        token: &str,
    ) -> Result<wire::CharacterCreationOptionsV1, SessionError> {
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        active_session(&mut tx, token, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        tx.commit().await.map_err(unavailable)?;
        let profiles = self
            .world
            .handle
            .creation_profiles()
            .await
            .map_err(|_| SessionError::Unavailable)?;
        let options = profiles
            .iter()
            .map(creation_option)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(wire::CharacterCreationOptionsV1 {
            control_api_version: wire::CONTROL_API_VERSION,
            options,
            maximum_characters: MAX_CHARACTERS_PER_ACCOUNT as u8,
        })
    }

    pub async fn create_character(
        self: &Arc<Self>,
        token: &str,
        request: wire::CharacterCreateRequestV1,
    ) -> Result<wire::CharacterCreatedV1, SessionError> {
        // Once admitted, finish the prepared lifecycle even if HTTP goes away.
        // The durable request receipt makes an uncertain response retryable.
        let state = Arc::clone(self);
        let token = token.to_string();
        tokio::spawn(async move { state.create_character_inner(&token, request).await })
            .await
            .map_err(unavailable)?
    }

    async fn create_character_inner(
        &self,
        token: &str,
        request: wire::CharacterCreateRequestV1,
    ) -> Result<wire::CharacterCreatedV1, SessionError> {
        let _transition = self.coordinator.transition().await;
        let request_bytes = serde_json::to_vec(&request.draft).map_err(unavailable)?;
        let fingerprint: [u8; 32] = Sha256::digest(request_bytes).into();
        let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
        let session = active_session(&mut tx, token, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        validate_csrf(session.csrf_digest, &request.csrf_token)?;
        if let Some(character) = creation_replay(
            &mut tx,
            session.account_id,
            request.request_id,
            &fingerprint,
        )
        .await?
        {
            tx.commit().await.map_err(unavailable)?;
            return Ok(created(character, wire::ReplayStatus::Replayed));
        }
        // Keep gameplay selection separate. Creation never revokes an existing character.
        let slot = available_slot(&mut tx, session.account_id, &request.draft.display_name).await?;
        tx.rollback().await.map_err(unavailable)?;
        let id = wire::CharacterId::new(Uuid::now_v7()).map_err(unavailable)?;
        let epoch = self
            .next_transfer_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(unavailable)?;
        let handle = &self.world.handle;
        let actor_id = handle
            .prepare_character_creation(
                epoch,
                CharacterId::new(id.to_string()),
                request.draft.clone(),
            )
            .await
            .map_err(|error| match error {
                crate::facet::FacetError::InvalidActor => SessionError::CharacterCreationRefused,
                _ => SessionError::Unavailable,
            })?;
        let durable = async {
            let checkpoint = handle.prepared_checkpoint(epoch).await.map_err(|_| SessionError::Unavailable)?;
            let mut tx = serializable(self.store.pool()).await.map_err(unavailable)?;
            Self::revalidate_exit_session(&mut tx, token, &session, &request.csrf_token, true).await?;
            let current_slot = available_slot(&mut tx, session.account_id, &request.draft.display_name).await?;
            if current_slot != slot { return Err(SessionError::CharacterCreationConflict); }
            sqlx::query("INSERT INTO tme.characters (character_id,account_id,slot,display_name,actor_id) VALUES ($1,$2,$3,$4,$5)")
                .bind(id.as_uuid()).bind(session.account_id.as_uuid()).bind(i16::from(slot))
                .bind(request.draft.display_name.as_str()).bind(actor_id.as_str())
                .execute(&mut *tx).await.map_err(unavailable)?;
            sqlx::query("INSERT INTO tme.character_creations (account_id,request_id,request_sha256,character_id) VALUES ($1,$2,$3,$4)")
                .bind(session.account_id.as_uuid()).bind(request.request_id.as_uuid())
                .bind(fingerprint.as_slice()).bind(id.as_uuid()).execute(&mut *tx).await.map_err(unavailable)?;
            Self::persist_prepared_checkpoint(&mut tx, &checkpoint).await?;
            self.commit_gameplay_transaction(tx).await.map_err(unavailable)
        }.await;
        if let Err(error) = durable {
            let _ = handle.rollback_transfer(epoch).await;
            return Err(error);
        }
        if handle.commit_transfer(epoch).await.is_err()
            || handle.publish_transfer(epoch).await.is_err()
        {
            self.ready.fail();
            return Err(SessionError::Unavailable);
        }
        Ok(created(
            wire::CharacterSummaryV1 {
                character_id: id,
                slot,
                display_name: request.draft.display_name,
            },
            wire::ReplayStatus::New,
        ))
    }
}

fn created(
    character: wire::CharacterSummaryV1,
    replay_status: wire::ReplayStatus,
) -> wire::CharacterCreatedV1 {
    wire::CharacterCreatedV1 {
        control_api_version: wire::CONTROL_API_VERSION,
        character,
        replay_status,
    }
}

async fn available_slot(
    tx: &mut Transaction<'_, Postgres>,
    account: wire::AccountId,
    name: &wire::DisplayName,
) -> Result<u8, SessionError> {
    sqlx::query("SELECT account_id FROM tme.accounts WHERE account_id=$1 FOR UPDATE")
        .bind(account.as_uuid())
        .fetch_one(&mut **tx)
        .await
        .map_err(unavailable)?;
    let rows = sqlx::query("SELECT slot,display_name FROM tme.characters WHERE account_id=$1")
        .bind(account.as_uuid())
        .fetch_all(&mut **tx)
        .await
        .map_err(unavailable)?;
    let mut used = BTreeSet::new();
    for row in rows {
        if row
            .try_get::<String, _>("display_name")
            .map_err(unavailable)?
            == name.as_str()
        {
            return Err(SessionError::CharacterCreationConflict);
        }
        used.insert(row.try_get::<i16, _>("slot").map_err(unavailable)?);
    }
    (1..=MAX_CHARACTERS_PER_ACCOUNT as u8)
        .find(|slot| !used.contains(&i16::from(*slot)))
        .ok_or(SessionError::CharacterCreationRefused)
}

async fn creation_replay(
    tx: &mut Transaction<'_, Postgres>,
    account: wire::AccountId,
    request: wire::CommandId,
    fingerprint: &[u8; 32],
) -> Result<Option<wire::CharacterSummaryV1>, SessionError> {
    let row = sqlx::query("SELECT r.request_sha256,c.character_id,c.slot,c.display_name FROM tme.character_creations r \
        JOIN tme.characters c ON c.character_id=r.character_id AND c.account_id=r.account_id \
        WHERE r.account_id=$1 AND r.request_id=$2")
        .bind(account.as_uuid()).bind(request.as_uuid()).fetch_optional(&mut **tx).await.map_err(unavailable)?;
    row.map(|row| {
        if row
            .try_get::<Vec<u8>, _>("request_sha256")
            .map_err(unavailable)?
            .as_slice()
            != fingerprint
        {
            return Err(SessionError::CharacterCreationConflict);
        }
        Ok(wire::CharacterSummaryV1 {
            character_id: wire::CharacterId::new(row.try_get("character_id").map_err(unavailable)?)
                .map_err(unavailable)?,
            slot: u8::try_from(row.try_get::<i16, _>("slot").map_err(unavailable)?)
                .map_err(unavailable)?,
            display_name: wire::DisplayName::new(
                row.try_get::<String, _>("display_name")
                    .map_err(unavailable)?,
            )
            .map_err(unavailable)?,
        })
    })
    .transpose()
}

fn attributes(value: &tme_rules::CharacterAttributes) -> wire::CharacterAttributes {
    wire::CharacterAttributes {
        strength: value.strength,
        dexterity: value.dexterity,
        constitution: value.constitution,
        intelligence: value.intelligence,
        wisdom: value.wisdom,
        charisma: value.charisma,
    }
}

fn creation_option(
    profile: &tme_rules::CharacterCreationProfileDef,
) -> Result<wire::CharacterCreationOptionV1, SessionError> {
    let bounds = &profile.attribute_bounds;
    Ok(wire::CharacterCreationOptionV1 {
        profile_id: wire::WireLabel::new(&profile.id).map_err(unavailable)?,
        class_name: wire::WireLabel::new(&profile.character.identity.display_class)
            .map_err(unavailable)?,
        nationality: wire::WireLabel::new(&profile.character.identity.nationality_id)
            .map_err(unavailable)?,
        minimum: wire::CharacterAttributes {
            strength: bounds.strength.inborn,
            dexterity: bounds.dexterity.inborn,
            constitution: bounds.constitution.inborn,
            intelligence: bounds.intelligence.inborn,
            wisdom: bounds.wisdom.inborn,
            charisma: bounds.charisma.inborn,
        },
        maximum: wire::CharacterAttributes {
            strength: bounds.strength.creation_cap,
            dexterity: bounds.dexterity.creation_cap,
            constitution: bounds.constitution.creation_cap,
            intelligence: bounds.intelligence.creation_cap,
            wisdom: bounds.wisdom.creation_cap,
            charisma: bounds.charisma.creation_cap,
        },
        suggested: attributes(&profile.character.attributes),
        attribute_points: profile.attribute_points,
    })
}
