---
last_updated: 2026-09-05
revision: 2
status: In progress; private development deployment followed by authoritative browser controls.
public_safe: true
summary: Authorized two-slice execution, findings, and proof for a persistent private two-client play loop.
---

# Private two-client play loop

The owner accepted this sequence on September 5, starting from `510b6e4`:

1. Resolve [#40](https://github.com/TusanHomichi/the-mortal-estate/issues/40):
   install an isolated persistent private development server, prove two-client
   timing/reconnect, restart and backup restoration, then complete Git delivery.
2. In a separate PR, implement minimal browser sign-in, character selection,
   movement, reconciliation, reconnect, and logout against that deployment.
   Prove two independently controlled characters through the real browser UI.

This Planning receipt owns execution and observed evidence. Server configuration
and operational semantics belong to [server notes](../server-notes.md); browser
control and credential contracts belong to [client architecture](../client-architecture.md).
The runtime remains private. The presentation pause remains in force.

## Boundaries and proof

- Use dedicated service names, listener ports, storage and resource limits.
  Existing host services and the feel preview are independent inputs to the
  before/after host-health check, never deployment targets.
- Consume the declared served-world inputs. A reproducible development-only
  seed adds a second controlled character; Rust validates the composed bootstrap.
  It conveys no content promotion or acceptance of inherited mechanics.
- Keep operator credentials, certificates, backups, local configuration, and
  host-specific receipts outside the checkout. Build from carried source and
  retain an immutable release digest plus source-tree identity.
- Run affected verification and the complete baseline before each merge.
  Deployment proof exercises installed services; browser proof exercises its
  actual control adapter and UI with normal certificate verification.
- Search blast radius with `rg` for control routes, session cookie/token use,
  protocol version literals, bootstrap consumers, and verification ownership.
  Change internal wire contracts atomically with every caller and refusal proof.

## Findings

| Finding | Owner and disposition |
| --- | --- |
| Existing production provisioning assumes a dedicated host and owns its default database/proxy configuration | Development operations use separate units, cluster, listeners and storage. No production bootstrap is run on this shared host. |
| Login sets a persistent all-path cookie, conflicting with the browser's transient credentials and ticket-only socket contract | Resolved by control API v4: explicit transient bearer token, strict POST bootstrap, ticket-only socket, retired routes/cookies refused, and every internal caller migrated. |

| Prepared lifecycle mutation publishes a new revision under an existing server sequence, disconnecting the other strict browser on logout | Fixed at the server publication boundary: prepared checkpoints bind before/after sequences, durable writes and runtime commit advance together, and all prepared-mutation callers use the same persistence owner. Real two-tab logout and a prepared-publication regression prove it. |

## Evidence

The installed development services passed trusted HTTPS readiness, two-character
mutual visibility, independently offset full 3,000 ms actions, cooldown rejection,
reconnect during cooldown, and a complete service restart during an action.
Both sessions were revoked afterward. A custom-format backup restored into an
isolated database retained two characters and one world, passed restore fencing
and store verification, and the drill database was removed. Actual development
database replacement also passed fencing and readiness; TLS leaf renewal passed
normal hostname/certificate validation.

Portable deployment tests cover host isolation, conflicting unit refusal,
source-root refusal, release tampering (including extra files and symlinks),
and backup mutation/storage drift. Release activation, deliberate rollback and
reactivation passed private readiness. The shared host retained its pre-existing
listeners and active host nginx/PostgreSQL; the protected feel preview still
responded with its authentication challenge.

`python3 tools/run_verification.py --scope full --keep-going --report-disk`
completed in 873.457 s with every selected step passing, using an independent
scratch PostgreSQL cluster and the real private denylist. This includes the
installed tests, native/WebAssembly corpus, PostgreSQL, trusted wire, both browser
capture engines and clean-copy build/test proof. PR #46 merged as `7907c56` after both required CI checks passed; #40 is closed.
Machine inventory, generated inputs and detailed receipts remain local; GitHub
owns delivered revisions and issue closure state.

## Browser slice

The private play shell consumes the diagnostic renderer through the existing
frame/pointer seam. The connection adapter owns auth, one pending immutable
command, epoch reconciliation and reconnect. Settings persist only text size and
semantic key bindings. The candidate feel preview and presentation pause are
unchanged. Browser and server bundle activation remains gated by equal storage
and content contracts. Oversized persistence and certification test files were
split by responsibility, preserving exact runner-selected test names.

Observed focused proof: 278 shared codec and real-adapter tests passed. Normal
TLS navigation to the installed host passed in both hardware-rendered engines.
Complete UI proof and the browser slice's full baseline are pending.
