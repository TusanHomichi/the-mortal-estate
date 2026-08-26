use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use tme_protocol as wire;
use tme_rules::content::{
    ActorSeedDef, CatalogProfileKey, CatalogV6, EcologySiteDef, GroundItemSeedDef,
    ItemInstanceSeedDef, MerchantInventorySeedDef, ServiceInstanceSeedDef, WorldSeedDef,
    WorldTemplateV3,
};
use tme_rules::{ActorId, Engine, GameDefinition, ValidatedWorldSeed};

use crate::{PostgresBootstrap, PostgresCharacterBootstrap, PostgresWorldBootstrap};

const PRODUCTION_BOOTSTRAP_SCHEMA_VERSION: u32 = 1;
const SIMULATION_SEED_SCHEMA_VERSION: u32 = 3;
const MAX_BOOTSTRAP_BYTES: u64 = 256 * 1024;
const MAX_CONTENT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionBootstrapManifestV1 {
    schema_version: u32,
    catalog: PathBuf,
    catalog_profile: CatalogProfileKey,
    world_template: PathBuf,
    world: ProductionWorldV1,
    characters: Vec<ProductionCharacterV1>,
}

/// D4: the manifest declares the one canonical world this process serves.
/// There is no family of copies and no arrival gate for switching between them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionWorldV1 {
    facet_id: wire::FacetId,
    key: String,
    simulation_seed: PathBuf,
    rng_seed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionCharacterV1 {
    account_id: wire::AccountId,
    character_id: wire::CharacterId,
    slot: u8,
    display_name: wire::DisplayName,
    actor_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SimulationSeedEnvelopeV3 {
    schema_version: u32,
    kind: String,
    id: String,
    actors: Vec<ActorSeedDef>,
    item_instances: BTreeMap<String, ItemInstanceSeedDef>,
    ground_items: Vec<GroundItemSeedDef>,
    service_instances: Vec<ServiceInstanceSeedDef>,
    merchant_inventories: Vec<MerchantInventorySeedDef>,
    ecology_sites: Vec<EcologySiteDef>,
}

impl SimulationSeedEnvelopeV3 {
    fn into_world_seed(self) -> Result<WorldSeedDef, String> {
        if self.schema_version != SIMULATION_SEED_SCHEMA_VERSION
            || self.kind != "simulation_seed"
            || self.id.trim().is_empty()
        {
            return Err("production simulation seed envelope is not current".to_string());
        }
        Ok(WorldSeedDef {
            actors: self.actors,
            item_instances: self.item_instances,
            ground_items: self.ground_items,
            service_instances: self.service_instances,
            merchant_inventories: self.merchant_inventories,
            ecology_sites: self.ecology_sites,
        })
    }
}

pub fn load_bootstrap(path: &Path) -> Result<PostgresBootstrap, String> {
    let manifest_path = canonical_regular_file(path)?;
    let manifest: ProductionBootstrapManifestV1 = read_json(&manifest_path, MAX_BOOTSTRAP_BYTES)?;
    if manifest.schema_version != PRODUCTION_BOOTSTRAP_SCHEMA_VERSION {
        return Err("production bootstrap manifest schema is not current".to_string());
    }
    let base = manifest_path
        .parent()
        .ok_or_else(|| "production bootstrap manifest has no parent".to_string())?;
    let catalog_path = resolve_reference(base, &manifest.catalog)?;
    let template_path = resolve_reference(base, &manifest.world_template)?;
    let catalog: CatalogV6 = read_json(&catalog_path, MAX_CONTENT_BYTES)?;
    let template: WorldTemplateV3 = read_json(&template_path, MAX_CONTENT_BYTES)?;
    if !catalog.clean_content || catalog.research_boundary.status != "clean_original_fixture" {
        return Err("production catalog is not clean runtime content".to_string());
    }
    let definition = GameDefinition::from_content(catalog, manifest.catalog_profile, template)
        .map_err(|error| error.to_string())?;

    let seed_path = resolve_reference(base, &manifest.world.simulation_seed)?;
    let seed: SimulationSeedEnvelopeV3 = read_json(&seed_path, MAX_CONTENT_BYTES)?;
    let validated = ValidatedWorldSeed::new(definition.clone(), seed.into_world_seed()?)
        .map_err(|error| error.to_string())?;
    let engine =
        Engine::new(validated, manifest.world.rng_seed).map_err(|error| error.to_string())?;
    let world = PostgresWorldBootstrap {
        facet_id: manifest.world.facet_id,
        key: manifest.world.key,
        engine,
    };
    let characters = manifest
        .characters
        .into_iter()
        .map(|character| PostgresCharacterBootstrap {
            account_id: character.account_id,
            character_id: character.character_id,
            slot: character.slot,
            display_name: character.display_name,
            actor_id: ActorId::new(character.actor_id),
        })
        .collect();
    Ok(PostgresBootstrap { world, characters })
}

fn resolve_reference(base: &Path, reference: &Path) -> Result<PathBuf, String> {
    if reference.as_os_str().is_empty() {
        return Err("production bootstrap contains an empty path".to_string());
    }
    let path = if reference.is_absolute() {
        reference.to_path_buf()
    } else {
        base.join(reference)
    };
    canonical_regular_file(&path)
}

fn canonical_regular_file(path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("production input is unavailable: {error}"))?;
    let metadata = canonical
        .metadata()
        .map_err(|error| format!("production input metadata is unavailable: {error}"))?;
    if !metadata.is_file() {
        return Err("production input is not a regular file".to_string());
    }
    Ok(canonical)
}

