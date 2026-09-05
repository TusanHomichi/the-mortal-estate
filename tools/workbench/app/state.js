/* The one state object, and the three questions everyone asks of it.
 *
 * This module sits at the bottom of the import graph and imports nothing. That
 * is not tidiness: every other module reads this object and most of them write
 * to it, so anything it depended on would be depended on by the whole page.
 *
 * It is deliberately one object rather than a scatter of module-level
 * variables. The two views share a camera, a tool, and a pending selection, and
 * a second copy of any of those is a second answer to the same question — the
 * exact failure the server side of this tool refuses to allow, held to on the
 * browser side for the same reason.
 *
 * The lookups live beside the state they read. `capturing()` decides the branch
 * at nearly every point in this page, and every module asking it of one object
 * is what keeps the two views from drifting into two behaviours.
 *
 * The candidate is held here for the same reason and with one addition: it is
 * DERIVED state. The staged log is what the session keeps; the candidate is a
 * function of it, produced by the server and replaced whole by the next preview.
 * Nothing on this page edits it, and nothing keeps a second copy of it.
 */

export const LOGICAL = "logical";
export const CAPTURE = "capture";
export const CANDIDATE = "candidate";

/* The server's operation-class names, written once because two modules say them:
 * the panel that stages an operation and the module that builds its fields. The
 * third class, dressing, is not here because nothing on this page can stage one
 * — it ships zero verbs, and the panel shows the ruling instead.
 */
export const TRUTH = "truth";
export const ASSET = "asset";

export const state = {
  view: LOGICAL,
  projection: null,
  member: null,
  captures: [],
  captureAvailable: false,
  captureInProgress: false,
  captureId: null,
  captureImage: null,
  tool: "click",
  scale: 26,
  origin: { x: 0, y: 0 },
  palette: new Map(), // terrain class -> hue, built once from the projection
  pending: null,      // the cells the current gesture covers (logical view)
  covered: null,      // the target rectangles it covers (capture view)
  gesture: null,      // the request body the server would be sent
  canvasRect: null,
  drag: null,
  panning: false,
  spaceHeld: false,
  selections: [],     // the packet ids this session holds, as the server lists them
  staged: [],         // the staged operations, as the server last returned them
  vocabulary: null,   // the operation vocabulary, fetched once and cached
  candidate: null,    // the candidate projection document, or null
  candidateDiff: null, // member -> Set of "x,y", the outlined cells
  applied: null,      // the last accepted Apply answer, which acceptance names
};

export function capturing() {
  return state.view === CAPTURE;
}

export function previewingCandidate() {
  return state.view === CANDIDATE;
}

export function currentCapture() {
  return state.captures.find((row) => row.capture_id === state.captureId) || null;
}

/* The member under the pointer, out of whichever document the current view is
 * of. A candidate is an edit to a member, not a different member, so the two
 * documents carry the same shape and the same member names — which is exactly
 * why one lookup serves both views and the drawing code below never learns that
 * a candidate exists.
 */
export function currentMember() {
  const document = previewingCandidate() ? state.candidate : state.projection;
  if (!document) return null;
  return document.members.find((member) => member.member === state.member);
}

/* The same member out of the accepted projection, whatever the current view is.
 * The diff needs both sides and must never take the candidate for the accepted
 * land by asking the view-aware lookup twice.
 */
export function acceptedMember(name) {
  if (!state.projection) return null;
  return state.projection.members.find((member) => member.member === name);
}
