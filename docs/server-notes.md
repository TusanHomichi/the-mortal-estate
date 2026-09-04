---
last_updated: 2026-09-05
revision: 7
status: Individual deadline cutover implemented and verified under the September 5 owner direction; external-boundary activation and other recorded acceptance gates remain pending.
public_safe: true
summary: One authoritative world with individual deadlines, precise persistence, credentials, immutable migrations, and PostgreSQL proof.
routes:
  - crates/tme-server/**
  - deploy/**
  - content/lands/*/world.json
  - .sqlx/**
  - tools/run_gated_postgres.py
  - tools/live_server_harness.py
  - tools/run_production_smoke.py
---

# Server Notes

The client's counterpart to this document is [client notes](client-notes.md),
which records how the client speaks the post-D4 wire and what it does with
credentials under D7.

## The world instance, and what it is for

The Mortal Estate has **one canonical persistent world**. Players never select
among independently mutable copies of it. This is owner ruling D4
(`the-mortal-estate-g0-owner-rulings.md`), and the predecessor's player-visible
"facet" selector — the bootstrap directory, the switch route, the character-to-
copy binding, and the cross-copy character transfer — was removed rather than
hidden.

An internal world-instance abstraction survives, and it exists for exactly four
things:

1. **Tests and development.** Test harnesses construct an instance directly and
   drive it without a database or a network.
2. **Private staging.** A staging deployment is a separate process against a
   separate database, running the same code.
3. **Disaster recovery.** A restore brings up an instance from a checkpoint, and
   the restore fence proves which cluster and database it came from.
4. **Future transparent scaling.** Scaling work may shard or migrate work
   *inside* one world's history. It may not create player-selectable divergent
   histories.

Everything about the shape enforces that reading:

- One server process serves one world instance. `PostgresState` holds a single
  registered world, not a registry keyed by identity.
- One database holds one world row. `tme.facets` carries a singleton unique
  index, so a second row cannot be inserted at all.
- `recover_or_initialize` fails closed if the durable store holds anything other
  than exactly one world.
- A character has no world column. It belongs to the world, because there is
  only the one. Neither does a socket ticket, a command receipt, an audit event,
  or a player-kill mark: a column that could only ever hold the one world's id is
  the schema's version of a field the client must echo, so those came out too.
  `tme.facets.facet_id` remains as the world's durable identity, which disaster
  recovery uses to prove which world a checkpoint belongs to.
- A player-kill mark's deterministic id derives from its kill sequence alone.
  Leaving the world id out of that derivation also keeps a mark's identity stable
  when a checkpoint is restored into a differently-identified database.
- The control API exposes no route that lists or selects a world, and the wire
  DTOs refuse the predecessor's directory and switch shapes outright — the wire
  fixture corpus now carries those as *reject* cases.

- **Nothing a player sends or receives names a world.** No client command
  carries a world id, and no server envelope returns one. The optimistic
  concurrency field survives as `observed_world_revision` / `world_revision`,
  which is a revision counter, not an address.

If a future slice wants more than one live instance, it needs an owner decision
against D4 first, not a quiet extension of this abstraction.

An earlier pass kept `facet_id` on every command and envelope, reasoning that it
was opaque routing rather than a selection. The owner overruled that: a field the
client must echo, which can only ever hold one value, is ceremony, and it is
exactly the handle a later change could quietly repromote into visible
selection. Transparent scaling routes server-side. So the identity came off the
wire entirely, both rejection codes that could only mean "you named the wrong
world" (`RejectionCode::WrongFacet`, `PathPreviewRejectionCode::WrongFacet`) are
gone, and the wire fixture corpus carries the retired shapes as *reject* cases so
they cannot come back unnoticed.

## Which world the one process serves

D4 fixes that there is exactly one canonical world. **Which** land that world is
made of is configuration, and it is configuration in two documents rather than
one, because the two facts have different owners.

**The bootstrap manifest** (`TME_BOOTSTRAP_MANIFEST`, loaded by
`production::load_bootstrap`) carries the facts a *deployment* owns: the world's
durable identity and key, and which account and character are bound to which
seeded actor. There is no default manifest and no built-in land. A process
started without the variable exits saying which variable it wanted, and a
manifest naming content that is not there is refused rather than replaced —
`crates/tme-server/tests/production_configuration.rs` starts the real binary and
proves both, because "there is no silent fallback" is a claim about a process.

**A land's served-world document** (`content/lands/<land>/world.json`, kind
`served_world`) carries the facts the *tree* owns: which catalog, which catalog
profile, which compiled world template, which simulation seed, which actor the
controlled character is, and the RNG seed. It is the single tracked statement of
what a land's world is made of. A harness or a deployment composes a bootstrap
manifest from it and its own accounts; nothing restates the content triple, so a
proof harness and the tree cannot disagree about which land is being served.

Today one land declares one: [the identity proof's](../content/lands/identity-proof/README.md),
whose world template is **the authoring compiler's emitted output** rather than a
hand-authored corpus file (owner ruling R1, 2026-08-21). That is what makes an
owner's Workbench edit able to reach play at all. `tools/run_client_live_proof.py`
serves exactly that document, and judges *where* the client ended up: the
observation centre the client presents must be the square the served seed seats
the controlled actor on, so a proof that signed into some other land fails
instead of reporting success.

The corpus remains non-canonical conformance data, not the served-world
declaration for the identity proof. Diagnostic harnesses may serve fixtures:
`tools/run_pulse_capture.py` explicitly loads the `first_land_structure` corpus
world, and `tools/run_fixture_land_capture.py` loads the compiled authoring
fixture. Neither grants those fixtures production content authority.

## The line a scaling change may not cross

D4's second clause is the one that gets tested later, when scaling work arrives
and the instance abstraction looks like a convenient place to put a shard, a
region, or a "copy". A change crosses the line if it produces any of the
following, whatever it is named:

- a player-visible choice, list, hint, or address that names an instance;
- two instances that can accept writes to the same world state and diverge;
- a character whose history depends on which instance it was routed to;
- a transfer, migration, or arrival flow presented to a player as an action.

Transparent scaling means a player cannot tell it happened. If a player could in
principle notice which instance served them, and get a different world because of
it, that is the killed design returning under a new word, and it needs a fresh
owner ruling rather than an implementation decision.

## Enrollment password policy

`validate_enrollment` in `crates/tme-server/src/auth.rs` rejects a password too
close to its own context: the service's own name, the account's username, the
account's display name, or an entry on a supplied blocklist.

### The defect this policy was written against

The ported implementation tested those context words with **whole-string
equality**, underneath a gate requiring 15-128 Unicode scalars. Every such test
against a word shorter than fifteen scalars was therefore unreachable. The
service words are six and seven scalars, so they were dead code: no input could
reach them, and the check had never once fired. The predecessor carried the
identical defect with its own five-scalar service word, and the mechanical port
reproduced it faithfully.

The general lesson, which outlives this particular fix: **a length gate above an
equality test silently deletes that test for every value shorter than the gate.**
Context checks belong below the gate, and must match the way the value can
actually appear — inside a longer password, not as the whole of it.

### The rulings

**Service words — containment, case-folded.** "mortal" and "estate" are rejected
wherever they appear in the normalized password, per NIST SP 800-63B's rule
against the name of the service. Comparison is against a case-folded copy, since
NFC normalization does not fold case on its own.

**Username — containment, but only from five scalars up**
(`USERNAME_CONTAINMENT_FLOOR`). Usernames may be as short as three scalars, and
rejecting every password containing an arbitrary trigram would refuse an enormous
number of legitimate passwords for no real gain. Nothing is lost at the short end
either: the old equality test was already dead there, because a password short
enough to equal a sub-floor username dies at the length gate first.

**Display name — normalized whole-string equality.** A display name may be a
single character, which makes containment absurd — it would reject nearly every
password. Equality is the right shape here, and unlike the cases above it was
already reachable, since display names are commonly long enough to clear the
length gate.

These are product policy, not implementation detail. Changing any of the three
thresholds changes which passwords real people are allowed to choose, so they are
recorded here rather than left to be re-derived from the code.

## Vocabulary debt: "facet" now means "world instance"

The D4 cut removed the concept but deliberately did **not** rename the remaining
internal uses of the word "facet", to keep the surgery's diff reviewable. The
wire and the rejection codes were renamed, because those are player-facing and
had to move with the cut; everything behind them still says "facet". That rename is owed, and until it lands the code reads as though the
retired model is still alive. The term map:

| Current | Should become |
| --- | --- |
| `FacetId` / `facet_id` | `WorldInstanceId` / `world_instance_id` |
| `facet_revision`, `observed_facet_revision` (server-internal only; the wire already uses the world names) | `world_revision`, `observed_world_revision` |
| `FacetHandle`, `FacetRequest`, `FacetError`, `FacetCommand` | `WorldInstance*` |
| `FacetCheckpointV5`, `FACET_CHECKPOINT_SCHEMA_VERSION` | `WorldCheckpointV4`, `WORLD_CHECKPOINT_SCHEMA_VERSION` |
| `facet_kill_sequence` | `world_kill_sequence` |
| `tme.facets`, `facet_key` | `tme.world_instances`, `world_key` |
| audit actions `facet_tick`, `facet_presence` | `world_tick`, `world_presence` |
| module `crate::facet` | `crate::world_instance` |

This is bulk-mechanical work verified by the compiler and the standing test run.
It is not a behavior change.

## The absent killer's karma — deferred, not waived

**Owner ruling, 2026-08-20 (private-archive issue #3): logging off is not a karma
escape.** When a delayed hostile effect kills and the credited killer's sheet is
not loaded, the consequence is recorded durably and applied at their next
admission.

### What this replaced, and the defect underneath it

`PlayerKillConsequenceV1::RequiresAbsentKiller` fires when a delayed hostile
effect kills a player whose credited killer has already left the world. The
predecessor resolved it by looking up which *other facet* hosted the killer and
fanning the consequence out to that live engine. With one world that lookup is
meaningless, so the fan-out was removed in the D4 cut.

What it left behind was worse than an incomplete feature. The removal left a
vestigial `remote_linked_karma` map that nothing populated, and the absent
branch demanded an entry from it, so every absent-killer kill returned an error
that failed the **entire durable commit** — no mark, no victim record, nothing.
An earlier revision of this document claimed the mark still committed and only
the karma was skipped. That was wrong, and the code never behaved that way.

### The mechanism

1. When the kill is assessed and the killer is absent, a row goes into
   `tme.pending_player_kill_consequences` **in the same transaction and the same
   loop as the mark**. There is no interval in which a mark exists without the
   consequence it defers.
2. At the killer's next admission, every consequence they owe is applied to one
   candidate engine, and that candidate's checkpoint compare-and-swap, the
   deletion of the pending rows, and the mark corrections all commit in **one**
   transaction.

Exactly-once falls out of that atomicity rather than being defended by a status
column. Crash before the commit and the rows survive to be applied next time;
crash after and they are gone with the sheet already updated. There is no
interleaving in which the sheet advances without the rows going with it, because
the sheet only becomes durable through that same compare-and-swap.

### The rulings behind the details

**Pending consequences do not expire.** An expiry would be the escape hatch the
ruling exists to close, just with a timer on it.

**They apply before the player sees the world.** Application lands inside the
admission transaction, so the welcome frame already reflects it. A player never
sees a clean sheet that then changes under a live session.

**A mark records `linked_karma_added = false` at kill time** — truthfully,
because nothing has been added yet — and the transaction that applies the
consequence updates it to what the rules actually produced. The durable record
is accurate at every point in time rather than only at the end.

**Forgiveness follows the karma, not the killer's session at kill time**
(owner ruling 2026-08-20: *"you should be able to forgive at any time after"*).
Eligibility was `linked_karma_added && killer_session_id.is_some()`, and an
absent killer has no session by definition, which would have made their marks
permanently unforgivable no matter what landed later. Now the transaction that
applies the consequence sets eligibility to match. A present killer always holds
a live session when their kill is assessed, so for them eligibility has always
been exactly `linked_karma_added` — which is why setting it the same way makes a
returned absent killer indistinguishable from a present one, from the victim's
side.

### Still open

Nothing about this path is deferred any more. The one thing a later slice should
not quietly change is the shape above: if pending consequences ever gain an
expiry, a partial-application mode, or a separate transaction, the ruling has
been reversed and that needs an owner, not a refactor.

## Individual deadline scheduling

The [current timing ruling](boundary-map.md#21-authoritative-individual-deadlines-d5)
replaces the shared pulse. A persisted facet rebases one monotonic clock on its
recovered logical time. Before admitting an action it advances the rules to that
clock's precise timestamp. Housekeeping checks for due work every 25 milliseconds;
the check interval is operational and does not define or round gameplay deadlines.
The rules process overdue work at its individual due times before reaching the
requested timestamp. Durable application remains serialized through the facet.

Checkpoint 5 stores explicit millisecond timestamps and each actor's recovery
anchor. Earlier scalar-time checkpoint payloads are refused. Recovery pauses
simulation during downtime and preserves remaining cooldowns. The separate
private test-server deployment on the development host is [issue #40](https://github.com/TusanHomichi/the-mortal-estate/issues/40).

## Schema version tags

`tme.facets.checkpoint_schema = 3` and `tme.command_receipts.outcome_schema = 3`
are payload-format tags for the checkpoint and receipt encodings, not migration
counters. They kept their values across the migration re-founding on purpose;
`tme_rules::FACET_CHECKPOINT_SCHEMA_VERSION = 5` is a separate, unrelated
version on the rules-side checkpoint envelope.

## The external boundary, when it activates

This section is the successor's carried-forward version of an approved policy: what
protects the wire and the durable data **once there is something outside to
protect**. It lives here, rather than in [the boundary map](boundary-map.md),
because it is operational policy about this server's own external surfaces —
version domains, support windows, deploy order, rollback. The boundary map names
owners and invariants; it is not the place for a procedure. The map points here.

### It is not active

No externally distributed client, real persistent player data, released save or
content format, public API consumer, or deployed service interface exists.

Activation is an **explicit project decision that records one of those**. It is not
implied by an architecture being approved, by a contract having a version number,
by a deployment existing, or by a slice finishing. Until that decision is on
record, the default in
[agent workflow](agent-workflow.md#no-compatibility-adapters) governs: only the
exact current contract is supported, replacements are atomic, and every owned
caller migrates in the same slice.

### Four version domains stay separate

| Domain | Versions | Today |
| --- | --- | --- |
| internal rules and content contracts | per-contract constants in `crates/tme-rules/src/view/contract_versions.rs` | replaced atomically |
| the external wire | `PROTOCOL_MAJOR` / `PROTOCOL_MINOR`, `CONTROL_API_VERSION` in `crates/tme-protocol` | one supported version |
| the checkpoint payload | `tme.facets.checkpoint_schema`, and the rules-side envelope version | format tags, not counters |
| the SQL schema | the tracked migration set under `crates/tme-server/migrations/` | immutable founding migration plus deadline audit and precise consequence-time migrations |

Conflating any two of these is how a wire change becomes a database outage.

### After activation

- Production supports the current and immediately previous **wire major** for at
  least 90 days and two normal releases, whichever is longer.
- An unsupported major fails **before** upgrade. A connection with no common minor
  fails with a typed protocol error and a close, not a best-effort decode.
- **Only the protocol DTO and conversion modules may contain wire compatibility.**
  It never leaks into rules, engine, or store code.
- Every released checkpoint schema retains a forward migrator to the one current
  in-memory shape. There is one in-memory shape, not a family of them.
- Current and previous binaries declare and test the SQL schema range they
  support.
- Schema change follows **expand → migrate → contract**, so a rollback has
  somewhere to land.
- Retirement waits for the support window to elapse. A security emergency may
  shorten it only through a recorded incident decision.

Internal contracts *behind* the boundary stay directly replaceable when every owned
caller moves together. Activation protects the outside, not the inside.

### Migration mechanics that already hold today

These are not future policy. They are implemented and provable now, which is why
activation does not require inventing them under pressure:

- **Applied migrations are immutable and checksummed.**
  `crates/tme-server/src/store/migrations.rs::verify` compares the applied history
  against the embedded set by version, description, success flag, **and checksum**,
  and fails on any count or content mismatch. An edited migration that has already
  run is a hard error, not a silent divergence.
- Migration locking stays enabled, and a missing applied migration is never
  ignored.
- **One deployment actor runs migrations**, transactionally.
- **Production down-migrations are forbidden.** Rollback is a previous compatible
  binary, a forward fix, or a verified pre-deploy restore — never a reverse script
  against live data.
- The server refuses a PostgreSQL major it was not built for
  (`verify_postgres_major`), before touching anything.
- **Privilege separation is real, and it is in the deployment reference.**
  `deploy/production/postgres/18/roles.sql` creates a non-login owner role that
  owns the schema, a migrator that may assume it, and a runtime role that owns
  nothing. `grants.sql` gives the runtime `CONNECT`, schema `USAGE`, and the
  specific DML it needs — and explicitly revokes the credential table from it.
  The runtime cannot `CREATE`, cannot assume the owner role, and cannot read
  password hashes.
- SQLx offline metadata is committed and checked against a database built only
  from tracked migrations.

### Rollback and data loss

- A binary rollback is allowed only against a **compatible schema**.
- An irreversible data or schema rollback restores the pre-deploy backup into a
  new database, verifies hydration and the schema, content, account, character,
  and actor identities, and only then switches the service.
- **Restoring to an older recovery point is free only before new admission.** Once
  new writes exist, going back requires a recorded incident decision with the
  measured loss; otherwise keep current data and apply a forward fix.
- Operators never edit gameplay through ad hoc SQL. If a fact is wrong, the
  boundary that owns it is what changes it
  ([boundary map](boundary-map.md)).

A restore is proven, not assumed: the product's own restore fence requires the
operator to confirm which database a checkpoint came back into, and the drill that
exercised the whole path is recorded in
[the deployment drill](deploy-drill-2026-08-20.md).

## Gated-test runner contract (PostgreSQL)

**The runner exists: `tools/run_gated_postgres.py`, the `gated` lane of
`tools/run_verification.py`.** Before Phase 8 it did not, and the six
`#[ignore]`-gated tests in this workspace compiled on every run and executed on
none (private-archive issue #10). A certification that is never executed is not
a certification.

```bash
TME_PG_ADMIN_URL_FILE=<file> python3 tools/run_verification.py --scope gated
python3 tools/run_gated_postgres.py --admin-url-file <file> --only fenced_restore
```

The superuser URL is read from a **file**, never from the environment: a URL
with a password in it does not belong in a process listing. Everything the run
creates — databases, the EV role, the private temp root — is dropped on the way
out, including after a failure.

### One fresh migrated database per gated test

The gated tests in `tests/postgres_persistence.rs` each assume a FRESH database:
create it, run the migrations, point `TME_TEST_DATABASE_URL` at it, run ONE
gated test, drop it. Running two gated tests against the same database in one
invocation fails on cross-test state — observed, not hypothetical. The runner
provisions per entry in `GATED_TESTS` and never reuses one.

### The three traps, and how the runner clears each

- **The fenced-restore source must be the durability test's database.** Later
  tests truncate and reseed different accounts, so a dump taken after them lacks
  `durable_tester`. The runner's table puts `fenced_restore` immediately after
  `durable` and dumps that specific database;
  `tests/test_run_gated_postgres.py::OneFreshDatabasePerTest::test_the_fenced_restore_follows_the_durability_test`
  asserts the ordering so a later edit cannot quietly break it.
- **Synthetic kill marks inserted by raw SQL are not anchored or scheduled.**
  Anchoring happens only on the durable-effects insert path, and
  `reconcile_all_player_kill_marks` does not re-anchor unless forced. Test
  fixtures must reproduce the product spacing formula and then assert
  `verify_player_kill_marks` accepts it.
- **The fenced-restore test needs no external drill.** `pg_dump` into a fresh
  database — a new oid is what makes it a real restore rather than a rename —
  then the product's own
  `tme-server store restore-fence --confirm-restored-database`, then the test.
  The runner does exactly that, so this test is now in a standing lane rather
  than a documented manual procedure.

### The EV certification's runner-owned identity

`ev_certification.rs` and `postgres/database_recovery_tests.rs::ev_database_fault_certification` assert
the environment they were given, and none of it can be satisfied by accident:

| What the test asserts | What the runner provides |
| --- | --- |
| `current_database()` equals `TME_EV_DATABASE_NAME`, and it starts `tme_ev_` | a database named `tme_ev_<token>` |
| `current_user` equals `TME_EV_DATABASE_ROLE`, and it starts `tme_ev_role_` | a dedicated login role that owns that database, connected as itself |
| `shobj_description` is exactly `tme_ev:<TME_EV_DATABASE_SENTINEL>` | `COMMENT ON DATABASE` with a per-run random sentinel |
| `server_version_num` in `180000..190000` | checked before provisioning; a wrong cluster is refused with its version, not a confusing failure later |
| the exact tracked migrations, via `migrations::verify` | `tme-server migrate` as the owning role |
| `TME_EV_PRIVATE_TEMP_ROOT` absolute, and 0700-creatable | a per-run directory created mode 0700, removed afterwards |

### The inventory fails closed

`tests/test_run_gated_postgres.py::TheGatedInventory::test_every_ignored_test_in_the_workspace_has_an_entry`
scans every `#[ignore]` in `crates/` and asserts the runner has an entry for it.
Adding a gated test without adding it to the runner turns the Python suite red,
which is the only reason the previous state — six gated tests and no runner —
cannot recur.