fn read_json<T: DeserializeOwned>(path: &Path, maximum: u64) -> Result<T, String> {
    let metadata = path
        .metadata()
        .map_err(|error| format!("production input metadata is unavailable: {error}"))?;
    if metadata.len() == 0 || metadata.len() > maximum {
        return Err("production input size is out of bounds".to_string());
    }
    let bytes = std::fs::read(path)
        .map_err(|error| format!("production input could not be read: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("production input is invalid: {error}"))
}

pub fn read_systemd_credential(name: &str) -> Result<String, String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
    {
        return Err("credential name is invalid".to_string());
    }
    let directory = std::env::var_os("CREDENTIALS_DIRECTORY")
        .ok_or_else(|| "systemd credential directory is unavailable".to_string())?;
    let path = PathBuf::from(directory).join(name);
    let metadata = path
        .metadata()
        .map_err(|_| "required systemd credential is unavailable".to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 8 * 1024 {
        return Err("required systemd credential size is invalid".to_string());
    }
    let mut value = std::fs::read_to_string(path)
        .map_err(|_| "required systemd credential is unreadable".to_string())?;
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    if value.is_empty() || value.contains(['\n', '\r', '\0']) {
        return Err("required systemd credential content is invalid".to_string());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgConnectOptions;
    use std::fs;
    use std::str::FromStr;
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn create() -> Self {
            let path =
                std::env::temp_dir().join(format!("tme-production-loader-{}", Uuid::now_v7()));
            fs::create_dir(&path).expect("create production loader test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn content_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
            .canonicalize()
            .expect("canonical test content path")
    }

    #[test]
    fn production_socket_credential_shape_is_accepted_by_sqlx() {
        let url = "postgresql://tme_runtime:fixture@%2Fvar%2Frun%2Fpostgresql/tme";
        assert!(PgConnectOptions::from_str(url).is_ok());
    }

    fn write_manifest(directory: &Path, schema_version: u32) -> PathBuf {
        let manifest = serde_json::json!({
            "schema_version": schema_version,
            "catalog": content_path("content/test-corpus/catalogs/prototype_catalog_v6.json"),
            "catalog_profile": "profile/world_topology_gallery",
            "world_template": content_path("content/test-corpus/world_templates/world_topology_gallery.json"),
            "world": {
                "facet_id": "018f4d9e-8d57-7a1c-9d1a-8cb840d86db1",
                "key": "world-topology-primary",
                "simulation_seed": content_path("content/test-corpus/simulation_seeds/world_topology_gallery.json"),
                "rng_seed": 7
            },
            "characters": [{
                "account_id": "018f4d9e-8d57-7a1c-9d1a-8cb840d86db2",
                "character_id": "018f4d9e-8d57-7a1c-9d1a-8cb840d86db3",
                "slot": 1,
                "display_name": "Wayfarer",
                "actor_id": "player"
            }]
        });
        let path = directory.join("bootstrap.json");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&manifest).expect("encode production manifest"),
        )
        .expect("write production manifest");
        path
    }

    #[test]
    fn current_clean_catalog_template_and_seed_load_as_production_bootstrap() {
        let directory = TestDirectory::create();
        let manifest = write_manifest(&directory.0, PRODUCTION_BOOTSTRAP_SCHEMA_VERSION);

        let bootstrap = load_bootstrap(&manifest).expect("load current production bootstrap");

        crate::postgres::validate_bootstrap(&bootstrap)
            .expect("validate current production bootstrap directory");
        assert_eq!(bootstrap.characters.len(), 1);
        assert_eq!(bootstrap.world.key, "world-topology-primary");
        assert_eq!(bootstrap.characters[0].actor_id.as_str(), "player");
    }

    /// The served world the identity proof runs in, as the tree declares it.
    ///
    /// The land's own `world.json` is the single tracked statement of which
    /// catalog, profile, compiled template, and seed make this world. The
    /// bootstrap manifest binds those to a deployment's accounts; it does not
    /// restate them, so a manifest and this test cannot disagree about which
    /// land is served.
    fn served_world(relative: &str) -> serde_json::Value {
        let path = content_path(relative);
        let document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("the served world is readable"))
                .expect("the served world is JSON");
        assert_eq!(document["schema_version"], 1);
        assert_eq!(document["kind"], "served_world");
        let base = path.parent().expect("a served world has a directory");
        let resolve = |field: &str| {
            base.join(document[field].as_str().expect("a path"))
                .canonicalize()
                .expect("the named content exists")
        };
        serde_json::json!({
            "id": document["id"],
            "catalog": resolve("catalog"),
            "catalog_profile": document["catalog_profile"],
            "world_template": resolve("world_template"),
            "simulation_seed": resolve("simulation_seed"),
            "controlled_actor": document["controlled_actor"],
            "rng_seed": document["rng_seed"],
        })
    }

    /// Compose a bootstrap manifest for a served world, exactly as a harness or
    /// a deployment does: the world document supplies the content, the caller
    /// supplies the accounts.
    fn manifest_for(directory: &Path, world: &serde_json::Value) -> PathBuf {
        let manifest = serde_json::json!({
            "schema_version": PRODUCTION_BOOTSTRAP_SCHEMA_VERSION,
            "catalog": world["catalog"],
            "catalog_profile": world["catalog_profile"],
            "world_template": world["world_template"],
            "world": {
                "facet_id": "018f4d9e-8d57-7a1c-9d1a-8cb840d86dc1",
                "key": "identity-proof-primary",
                "simulation_seed": world["simulation_seed"],
                "rng_seed": world["rng_seed"],
            },
            "characters": [{
                "account_id": "018f4d9e-8d57-7a1c-9d1a-8cb840d86dc2",
                "character_id": "018f4d9e-8d57-7a1c-9d1a-8cb840d86dc3",
                "slot": 1,
                "display_name": "Wayfarer",
                "actor_id": world["controlled_actor"],
            }],
        });
        let path = directory.join("bootstrap.json");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&manifest).expect("encode manifest"),
        )
        .expect("write manifest");
        path
    }

    /// The slice's acceptance proof: the server's one canonical world is the
    /// identity proof's land, loaded from the authoring compiler's emitted
    /// template, with the cast the proof needs standing in it.
    #[test]
    fn the_identity_proof_land_loads_from_the_compilers_emitted_template() {
        let directory = TestDirectory::create();
        let world = served_world("content/lands/identity-proof/world.json");
        let manifest = manifest_for(&directory.0, &world);

        let bootstrap = load_bootstrap(&manifest).expect("load the identity proof bootstrap");
        crate::postgres::validate_bootstrap(&bootstrap)
            .expect("validate the identity proof bootstrap directory");

        let identity = bootstrap.world.engine.definition().content_identity();
        assert_eq!(identity.world_template_id, "identity_proof");
        assert_eq!(identity.catalog_profile, "profile/first_land_structure");

        // The cast of packet section 3: the player, the keeper who carries the
        // restoration service, and the hostile at the end of the dangerous
        // route. Every seeded position is traversable and inside the settlement
        // — the seed validator refuses a blocked or out-of-bounds placement, so
        // loading at all is that proof.
        let state = bootstrap.world.engine.world();
        let actors = state
            .actors
            .iter()
            .map(|actor| actor.id.as_str().to_owned())
            .collect::<Vec<_>>();
        assert!(actors.contains(&"player".to_string()), "{actors:?}");
        assert!(
            actors.contains(&"threshold_keeper".to_string()),
            "{actors:?}"
        );
        for actor in &state.actors {
            assert_eq!(actor.location.realm, "identity_proof");
            assert_eq!(actor.location.level, "settlement");
        }

        assert_eq!(state.service_instances.len(), 1);
        let service = &state.service_instances[0];
        assert_eq!(service.id, "keeper_rite");
        let keeper = state
            .actors
            .iter()
            .find(|actor| actor.id.as_str() == "threshold_keeper")
            .expect("the keeper is seeded");
        assert_eq!(
            service.position, keeper.location,
            "the restoration service stands with the keeper"
        );

        assert_eq!(state.ecology_sites.len(), 1);
        assert!(state.ecology_sites.contains_key("ruin_mouth_lair"));

        assert_eq!(bootstrap.characters.len(), 1);
        assert_eq!(bootstrap.characters[0].actor_id.as_str(), "player");
    }

    /// The rejection beside it: a template that fails the runtime's own
    /// validation is refused at bootstrap, by name, rather than loading a world
    /// nobody proved.
    #[test]
    fn a_world_template_that_fails_validation_is_refused_at_bootstrap() {
        let directory = TestDirectory::create();
        let world = served_world("content/lands/identity-proof/world.json");
        let source = PathBuf::from(world["world_template"].as_str().expect("a path"));
        let mut template: serde_json::Value =
            serde_json::from_slice(&fs::read(&source).expect("read the compiled template"))
                .expect("decode the compiled template");
        template["realms"]["identity_proof"]["levels"]["settlement"]["cells"][0][0] =
            serde_json::json!(["no_such_terrain"]);
        let broken = directory.0.join("broken-template.json");
        fs::write(
            &broken,
            serde_json::to_vec_pretty(&template).expect("encode the broken template"),
        )
        .expect("write the broken template");

        let mut world = world;
        world["world_template"] = serde_json::json!(broken);
        let manifest = manifest_for(&directory.0, &world);

        let error = load_bootstrap(&manifest)
            .err()
            .expect("an invalid template is refused");
        assert!(error.contains("no_such_terrain"), "{error}");
    }

    /// And the other half of "no silent fallback": a manifest that names a
    /// world template which is not there refuses rather than serving some other
    /// land the tree happens to carry.
    #[test]
    fn a_manifest_naming_an_absent_world_template_is_refused() {
        let directory = TestDirectory::create();
        let mut world = served_world("content/lands/identity-proof/world.json");
        world["world_template"] = serde_json::json!(directory.0.join("not-a-world.json"));
        let manifest = manifest_for(&directory.0, &world);

        let error = load_bootstrap(&manifest)
            .err()
            .expect("an absent template is refused");
        assert!(error.contains("production input is unavailable"), "{error}");
    }

    /// D4: the manifest declares one world. The predecessor's array of
    /// selectable copies is refused outright rather than partially honoured.
    #[test]
    fn a_predecessor_multi_copy_manifest_is_refused() {
        let directory = TestDirectory::create();
        let manifest = write_manifest(&directory.0, PRODUCTION_BOOTSTRAP_SCHEMA_VERSION);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest).expect("read manifest"))
                .expect("decode manifest");
        let world = value["world"].clone();
        let object = value.as_object_mut().expect("manifest object");
        object.remove("world");
        object.insert("facets".to_string(), serde_json::json!([world]));
        fs::write(
            &manifest,
            serde_json::to_vec_pretty(&value).expect("encode predecessor manifest"),
        )
        .expect("write predecessor manifest");
        assert!(load_bootstrap(&manifest).is_err());
    }

    #[test]
    fn database_bootstrap_constraints_are_part_of_offline_verification() {
        let directory = TestDirectory::create();
        let manifest = write_manifest(&directory.0, PRODUCTION_BOOTSTRAP_SCHEMA_VERSION);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest).expect("read manifest"))
                .expect("decode manifest");
        value["characters"][0]["slot"] = serde_json::json!(0);
        fs::write(
            &manifest,
            serde_json::to_vec_pretty(&value).expect("encode invalid manifest"),
        )
        .expect("write invalid manifest");

        let bootstrap = load_bootstrap(&manifest).expect("load rules-valid manifest");
        assert!(crate::postgres::validate_bootstrap(&bootstrap).is_err());
    }

    #[test]
    fn obsolete_manifest_schema_and_non_file_input_fail_closed() {
        let directory = TestDirectory::create();
        let manifest = write_manifest(&directory.0, 0);

        assert!(load_bootstrap(&manifest).is_err());
        assert!(load_bootstrap(&directory.0).is_err());
    }

    #[test]
    fn credential_names_are_bounded_before_environment_or_file_access() {
        for name in ["", "Database-Url", "database_url", "../database-url"] {
            assert!(read_systemd_credential(name).is_err(), "accepted {name:?}");
        }
    }
}
