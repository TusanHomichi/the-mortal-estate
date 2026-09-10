import { expect, it } from "vitest";
import { allocationState } from "../src/play/creationAllocation";
import type { CreationOption } from "../src/play/control";

const minimum = { strength:14, dexterity:14, constitution:14, intelligence:12, wisdom:12, charisma:10 };
const maximum = { strength:18, dexterity:17, constitution:18, intelligence:16, wisdom:16, charisma:16 };
const option: CreationOption = { profile_id:"creation/fighter", class_name:"Fighter", nationality:"unused",
  minimum, maximum, attribute_points:15, suggested:{strength:18,dexterity:17,constitution:18,intelligence:14,wisdom:14,charisma:10} };

it("requires every unassigned point and respects the class's individual bounds", () => {
  expect(allocationState(option, minimum)).toEqual({remaining:15,valid:false});
  expect(allocationState(option, option.suggested)).toEqual({remaining:0,valid:true});
  expect(allocationState(option, {...option.suggested, charisma:11})).toEqual({remaining:-1,valid:false});
  // Equal total alone must not admit an above-cap or below-base stat.
  expect(allocationState(option, {...option.suggested, strength:19,intelligence:13}).valid).toBe(false);
  expect(allocationState(option, {...option.suggested, charisma:9,intelligence:15}).valid).toBe(false);
  expect(allocationState(option, {...option.suggested, strength:17.5,intelligence:14.5}).valid).toBe(false);
});
