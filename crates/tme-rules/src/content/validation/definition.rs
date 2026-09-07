use super::*;

impl SelectedCatalog {
    pub fn validate_with_template(
        &self,
        template: &WorldTemplateV3,
    ) -> Result<(), ValidationError> {
        let policy = boundary_policy(self.clean_content, &self.research_boundary)?;
        let template_value = serde_json::to_value(template).map_err(|error| {
            ValidationError::new(vec![format!(
                "world_template could not be serialized for boundary validation: {error}"
            )])
        })?;
        scan_raw_documents(policy, [("world_template", &template_value)])?;

        let mut errors = Vec::new();
        world_template::validate_envelope(template, &mut errors);
        world_template::validate_template(
            template,
            &self.terrains,
            self.clean_content,
            &mut errors,
        );

        ValidationBundle::definition_only(self, template).validate_definition(errors)?;
        let context = world_seed::SourceWorldSeedValidationContext::new(self, template);
        for profile in &self.creation_profiles {
            let seed = profile.materialize(
                &crate::model::CharacterId::new("creation-validation"),
                profile.character.attributes.clone(),
                template.arrivals.get(&profile.arrival_id),
                self.actor_definitions.iter().any(|actor| {
                    actor.id == profile.actor_definition_id && actor.kind == ActorKind::Player
                }),
                |id| {
                    self.items
                        .iter()
                        .find(|item| item.id == id)
                        .and_then(|item| item.capability.as_ref())
                        .is_some_and(|capability| capability.spell_book_for.is_some())
                },
            )?;
            seed.validate_with_context(&context)?;
        }
        Ok(())
    }
}

impl WorldTemplateV3 {
    pub fn validate_with(&self, catalog: &SelectedCatalog) -> Result<(), ValidationError> {
        catalog.validate_with_template(self)
    }
}

impl WorldSeedDef {
    pub fn validate_with(
        &self,
        catalog: &SelectedCatalog,
        template: &WorldTemplateV3,
    ) -> Result<(), ValidationError> {
        catalog.validate_with_template(template)?;
        let context = world_seed::SourceWorldSeedValidationContext::new(catalog, template);
        self.validate_with_context(&context)
    }

    pub fn validate_with_context(
        &self,
        context: &impl WorldSeedValidationContext,
    ) -> Result<(), ValidationError> {
        let seed_value = serde_json::to_value(self).map_err(|error| {
            ValidationError::new(vec![format!(
                "simulation_seed could not be serialized for boundary validation: {error}"
            )])
        })?;
        scan_raw_documents(
            context.boundary_policy(),
            [("simulation_seed", &seed_value)],
        )?;
        world_seed::validate_world_seed(self, context)
    }
}
