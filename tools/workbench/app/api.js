/* Every request this page makes, and the one place a refusal is shown.
 *
 * One function per route, and no route string anywhere else in the app. The
 * server owns the answer to every question worth asking — which cells a gesture
 * covers, what identity they carry, what a capture holds — so the browser's
 * entire relationship with it is this file. Keeping it that way is what makes
 * "the browser resolves nothing" a claim an owner can check by reading one
 * module instead of grepping a page.
 *
 * Refusal handling lives here rather than at the call sites because there is
 * exactly one right thing to do with a refused request: show the reason the
 * server gave, verbatim, and leave the caller's own state standing. A caller
 * that wants to ignore the failure catches the throw; it never has to invent an
 * error message, and it never has to guess what went wrong.
 */

async function request(path, options) {
  const response = await fetch(path, options);
  const payload = await response.json().catch(() => ({
    error: "unreadable",
    detail: `${response.status} ${response.statusText}`,
  }));
  if (!response.ok) {
    refuse(payload);
    throw new Error(payload.detail || "request refused");
  }
  clearRefusal();
  return payload;
}

function post(path, body) {
  return request(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

export function refuse(payload) {
  const box = document.getElementById("refusal");
  box.hidden = false;
  box.textContent = `${(payload.error || "refused").toUpperCase()}: ${payload.detail || ""}`;
}

function clearRefusal() {
  document.getElementById("refusal").hidden = true;
}

/* -------------------------------------------------------------- the routes */

export function readState() {
  return request("/api/state");
}

export function readProjection() {
  return request("/api/projection");
}

export function previewLogicalGesture(body) {
  return post("/api/preview", body);
}

export function previewCaptureGesture(body) {
  return post("/api/capture/preview", body);
}

export function writeLogicalSelection(body) {
  return post("/api/selection", body);
}

export function writeCaptureSelection(body) {
  return post("/api/capture/selection", body);
}

export function readPacket(identifier) {
  return request(`/api/packet?id=${encodeURIComponent(identifier)}`);
}

export function stageOperation(body) {
  return post("/api/stage", body);
}

export function retractOperation(body) {
  return post("/api/retract", body);
}

export function previewCandidateGesture(body) {
  return post("/api/candidate/preview", body);
}

export function acceptCandidate(body) {
  return post("/api/accept", body);
}

/* The one route that costs seconds, because it runs the real client. It is a
 * route of its own on the server for that reason, and a function of its own
 * here so the cost is visible at every call site.
 */


/* The three routes that run the authoring compiler. They are grouped and
 * labelled for the same reason the capture route is: each costs a moment rather
 * than milliseconds, each starts a program, and a call site that did not know
 * that would put one of them on the pointing path — where nothing at all is
 * allowed to start.
 */
export function readVocabulary() {
  return request("/api/operations");
}

export function previewCandidate() {
  return post("/api/candidate", {});
}

export function applyStaged(body) {
  return post("/api/apply", body);
}

/* Not a fetch — the browser loads this one through an <img> — but it is still a
 * route, and routes live in this file or the claim above stops being true.
 */
export function captureImagePath(identifier) {
  return `/api/capture/image?id=${encodeURIComponent(identifier)}`;
}
