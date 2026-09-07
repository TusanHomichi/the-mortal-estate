import { readFileSync } from "node:fs";
import path from "node:path";
import { beforeAll, expect, it } from "vitest";
import { WireCodec } from "../src/authoritative/codec";
import { actionGroups, offeredIntent, withGoldAmount } from "../src/authoritative/gameplay";
import type { Envelope } from "../src/authoritative/state";

let envelope: Envelope;
beforeAll(async () => {
  const root = path.resolve(import.meta.dirname, "../..");
  const codec = await WireCodec.create(readFileSync(path.join(root, "target/wasm32-unknown-unknown/release/tme_protocol.wasm")));
  const cases = JSON.parse(readFileSync(path.join(root, "tests/fixtures/wire/server_envelope.json"), "utf8")).cases;
  envelope = codec.decode<Envelope>("server_envelope", cases.find((row: { case_id: string }) => row.case_id === "accept_server_welcome").input_utf8);
});

it("exposes every service capability from a real codec-validated frame without altering its intents", () => {
  const before = structuredClone(envelope.frame), groups = actionGroups(envelope.frame);
  const intents = groups.flatMap(group => group.actions).map(action => action.intent?.kind);
  for (const kind of ["train", "critique", "learn_spell", "promote_class", "commit_service_transaction",
    "buy_from_merchant", "sell_to_merchant", "use_item_service", "use_restoration_service",
    "deposit_bank_gold", "withdraw_bank_gold", "deposit_locker_item", "withdraw_locker_item", "move_item", "search_corpse"]) {
    expect(intents).toContain(kind);
  }
  for (const group of groups) for (const action of group.actions.filter(row => row.enabled && row.intent)) {
    expect(offeredIntent(envelope.frame, group.key, action.id)).toBe(action.intent);
  }
  expect(envelope.frame).toEqual(before);
});

it("refuses missing, disabled, null and ambiguous choices without falling back to another action", () => {
  const frame = structuredClone(envelope.frame), action = frame.action_options[0]!;
  expect(offeredIntent(frame, "absent", action.id)).toBeNull();
  expect(offeredIntent(frame, "character", "absent")).toBeNull();
  action.enabled = false; expect(offeredIntent(frame, "character", action.id)).toBeNull();
  action.enabled = true; const intent = action.intent; action.intent = null;
  expect(offeredIntent(frame, "character", action.id)).toBeNull();
  action.intent = intent; frame.action_options.push(structuredClone(action));
  expect(offeredIntent(frame, "character", action.id)).toBeNull();
});

it("retains exact wide quantities and exposes server truncation", () => {
  const frame = structuredClone(envelope.frame);
  const bank = frame.services_here[0]!.capabilities.find(row => row.kind === "bank")!;
  if (bank.kind !== "bank") throw new Error("bank fixture required");
  bank.balance_gold = "9007199254740993"; frame.action_options_truncated = true;
  const groups = actionGroups(frame);
  expect(groups[0]!.facts).toContain("Bank balance: 9007199254740993 gold");
  expect(groups.find(row => row.key === "character")!.facts).toContain("The server returned a limited action list.");
});

it("changes only an entered gold quantity, preserving exact integers and the offered target", () => {
  const intent = { kind: "withdraw_bank_gold", service_id: "counter", capability_id: "bank", amount: "100" };
  expect(withGoldAmount(intent, "9007199254740993")).toEqual({ ...intent, amount: "9007199254740993" });
  expect(intent.amount).toBe("100");
  for (const amount of ["0", "-1", "1.5", "1e3", " 1", "01", "9223372036854775808"]) expect(withGoldAmount(intent, amount)).toBeNull();
  expect(withGoldAmount({ kind: "wait" }, "1")).toBeNull();
  const move = { kind: "move_gold", source: { kind: "carried", position: "sack" }, destination: { kind: "ground_here" }, quantity: { kind: "all" } };
  expect(withGoldAmount(move, "30")).toEqual({ ...move, quantity: { kind: "exact", amount: "30" } });
});
