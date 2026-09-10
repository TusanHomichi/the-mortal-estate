---
last_updated: 2026-09-10
revision: 3
status: Audit and full local proof complete; PR review correction routes activation status solely to server notes.
public_safe: true
summary: Bounded code review, shortened checkpoint, private-state guidance, activation ownership and verification receipts.
---

# Post-merge repository audit

This Planning record owns the owner's requested repository, code and agent-context
audit against `0ca9156702ecdf01c3f78bdb8ad84decac38a581` (merged PR #53).
Scope: checkout hygiene, recent entry/control and dungeon-renderer code, maintained
documents and agent navigation. Gameplay, artwork and deployment changes remain
outside this maintenance slice. The owner subsequently authorized committing the
documentation cleanup, opening a PR and merging after required verification.

## Findings and ownership

- The genesis checkpoint mixed the delivered build with superseded "latest"
  dispatches. The ledger remains the Canonical checkpoint owner; its prior
  narrative is preserved in a separate History record. The checkpoint now links
  the continuation, code/content owners and separate paused work directly.
- The settled-conclusions Canonical index described the renderer replacement as
  pending. Its client row now routes implementation status to the browser owner.
  The README likewise points to maintained implementation and provenance owners.
- The workflow Contract and Canonical boundary/server documents claimed that no
  persistent player data or deployed interface existed. The server owner now
  distinguishes private saved-state obligations from external activation; the
  workflow and boundary map link there. No compatibility policy or owner ruling
  changes. The PR review caught remaining status assertions in the root guide,
  workflow and boundary heading; those are now conditional or status-neutral.
  Current activation status remains solely in the server owner.
- The [existing migration finding](https://github.com/TusanHomichi/the-mortal-estate/issues/54)
  remains owned by rules checkpoint migration and the offline server workflow.
  Its required proof precedes a future terrain-changing migration; this audit
  does not perform or authorize one.

`AGENTS.md` remains the single shared entry contract; `CLAUDE.md` only imports it.
Machine configuration, worker logs and session handoff stay outside the checkout.

## Verification and closeout

At entry, the checkout was clean on `main`, HEAD and remote `main` both matched
`0ca9156`, and only the local `main` branch remained. GitHub reported PR #53 merged
and both PR verification jobs successful. These are observed entry-state facts;
the audit's local edits and proof are recorded below.

The parent reviewed creation retry/control, actor-offer filtering, local-door
movement changes, and dungeon selection, visibility filtering, pointing,
occlusion and resource lifecycle alongside their direct tests. No new confirmed
code defect was found in those paths. A supervised read-only renderer reviewer
was stopped after eight minutes without a final report; that incomplete trial
adds no independent clean-review claim. Existing provisional artwork and broader
native route coverage remain owned by the live-dungeon execution record.

After inspecting the resolved plan, the parent ran:

```bash
python3 tools/run_verification.py --list --scope portable --scope web
python3 tools/run_verification.py --scope portable --scope web
```

Result: **COMPLETE**, all selected steps passed in 258.908 seconds. This includes
1,459 Rust tests, 553 Python tests, 556 browser unit tests, formatting, Clippy,
workspace compilation, Workbench proof, browser typecheck/build, routing, relative
links and public-boundary checks using the real private denylist. Seven gated
Rust cases remained ignored in this initial selection. PostgreSQL/native-browser
and clean-copy proof had not yet been rerun; the initial audit session had no
PostgreSQL capability configured. The delivery follow-up supplies the existing
local PostgreSQL capability and runs the complete baseline before merge.

A direct comparison proved the extracted checkpoint narrative byte-identical
and the phase records, standing orders and gates unchanged. The current-checkpoint
section shrank from 259 to 45 lines. The fact-class search found no remaining
copies of the retired no-private-state claim. Final closeout prose is checked
through the fast lane with every changed path explicitly selected.

Only documentation changed. No new deployment or saved-state operation is needed;
the playable source remains the merged baseline. The initial audit handed off an
uncommitted documentation patch on `main` at `0ca9156`. After owner authorization,
the delivery PR owns the resulting commit, full local verification, CI and merge
receipts. Machine-specific commands and logs remain in the local handoff. Resume
from this record and the current checkpoint; the next presentation objective
remains in its existing continuation record.


## Delivery verification

At `89512e6`, the complete local runner with the existing PostgreSQL capability
passed in 899.257 seconds, including all seven gated database checks, live TLS
and control proof, Chromium/Firefox/WebKit authoritative capture, and the isolated
clean-copy build/tests. Both initial PR CI jobs also passed. The subsequent PR
review correction changes only agent/document navigation and conditional policy
wording; its affected surfaces receive the fast docs/boundary lane and both CI
jobs run again. The PR owns those final-commit receipts.

Dependency installation reported two moderate package entries for one Vitest
advisory. [Issue #56](https://github.com/TusanHomichi/the-mortal-estate/issues/56)
records the observed versions, upstream advisory, scoped exposure assessment,
test-toolchain owner and required upgrade proof. No dependency upgrade is included
in this documentation slice.
