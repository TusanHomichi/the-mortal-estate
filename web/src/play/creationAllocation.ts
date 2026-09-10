import type { Attributes, CreationOption } from "./control";

export const creationAttributes = ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"] as const;

/** Presentation of server-provided allocation bounds; final legality stays in rules. */
export function allocationState(option: CreationOption, values: Attributes) {
  const remaining = option.attribute_points - creationAttributes.reduce((sum, key) => sum + values[key] - option.minimum[key], 0);
  const withinBounds = creationAttributes.every(key => Number.isInteger(values[key]) && values[key] >= option.minimum[key] && values[key] <= option.maximum[key]);
  return { remaining, valid: withinBounds && remaining === 0 };
}
