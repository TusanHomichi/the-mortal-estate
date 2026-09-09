import { describe, expect, it } from "vitest";
import { parseCoastalProfile, sampleCoastalProfile } from "../src/coastalProfile";

const profile = { offshore_depth: 5, shore_slope: .5, zones: [
  { centre: [0,0], radius: [4,4], depth: .3, exposure: .1 },
  { centre: [8,0], radius: [3,3], depth: 3, exposure: .85 },
] };

describe("authored coastal depth and exposure", () => {
  it("keeps a broad shelf shallow and a near-land channel deep independently of shore distance", () => {
    const p = parseCoastalProfile(profile);
    expect(sampleCoastalProfile(p,0,0,10).depth).toBeCloseTo(.3);
    expect(sampleCoastalProfile(p,0,0,10).exposure).toBeCloseTo(.1);
    expect(sampleCoastalProfile(p,8,0,.7).depth).toBe(3);
    expect(sampleCoastalProfile(p,8,0,.7).exposure).toBe(.85);
    expect(sampleCoastalProfile(p,30,0,100)).toEqual({ depth: 5, exposure: 1 });
    const a=sampleCoastalProfile(p,4-1e-6,0,10),b=sampleCoastalProfile(p,4+1e-6,0,10);
    expect(Math.abs(a.depth-b.depth)).toBeLessThan(1e-8);
    expect(Math.abs(a.exposure-b.exposure)).toBeLessThan(1e-8);
  });
  it("refuses malformed, unbounded and unknown scenic inputs", () => {
    for (const value of [null, {...profile,unknown:1}, {...profile,offshore_depth:NaN},
      {...profile,shore_slope:0}, {...profile,zones:Array(17).fill(profile.zones[0])},
      {...profile,zones:[{...profile.zones[0],radius:[0,4]}]},
      {...profile,zones:[{...profile.zones[0],exposure:1.1}]},
      {...profile,zones:[{...profile.zones[0],depth:-1}]}]) {
      expect(() => parseCoastalProfile(value)).toThrow(/coastal/);
    }
  });
});
