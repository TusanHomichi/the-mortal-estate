---
last_updated: 2026-09-04
revision: 2
status: Complete. Documentation and agent-context audit plus owner-authorized integrity tooling fixes verified locally and in a clean-copy Python run. No product implementation or owner gate change.
public_safe: true
summary: Scope, evidence, findings, and verification for the documentation and agent-context audit.
---

# Documentation and agent-context audit

## Scope

Audit the carried documentation against the current tree, correct demonstrably
stale claims, and make agent entry points lean indexes into fuller owners.
The owner additionally authorized fixing discovered tooling defects in this slice.
Preserve owner rulings, pending acceptance, historical evidence, and the
pre-external-boundary policy. Do not change gameplay, presentation code, Git
lifecycle, deployment state, or local agent worktrees.

Starting revision: `642ce24` on `main`. The initial working tree had only an
untracked `.claude/` directory; it is outside this edit set.

## Findings and resolution

| Finding | Evidence inspected | Resolution and sole owner |
| --- | --- | --- |
| Entry context carried a verification manual; the decision index repeated full and superseded rulings | Original `AGENTS.md`: 2,098 words; original settled index: 3,187 words | Entry guide reduced to 931 words; index to 871. Detailed commands moved to [verification](../verification.md); [workflow](../agent-workflow.md#context-loading) defines selective reading. `CLAUDE.md` remains the same import-only stub. |
| Agent setup assumed Claude worktree placement and a standalone charter the tree does not carry | `git branch --show-current`, current root, Markdown inventory, and workflow instructions | [Workflow](../agent-workflow.md#working-in-a-linked-worktree) starts from observed checkout state and points to carried rulings. No provider-specific parallel rulebook or private archive prerequisite added. |
| CI example omitted the web scope; prose claimed remote protection state as fact | `.github/workflows/verify.yml`, `tools/verification/table.py` | [CI usage](../agent-workflow.md#github-ci-and-issues) matches the workflow and distinguishes required remote setup from what local tests prove. |
| Presentation described the Godot lattice as the only renderer and the engine choice as unresolved | `web/src/main.ts`, `web/src/feelScene.ts`, browser-first contract | [Client notes](../client-notes.md#current-implementation) owns the current browser/Godot split. Presentation links there rather than duplicating implementation state. |
| Earlier chrome, bag, log, and display rules remained current beside their replacements | September 3 rulings already recorded in presentation direction and the latest documentation commits | [Presentation](../presentation-direction.md#chrome-and-actions) consolidates the current target and labels what it supersedes. No visual acceptance or new design decision inferred. |
| Source-geometry language read as forbidding the subsequently ruled card output | Projection and later structure/card rulings in the same owner | [Projection](../presentation-direction.md#projection-and-surface-ruling) distinguishes world-up source construction from runtime representation; [structure and cards](../presentation-direction.md#structure-and-cards) keeps the output rule. |
| Accepted interface/scaling direction could be mistaken for completed browser functionality | `web/src/main.ts`, `web/src/camera.ts`, `web/tests/camera.test.ts`, `web/src/style.css` | [Current implementation](../client-notes.md#current-implementation) records the integration and framing gaps, their owning layout slice, and the matched-extent proof required. This audit does not implement or dispatch that slice. |
| Browser operating instructions were mixed with Godot environment controls | `web/vite.config.ts`, `web/src/presets.ts`, `web/proof/serve.mjs`, walk and capture scripts | [Browser operation](../client-notes.md#browser-operation-and-proof) documents dev-server-only packet serving, URL controls, and optional proof inputs. A build is not a packaged private asset set. |
| Authoring method still claimed no validators or verification runner existed | Compiler, boundary checks, Workbench, and verification tools | [Authoring standard](../authoring-contracts.md#0-reading-guide) links the implementations and keeps unimplemented method obligations explicit. |
| Fixture master anchor pointed to its previous module; land README called authored seed data generated | `crates/tme-authoring/src/contract/fixture.rs`, `promotion.rs`, identity-proof directory | Corrected [fixture editing](../../content/authoring-fixture/README.md#editing-rules) and [land inputs](../../content/lands/identity-proof/README.md#what-is-here). No receipt or master changed. |
| Workbench promotion proof still watched the old anchor module | `DIGEST_CONSTANT` and `watched` in `tests/test_workbench_apply.py`; actual `master_digest` in `contract/fixture.rs` | Corrected the existing test's watched path to the fixture contract. This repairs the proof behind the documentation; no product behavior changed. |
| Documentation said fixtures were never served; pulse comments said it used the sign-in proof's world | `FIRST_LAND` in `tools/run_pulse_capture.py`, fixture capture, and sign-in live proof | [Server notes](../server-notes.md#which-world-the-one-process-serves) distinguishes production content authority from diagnostic serving. Pulse documentation now names its actual corpus input. |
| Agent CLI example passed an unsupported `--session` to `verbs` and omitted land-selection guidance | `tools/workbench/stage.py::build_parser`, `verbs --help`, `serve.py --help` | Corrected the docstring and [V1 example](../workbench-v1.md#agent-parity); [V0](../workbench-v0.md#running-it) now shows explicit projection selection. Executable Python is unchanged. |
| Ownership map called six listed contracts four, assigned wire ownership imprecisely, and cited a moved test | `crates/tme-protocol`, authoring contract modules, `crates/tme-rules/tests/inventory_services_social.rs` | Corrected [boundary map](../boundary-map.md#11-the-authoredruntime-contract-seam), including the path to `cases/service_transactions.rs`. |
| Private-root wording conflicted with declared capabilities and reproducible dependencies | `.gitignore`, `client/.gitignore`, capability resolver, web package scripts | [Working-root policy](../working-root-policy.md#the-roots) distinguishes disposable input from declared capabilities and reproducible output. [Public policy](../public-boundary-policy.md#three-kinds-of-material) explicitly keeps quarantine outside the checkout. |
| Resume context ended at the August presentation pause despite later authorized browser work | Genesis ledger, paused plan, September browser and presentation rulings | Added the [current checkpoint](genesis-ledger.md#current-checkpoint-2026-09-04) and pointers from the older plans. P1, remaining identity-proof slices, G10, and G11 retain their recorded limits. |

Present facts live in those owners. This table is a dated audit record, not a
second specification or an authorization for implementation work.

## Coverage and limits

Reviewed root entry points, every top-level documentation owner's routing and
status, active-plan checkpoints, content READMEs/provenance inventory, and the
deployment overview and runbooks. Checked the carried Markdown link graph and
sampled implementation claims against their named modules, constants, parser
options, fixtures, and workflow. Historical drill receipts and old plan evidence
remain dated; they were not rewritten as fresh observations.

No deployment was contacted, remote branch protection verified, or owner gate
closed. Existing unimplemented targets remain explicit in their owners. The
audit changed documentation, two Python documentation strings/comments, and one
existing test's watched path; the subsequent authorized repair changes Workbench
integrity tooling and adds regression proof. An AST comparison against the
starting revision confirmed `run_pulse_capture.py` and `stage.py` retain their
executable code. No routinely read
documentation file exceeds 1,000 lines.

## Verification

Initial `python3 tools/run_verification.py --scope docs`: COMPLETE. Routing,
whitespace, and Markdown links passed before edits. This establishes that the
stale prose was not a broken-link problem.

The first post-move docs run caught the old V0 link to `AGENTS.md#the-four-lanes`.
It was migrated to the new verification owner; the subsequent docs run passed.

The first focused run passed metadata, documentation, all boundary checks, and
the boundary/capture/harness/verification Python groups. Workbench Apply's
whole-tree atomicity check failed while the audit was still editing documents:
the reported changed hash `a700a5d742351c9a...` identified `docs/agent-workflow.md`.
The scripted demo consequently did not run. That run is a failure, not a pass;
the tree is frozen for the repeat. The workflow now records this integrity-proof
constraint.

Direct CLI checks: `stage.py verbs --help` confirms there is no `--session`;
`serve.py --help` confirms `--projection`. Running `stage.py --projection
content/lands/identity-proof/generated/workbench_projection.json verbs --json`
returned valid JSON with `truth`, `dressing`, and `asset` groups.

The repeated focused run used `--scope fast` with a `--changed-path` argument
for every modified path plus the two new audit/verification documents. The
resolved scopes were `meta`, `docs`, `boundary`, `python`, and `workbench`.
It finished **COMPLETE**, exit 0, in 294.280 seconds:

- routing, Markdown links, whitespace, step targets, and all boundary checks
  passed, including the provisioned private-terms check;
- Python groups ran 552 tests: 549 passed and three capture tests skipped
  because `TME_GODOT` was unset;
- the Workbench demo passed selection, resolution, staging, preview, successful
  and rejected Apply, and the unchanged-tree assertion. Its capture branch used
  the tracked fixture after explicitly reporting a fresh capture unavailable;
- no full Rust, browser, native Godot, PostgreSQL, or clean-clone run is claimed.
  The path-selected runner did not select those lanes for these changes.

After the integrity run finished, only documentation closeout was edited; the
docs and boundary lanes were rerun for those final bytes. No commit, branch,
pull request, publication, or remote issue was created.

## Resolved tooling issue A1: integrity inventory included disposable subtrees

**Owner:** [Workbench atomicity proof](../workbench-v1.md#apply).

**Evidence:** the two former `carried_tree` functions recursively walked the
filesystem and compared root ignore entries only with `relative.parts[0]`.
`web/node_modules` could never match `web`. The walk also descended into nested
agent worktrees, dependencies, and build output. The successful earlier run
spent 131.469 seconds in Workbench tests and 122.704 seconds in the demo; those
are observed timings, not attribution of every second to traversal.

**Resolution:** both callers now use `tools/workbench_integrity.py`, with file
selection delegated to the existing `boundary_common.carried_files` owner.
This preserves untracked sources, honors Git ignore rules, and excludes nested
repositories without reading their files. No worktree was removed. The earlier
comments and initial issue incorrectly called the clean copy Git-free:
`run_clean_clone_proof.py` initializes and stages a fresh repository before
running tests. That existing contract requires no filesystem fallback.

**Related defect fixed:** the demo formerly printed a changed-tree warning and
continued to a successful exit. It now raises with the changed paths.

**Regression proof:** six scratch-repository tests cover ignored dependencies,
nested ignore rules and negation, forcibly tracked ignored files, ordinary and
linked nested repositories, source edits/additions/deletions, fresh-index clean
exports, absent Git metadata failing closed, and a source-mutation mutant in the
demo's rejection path. All six passed directly and in both final runs recorded below.

The first post-repair focused run stopped at the hostname check: the test's Git
configuration key for a fixture commit was parsed as a hostname. The scratch
commit now receives its identity through process-local Git environment variables,
with a reserved example address. No check was weakened or bypassed; verification
will be repeated on that correction.

The next focused run and clean-copy Python run both caught the initial helper
placement inside `tools/workbench/`: that package's dependency proof rejects
imports of other repository tool modules. The helper belongs to verification,
so it was moved alongside the demo as `tools/workbench_integrity.py`. The
Workbench package's dependency contract remains unchanged. All six new inventory
regressions passed in both runs; the complete runs are repeated after relocation.

## Final verification after the integrity repair

The final `--scope fast` run selected every changed path and the three new files
(the verification owner, this audit, and the shared integrity helper). It was
**COMPLETE**, exit 0, in **61.963 seconds**: routing, Markdown links, whitespace,
all four boundary checks, all Python groups, and the Workbench demo passed.
The Python groups ran **558 tests: 555 passed and three capture tests skipped**
because `TME_GODOT` was unset. Workbench tests took 10.819 seconds; the full demo
step took 0.866 seconds and reported the carried tree unchanged. The demo used
the tracked capture fixture and explicitly reported fresh capture unavailable.
These are observed run times, not a controlled benchmark.

`python3 tools/run_clean_clone_proof.py --scope python` also passed, exit 0.
It exported **1,137 carried files**, initialized a fresh index, and ran the same
558 Python tests with the same three skips. Its inner lane took **65.727 seconds**;
peak isolated build output was **346 MiB**, removed on completion along with the
scratch copy. This is a clean-copy **Python** proof, including real Workbench
compiler calls; it does not claim the default portable/full Rust suite, browser,
native Godot, PostgreSQL, or the demo in that copy.

Both runs finished before this documentation receipt was written. Final docs and
boundary checks cover the receipt. A1 and the related demo exit-status defect
are fixed; no audit tooling finding remains open. At this audit checkpoint, changes were local and uncommitted.

## Integration with current main

After the owner authorized commit, push, merge, and cleanup, a remote fetch
found the starting checkout 43 commits behind `origin/main` at `606a738`.
The audit was rebased onto that head. The integration retains the subsequent
cell-count and ultrawide rulings, projection reaffirmation, live-character and
Tauri decisions, two-engine browser proof, and schema-5 gait implementation.
The concise index and current checkpoint now route to those owners. Browser
operation requires both engines; the implementation record distinguishes the
moving presented figure from its still pulse-bound authoritative square.

The owner confirmed the personal repository linked by the root README. The
workflow now resolves the current remote before GitHub lifecycle work instead
of carrying an obsolete organization name. The owner's explicit move instruction
also updates the original-content copyright notice to the personal identity;
license terms and third-party attributions are unchanged. Earlier timings and counts above
remain evidence for their stated audit checkpoints. The integrated commit's
complete proof is recorded in its pull-request checks before merge.
