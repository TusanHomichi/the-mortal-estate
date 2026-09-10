import { readFileSync } from "node:fs";
import path from "node:path";
import { beforeAll, expect, it } from "vitest";
import { WireCodec } from "../src/authoritative/codec";
import { PlayControl, type CreationDraft, type Transport } from "../src/play/control";

const root = path.resolve(import.meta.dirname, "../..");
const fixture = (name: string) => JSON.parse(JSON.parse(readFileSync(path.join(root, `tests/fixtures/wire/${name}.json`), "utf8")).cases.find((row: { expect: string }) => row.expect === "accept").input_utf8);
let codec: WireCodec;
beforeAll(async () => { codec = await WireCodec.create(readFileSync(path.join(root, "target/wasm32-unknown-unknown/release/tme_protocol.wasm"))); });

it.each(["lost_response", "unavailable"])("retries identical creation after %s without a second identity", async failure => {
  const requests: unknown[] = [];
  let attempt = 0, identities = 0;
  const transport: Transport = {
    uuid: () => { ++identities; return "11111111-1111-4111-8111-111111111111"; },
    socket: () => { throw new Error("creation cannot admit a socket"); },
    fetch: (async (url, options) => {
      const route = new URL(String(url)).pathname;
      if (route.endsWith("/login")) return new Response(JSON.stringify(fixture("login_response_v1")));
      if (route.endsWith("/creation")) return new Response(JSON.stringify(fixture("character_creation_options_v1")));
      if (route.endsWith("/create")) {
        requests.push(JSON.parse(String(options!.body)));
        if (++attempt === 1) {
          if (failure === "lost_response") throw new Error("connection closed");
          return new Response(JSON.stringify({ code: "unavailable" }), { status: 503 });
        }
        return new Response(JSON.stringify(fixture("character_created_v1")));
      }
      throw new Error("unexpected route");
    }) as typeof fetch,
  };
  const control = new PlayControl(codec, "https://localhost:18743", () => {}, transport);
  try {
    await control.login("tester", "long enough password"); await control.openCreation();
    const option = control.view.creationOptions[0]!;
    const draft: CreationDraft = { profile_id: option.profile_id, display_name: "New Arrival", attributes: option.suggested };
    await expect(control.createCharacter(draft)).rejects.toThrow();
    await expect(control.createCharacter({ ...draft, display_name: "Another" })).rejects.toThrow("unchanged");
    expect(requests).toHaveLength(1);
    expect(control.view.creationRetry).toEqual(draft);
    control.closeCreation();
    expect(control.view.creationOptions).toEqual(fixture("character_creation_options_v1").options);
    expect(control.view.creationRetry).toEqual(draft);
    await control.createCharacter(draft);
    expect(requests[1]).toEqual(requests[0]); expect(identities).toBe(1);
    expect(control.view.createdCharacterId).toBe(fixture("character_created_v1").character.character_id);
    expect(control.view.phase).toBe("selecting"); expect(control.view.creationOptions).toEqual([]);
    expect(control.view.creationRetry).toBeNull();
  } finally { control.dispose(); }
  expect(control.view.createdCharacterId).toBeNull(); expect(control.view.characters).toEqual([]);
});
