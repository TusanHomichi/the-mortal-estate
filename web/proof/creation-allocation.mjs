// Shared manual class allocation for native proofs.
// Creation has no recommended presets: every point is spent through the real
// Increase controls from the selected server option's own bounds.
import assert from "node:assert/strict";

export const creationAttributes = ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"];
const title = key => key[0].toUpperCase() + key.slice(1);
const button = (page, name) => page.getByRole("button", { name, exact: true });

export async function openCreation(page, { className } = {}) {
  const response = page.waitForResponse(row => row.url().endsWith("/characters/creation"));
  await button(page, "Create a new character").click();
  const options = (await (await response).json()).options;
  await page.locator("#creation-form").waitFor({ state: "visible" });
  assert.equal(await page.locator("#creation-profile input").count(), options.length, "creation UI shows every server option");
  assert.equal(await page.getByRole("button", { name: /suggested|recommended/i }).count(), 0, "unsubstantiated allocation presets are absent");
  if (className) await page.getByRole("radio", { name: className, exact: true }).check();
  const selected = await page.locator("#creation-profile input:checked").getAttribute("value");
  const option = options.find(row => row.profile_id === selected);
  assert(option, "the checked creation profile has no server option");
  return option;
}

export async function allocateManually(page, option) {
  await button(page, "Reset points").click();
  let remaining = option.attribute_points;
  const values = { ...option.minimum };
  for (const key of creationAttributes) {
    const spend = Math.min(remaining, option.maximum[key] - option.minimum[key]);
    for (let n = 0; n < spend; n++) await button(page, `Increase ${title(key)}`).click();
    values[key] += spend; remaining -= spend;
  }
  assert.equal(remaining, 0, "the server pool fits inside the option bounds");
  for (const key of creationAttributes) {
    assert.equal(Number(await page.locator(`#creation-${key}`).textContent()), values[key], `${key} matches manual spending`);
    assert(await button(page, `Increase ${title(key)}`).isDisabled(), `${key} is capped after manual spending`);
  }
  assert.equal(await page.locator("#creation-budget").textContent(), "0 points remaining");
  return values;
}

export async function createCharacterManually(page, { className, name } = {}) {
  const option = await openCreation(page, { className });
  await page.locator("#creation-name").fill(name);
  const values = await allocateManually(page, option);
  assert(await button(page, "Create character").isEnabled(), "manual allocation satisfies creation");
  await button(page, "Create character").click();
  return { option, values };
}
