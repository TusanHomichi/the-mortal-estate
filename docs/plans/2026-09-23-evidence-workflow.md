---
last_updated: 2026-09-23
revision: 2
status: History; standing workflow setup completed locally, with a verified repair cycle and retained evidence. Subsequent authorized delivery is owned by PR 82.
public_safe: true
summary: Archived setup graph, owner effort waiver, hostname repair, verification evidence and local handoff limits.
---

# Evidence-driven workflow setup

## Outcome and authority

The September 23 owner request establishes the supplied evidence-driven workflow
as this project's standing default. Adapt the existing contract, workflow,
verification usage and ledger; preserve project boundaries and existing work.
This History record archives the canonical execution graph for setup. It does not dispatch the
gameplay backlog or create an executor, plugin, dashboard or CI system.

The owner's subsequent merge-and-cleanup request is tracked separately in
[PR #82](https://github.com/TusanHomichi/the-mortal-estate/pull/82). Its canonical
delivery graph owns current delivery state, required gates and the next action;
this record preserves the original local setup's scope and receipts.

Acceptance: the four entry documents agree on ownership, permissions, models,
evidence and continuation; the changed-path documentation gates and link review
pass; a fresh instruction-loading check is attempted and its actual result is
recorded; this setup completes the graph through review and archival.

Entry state: clean `main` at
`0764c639d3a4fa10e200eb885479ca3ffb21acfa`. Read-only repository identity and
PR inspection confirmed that PR #77 merged at that revision. Existing gameplay
dispatch remains in the [death-return record](2026-09-13-death-return.md).
No gameplay, content, runtime, CI or deployment change belongs to this setup.

Authority comes from this setup request and
[implementer autonomy](../agent-workflow.md#implementer-autonomy). Local edits,
checks and useful delegated work are authorized. Git lifecycle and remote writes
have no setup-specific dispatch; prior delivery permissions are scoped to their
own records. Private-preview authority remains with
[server notes](../server-notes.md#private-development-deployment), and this
documentation slice changes no playable build. Protected inputs remain governed
by the [public boundary](../public-boundary-policy.md) and
[working-root policy](../working-root-policy.md).

Effort policy: during setup on September 23, the owner explicitly rejected an
arbitrary repair-cycle limit. The standing effort waiver is maintained in
[authority and effort](../agent-workflow.md#authority-and-effort); all setup nodes
inherited it. The waiver supplies no additional scope or Git, spending or
publication authority.

## Canonical graph

All nodes inherit the outcome, authority and effort policy above. State and
evidence are updated only from observed artifacts. The parent reviews delegated
results; a worker's completion message alone does not satisfy acceptance.

| ID | Concrete outcome | Depends on | Assigned owner / effort | Inputs | Acceptance | State | Evidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| W1 | Identify existing gates, evidence conventions and instruction chain | none | `gpt-6-luna` / `max`, parent review by `gpt-6-astra` / `max` | `AGENTS.md`, workflow, verification, runner, CI, current ledger | Source-backed bounded report reviewed against the setup scope | complete | Parent-reviewed source report; `carried_files`, Markdown checker and CLI inspected; listed plan selects `meta, docs, boundary` |
| W2 | Establish the standing procedure and concise checkpoint | W1 | `gpt-6-astra` / `max` | W1, owner setup request, maintained boundaries | Diff preserves rulings, assigns one owner per workflow fact and records actual authority | complete | Parent self-review of contract, workflow and checkpoint; effort ruling recorded by W2-E |
| W2-V | Document actual verification receipt and capability usage | W1 | `gpt-6-luna` / `max`; owns only `docs/verification.md` | Runner CLI/executor, carried-file scanning, W1 | Parent-reviewed concise guidance with no invented automation or new gate | complete | Parent inspected diff and runner output code; clarified separate configured/default evidence and refreshed summary |
| W2-E | Set the standing repair-effort limit or waiver | none | owner decision; `gpt-6-astra` / `max` records it | Existing autonomy rule and September 23 owner response | Explicit owner choice recorded in the workflow, with its source | complete | Owner rejected an arbitrary repair-cycle limit; standing scoped waiver recorded |
| W3-R | Register the cited documentation hosts under existing policy | W2, W2-V | `gpt-6-luna` / `max`; owns hostname allowlist and boundary-check documentation | Failed W3 log, exact source URLs, existing allowlist policy | Only two reasoned exact-host entries; current inventory has one owner; scanner rules retained | complete | Parent reviewed actual two-file diff, exact-host policy and unchanged scanner; clarified the citation comment's scope |
| W3 | Prove documentation and instruction discovery | W2, W2-V, W3-R | `gpt-6-luna` / `max` for documentation gates; `gpt-6-astra` / `max` for fresh-session review | Exact candidate, resolved check plan, official instruction guidance | Changed-path gates, supplemental new-file review and fresh-session result retained separately | complete | Parent reviewed all 14 PASS steps and 670 Python tests, fresh-session response and qualified whitespace control; receipts below |
| W4 | Review, archive and hand off the completed setup | W3, W2-E | `gpt-6-astra` / `max` | Final diff, raw W3 receipts, graph and permission record | Parent review, affected rechecks, evidence read-back and owned cleanup; remaining limits and one next action recorded | complete | Parent self-review and receipt read-back; graph archived here, final affected-check receipts retained in the handoff bundle |

Original setup handoff: at the next gameplay resumption, reconcile the
[death-return record](2026-09-13-death-return.md) against current source and
receipts before selecting its next bounded graph. This setup ends at the verified
local documentation handoff. Subsequent authorized delivery follows PR #82 above.

## Evidence and findings

The repository already has a runner, documentation/link checks, issue index and
dated execution records. Reuse them. The current checkpoint is long because it
mixes dated deliveries with the next action; retain the dated material as history
and put a short current entry above it.

W1's source-backed report was reviewed against `tools/boundary_common.py`,
`tools/check_markdown_links.py` and the runner. Carried-file checks include new
untracked Markdown without staging; ordinary Git whitespace checks do not.
The initial documentation plan had eight steps in `meta, docs, boundary`.
Full verification remains the pre-merge gate; this local documentation setup
does not require a runtime, browser or deployment run.

The initial documentation run used `--keep-going`: all eight step outcomes were
reviewed. Seven passed, and hostnames failed on the two newly cited official
documentation domains. The
[existing allowlist policy](../boundary-checks.md#allowlists) permits exact hosts
with a reason. W3-R adds only the cited hosts and routes the boundary document's
already-stale inventory to its owning allowlist. It changes no scanner rule.
The failed run remains `initial-docs.json` / `initial-docs.log`, with exit 1;
it is not relabeled as a pass after repair.

The initial supplemental `git diff --no-index --check` returned 1 with an empty
log. A disposable clean control also returned 1 without diagnostics; its
trailing-space mutant returned 3 and named the defect. The new record matched
the clean control. The qualified observation exited 0 and removed its own
control directory. The raw initial receipt remains intact; the lesson is recorded
in [verification usage](../verification.md#the-four-lanes). No repository check
was added or changed.

The allowlist edit adds the existing Python suite to the runner's selected fast
plan, including the hostname scanner's existing refusal mutants. The rerun
supplied all seven changed paths and passed all 14 selected steps. Its five Python
groups ran 99, 55, 161, 172 and 183 tests: 670 in total. The private denylist was
available; this was a configured local COMPLETE verdict without degradation.

The fresh instruction check completed in 66.371 seconds with exit 0. A separate
CLI invocation explicitly requested `gpt-6-astra` / `max`, read-only operation and
no tools. Its sole response reproduced the new project evidence bullet and the
machine/project rules while correctly identifying linked documents as unread.
The event log contains no tool calls. The parent read the raw response and
verified its stored SHA-256:
`ffdd253ed5c5572cf18ea499d6a5747391df41e3bd7d79329dd37749b4ad42da`.
The project entry file has not changed since that check; later procedure and
receipt edits still receive their own affected documentation checks.

The active host exposes the requested three GPT model names and `max` effort.
The local CLI configuration requests `gpt-6-astra` / `max`; the native worker was
explicitly requested as `gpt-6-luna` / `max`. Neither a configuration nor a routing
request independently attests the running model. Sol has no necessary task in
this documentation slice. DeepSeek remains paused under the existing owner
instruction; the generic template's optional-helper paragraph does not re-enable
it.

Official sources consulted for the instruction-loading check:
[AGENTS.md discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md)
and [concise, task-routed instructions](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra).
These explain host behavior; the owner selects the project's model division.

## Receipts and closeout

The private handoff bundle is identified as `workflow-setup-20260923-xf_137sk`.
It retains actual argv, working directory, UTC start/end, elapsed time, exit
status, base revision, candidate patch and per-file hashes, emitted logs and
their digests. The repaired and final candidates are retained separately for
read-back. The base commit identifies ancestry; these uncommitted edits are
identified by their file hashes, not relabeled as that commit.

| Receipt | Observed result | Elapsed seconds | Emitted-log SHA-256 |
| --- | --- | --- | --- |
| `initial-docs` | exit 1; seven PASS and hostname FAIL | 18.891 | `4dbf6e8b3b38cd2fa2b4e4ff9636ec9c461c7990e626d9d923c0323f73268f5a` |
| `fresh-instructions` | exit 0; new entry rule loaded without tool/file reads | 66.371 | `ffdd253ed5c5572cf18ea499d6a5747391df41e3bd7d79329dd37749b4ad42da` |
| `repaired-fast` | exit 0; COMPLETE, 14 PASS, 670 Python tests | 136.222 | `9d63df9253a483ec3a3410b5426ae629aefdfb325ce1e40768155f9e8f9942f4` |
| `qualified-untracked-whitespace` | exit 0; clean/new-file agreement and negative-control rejection | 0.050 | `26fda5e09f4f3d30342f9c5e8212263c1600e9e49f0b032753a8bdb712466c52` |

The selected fast command was inspected with `--list` before execution:

```bash
python3 tools/run_verification.py --scope fast --keep-going \
  --changed-path AGENTS.md \
  --changed-path docs/agent-workflow.md \
  --changed-path docs/verification.md \
  --changed-path docs/plans/genesis-ledger.md \
  --changed-path docs/plans/2026-09-23-evidence-workflow.md \
  --changed-path docs/boundary-checks.md \
  --changed-path tools/hostname-allowlist.txt
```

Closeout documentation changes receive a fresh invocation of that complete
selected plan and the supplemental whitespace observation. Their `final-fast`
and `final-untracked-whitespace` receipts bind the final file hashes; earlier
receipts retain their original candidates. The final handoff points to those
artifacts so this record need not embed its own hash.

File ownership: `AGENTS.md`, workflow and verification usage are Contract;
`docs/boundary-checks.md` is Canonical; the genesis ledger is the Planning entry
point with dated history; this completed graph is History. The hostname allowlist
is configuration owned by the boundary checker. The historical checkpoint was
preserved verbatim below the new concise entry.

Review was parent self-review with bounded Luna exploration, documentation edits
and check execution. It was not a separate independent full review. The parent
inspected actual diffs, all completed step results, the fresh-session event log
and stored artifact hashes. No findings remain unrecorded. The disposable
whitespace controls were removed; proof artifacts are deliberately retained.

Limits: the running models' backend identities are not independently exposed by
the host; requested routing and configuration are recorded honestly. This remains
an agent-run procedure without an automatic dispatcher or restart service. Full
runtime verification, hosted CI, merge and preview refresh were outside this
documentation-only local dispatch. At that original handoff, no task branches,
worktrees or services had been created and the seven changed files were
uncommitted on the entry `main` revision. Later delivery evidence belongs to PR #82.
