---
last_updated: 2026-09-09
revision: 3
status: Manual allocation deployed with preserved saves; actor interaction follow-up in progress; JavaScript-disabled WebKit proof remains unavailable.
public_safe: true
summary: Game entry presentation, recovered class allocation and durable creation through the real authority.
---

# Game entry and character creation

The owner directed sign-in, roster selection and character creation to belong
visually to the pixel game, with no website-like lead-in. Creation uses the five
known starting classes and recovered allocation constraints; nationalities are
deferred. [Presentation direction](../presentation-direction.md#entry-and-character-creation)
owns that ruling; [browser client](../browser-client.md#private-authoritative-play)
owns implementation, and [class training](../class-training-contract.md#character-creation)
owns mechanical meaning.

## Initial delivered slice

- Full-screen sign-in, roster, creation and recovery share restrained bronze
  frames, original class emblems, readable type and the verified pixel town.
- Each class begins at its catalog minimums. Six plus/minus rows enforce the
  individual caps and remaining pool; Reset and Suggested allocation are explicit.
  A name and exact point spend are required before submitting.
- The existing authority creates the character and persists its sheet. The
  roster selects the returned identity. No client-owned resources, equipment,
  class tables or substitute creation backend were introduced.
- After an uncertain reply, editing and Back lock while Retry reuses the same
  draft and request ID. Native proof loses a reply after the real server commits,
  then verifies one roster identity, admission and persistence across sign-in.
- Login remains inert before startup completes, with unnamed credential fields
  and blocked native form navigation. Nationality is absent from presentation.

The [gameplay provenance](../../content/lands/first-expedition/gameplay.provenance.md)
distinguishes recovered bounds/caps/pools from authored suggested allocations and
provisional resource/loadout defaults. This UI change does not claim a recovered
original roll or attribute-dependent resource formulas.

## Initial verification and deployment

Evidence is retained outside the checkout under the September 9 entry-creation
execution directory. Native captures cover 1280×800 and 1920×1080. The dedicated
`--entry --proof` mode of `tools/run_pixel_temple.py` owns disposable three-engine
creation proof; the normal renderer and login-startup proofs retain their owners.
Observed results:

- All 538 browser unit tests passed. Typecheck, production build, documentation
  routes/links and public-boundary checks passed (final focused run: 28.683 s).
- All selected Python groups passed. The initial unit run caught an incorrect
  test assumption that its one-profile wire fixture contained all five product
  classes; the corrected assertion checks the fixture's actual immutable options.
- Chromium, Firefox and WebKit passed all five class sheets, caps, point pools,
  reset/suggested controls, Back, a shorter-window reachable footer, exact retry
  after a committed creation lost its reply, one new roster entry, and admission
  whose six server attributes exactly match the submitted sheet.
- Reconnect, sign-out and a fresh sign-in preserve the created roster entry in
  every engine. Captures at both target resolutions were opened for visual review.

Enabled delayed/failed startup passed in all three browsers. JavaScript-absent
and blocked native form submission passed in Chromium and Firefox; the WebKit
case is UNAVAILABLE, with an INCOMPLETE aggregate and exit 3. See the finding below.
The final proof-script/document follow-up passed the focused verification lane
in 27.019 s, including all 538 browser tests again.

The private preview was backed up and activated from source tree
`8aa1c9616bcf2adcb52a1f41599703af2ff539a2`, based on
`5d01572b10741a89dc379914b146a39157eb7aaf`. The immutable release includes the
matching verified pixel packet. Content and storage were unchanged; no world
migration or reset was required. Hosted Chromium, Firefox and WebKit each passed
root/index entry, all five creation choices, reset/suggested controls, Back,
Deck/desktop captures, a two-command movement round trip, reconnect and sign-out.
Hosted proof did not create test characters. Three characters, two accounts,
player positions/home locations/inventory/sheets/resources, item instances,
banks and lockers were preserved. Final readiness passed with zero open sessions.
The installed WebKit creation capture was opened for review.

External receipts: `verification.json`, `startup.json`, `deployment.json`,
`public/proof.json` and `final-state.json` in the entry-creation execution directory.
Source changes remain uncommitted on `main`; the prior PR #52 closeout remains
complete. This slice does not claim a new PR or merge.

## Findings and limits

The first native capture exposed that decoded packet images outlive their
revoked blob URLs. The menu now paints verified decoded pixels once into its
own static canvas, preserving integer enlargement without a duplicate fetch or
animation loop. A native roster sign-out also exposed a late-loaded pixel CSS
stacking override; the entry-state selector now keeps session controls reachable.
Both are repaired in this slice. The separate startup proof also assumed every
launcher returned a Browser; the trusted Firefox launcher returns a persistent
context. That caller now obtains the owning Browser before making JavaScript-off
and stalled-startup contexts, preserving certificate verification.

The layout is an implemented candidate, pending owner visual acceptance. Native
browser viewport proof is not a physical Steam Deck controller/input review.
Class-specific character portraits and nationality choices remain outside this
slice. The existing rendering detail and gameplay authority remain their owners.

### WebKit JavaScript-disabled capability finding

Owner: browser proof (`web/proof/login-startup-proof.mjs` and the native launcher).
The installed WebKitGTK engine crashes its page when JavaScript is disabled and
it renders a fixed-position section. An independent data-URL reproduction with
only `<section style="position:fixed"><button disabled>Sign in</button></section>`
also crashes; the same one-button page without positioning passes. No game code,
asset, request interception or authentication is involved in that reproduction.

The proof records this observed page crash as **UNAVAILABLE**, runs the enabled
startup checks separately, and exits 3 with an **INCOMPLETE** aggregate. A crash
cannot certify disabled credentials or blocked native submission. Chromium and
Firefox retain those checks. Retire this finding only when that minimal native
reproduction and the unchanged JavaScript-disabled product path both pass in
WebKit. The ordinary three-browser game flow retains its separately observed
PASS; JavaScript-disabled WebKit safety is not claimed.

## Manual-allocation and first-play follow-up

The owner continued character creation and the first-play flow, then required
evidence for any recommended allocation. The
[presentation owner](../presentation-direction.md#entry-and-character-creation)
records that ruling. The existing [provenance](../../content/lands/first-expedition/gameplay.provenance.md)
identifies suggestions as authored examples; recovered minimums, caps and pools
remain authoritative catalog inputs.

The bounded slice removes the Suggested allocation control and its form-side
consumer, retains manual allocation and Reset points, and updates native proof
to spend the pool through the real controls for every class. The proof must also
refuse a returned recommendation control. Review the existing entry/creation
lifecycle for concrete correctness findings and resolve any confirmed in-scope
defects with focused proof. Source changes from the initial slice are already
uncommitted in this checkout and remain part of the working tree.

Manual allocation passed all 538 browser tests, typecheck, build, documentation
and boundary checks (29.424 s focused run). Chromium, Firefox and WebKit each
passed all five manual class sheets, exact creation retry and durable admission.
Hosted proof passed in all three engines after activation of source tree
`d755f9f13d900ab35512e9ee7bfeece971d4a7cc`, based on the same commit as above.
Three characters, two accounts, sheets, resources, locations, inventory, banks
and lockers were preserved; final readiness had zero open connections.
External evidence lives in the September 9 entry-followup execution directory:
`native/verification.json`, `deployment.json`, `public/proof.json` and
`final-state.json`. The native creation capture was opened for review.

The first hosted request exposed a separate reboot failure: an obsolete,
separate-port pixel-study proxy still referenced a deleted temporary certificate.
That prevented the system proxy from starting after reboot. Its unused
configuration was retained outside the active configuration directory; config
validation, startup and HTTPS/gameplay then passed. The permanent preview uses
its durable installation certificate. Lesson: temporary proof endpoints must not
remain in a persistent proxy's startup configuration.

Two earlier diagnostic proof callers also depended on the retired button.
They now share a helper that reads the actual creation response and spends
points through the selected class's real controls. Their runtime verification
is recorded with the continuation below. Catalog, resource formulas, saved
characters and equipment remain outside this presentation change.

## Actor interaction continuation

The next first-play audit found that the pixel actor panel selected only resident
and service groups. Consequently the authority's creature-targeted combat offers
were absent from that surface after removal of the temporary HUD. The
[browser owner](../browser-client.md#temple-resident-interaction-direction)
now routes exact actor-targeted physical and spell offers through the existing
panel and dispatch owner. Unit proof covers target identity, blocked offers,
withdrawal and ambiguity; native temple proof adds a nonmutating creature-menu
open and an attack preserving the real offered target and authorization.
Verification passed: 540 browser unit tests, typecheck, build, docs and boundary
checks (28.248 seconds); the native temple/combat loop passed in all three
engines, and the migrated manual-creation service proof passed all nine runs.
The first combat proof stood beside the creature while expecting a same-square
fight offer; correcting its position made it exercise the intended action.
The authority's original adjacent offer was correct.

Source tree `6c0dd60a2ba45e2846d850d3cc823d8e2d1142cc` is privately deployed.
Hosted entry, movement, reconnect and saved-state checks passed in all three
engines. Three characters, two accounts, resources, inventory, banks and lockers
were preserved. Receipts are in the external entry-followup execution directory's
`combat-native-r2`, `services-native` and `actor-deployment` subdirectories.
The shared allocation helper's expedition caller still needs its complete
expedition run during the subsequent dungeon-geography migration.
