// Coordinate the runner that owns the server with the browser proof that drives it.
//
// A browser proof cannot replace the serving process it is talking to, and the
// runner cannot see the state the proof is standing in. The two halves therefore
// meet on files: the proof writes a request naming what it wants observed, the
// runner answers with evidence it read from the authoritative side, and the
// proof keeps going. Every request is one file pair under the proof output
// directory, named `<engine>-<journey>-<kind>-request.json` and
// `-complete.json`, and each is answered exactly once.
//
// Only addresses and the runner's own evidence cross the boundary. No gameplay
// fact is injected in either direction, and the proof re-observes the state
// through the ordinary client.

import {existsSync} from 'node:fs';
import {readFile, writeFile} from 'node:fs/promises';

/**
 * Ask the runner for one piece of evidence and wait for its answer.
 *
 * `kind` names the handshake — `restart` or `ledger` — and is also the file
 * suffix, so the two directions cannot collide.
 */
export async function requestRunner(configuration, kind, payload, eventually, timeout = 300000) {
  const stem = `${configuration.output}/${configuration.engine}-${configuration.journey ?? 'death'}-${kind}`;
  const complete = `${stem}-complete.json`;
  await writeFile(`${stem}-request.json`, JSON.stringify(payload));
  await eventually(() => existsSync(complete), timeout);
  return JSON.parse(await readFile(complete, 'utf8'));
}

/**
 * Ask the runner to restart the serving process and return its evidence.
 *
 * `expected` is the proof's own account of the state it is standing in, and is
 * what the runner checks the durable state against before it replaces anything.
 * `configuration` is mutated in place: `origin` becomes the restarted origin so
 * the caller's later reconnects use the process that is actually listening.
 */
export async function restartServingProcess(configuration, expected, eventually) {
  const restart = await requestRunner(configuration, 'restart', expected, eventually);
  if (restart.origin) configuration.origin = restart.origin;
  return restart;
}

/**
 * Ask the runner to read, or to audit, the authoritative item and coin ledger.
 *
 * `action` is `capture` (remember the ledger under `label`) or `audit` (compare
 * the current ledger with the one remembered under `label`). The comparison is
 * the runner's, over the stored checkpoint: the browser never assembles its own
 * inventory model, it only asserts on the authoritative verdict.
 */
export async function checkpointLedger(configuration, action, label, eventually) {
  return requestRunner(configuration, 'ledger', {action, label}, eventually);
}
