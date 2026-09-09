import { describe,it,expect } from "vitest";
import { pixelRouteLength,pixelTravelDuration,samplePixelTravel,pixelStrideFrame } from "../src/play/pixelMotion";

describe("pixel locomotion",()=>{
  const route=[{x:0,y:0},{x:1,y:0},{x:1,y:1},{x:2,y:1}];
  it("follows each committed corner at distance-based speed",()=>{
    const duration=pixelTravelDuration(route);
    expect(duration).toBe(1680);
    expect(samplePixelTravel(route,280,duration).at).toEqual({x:.5,y:0});
    expect(samplePixelTravel(route,840,duration).at).toEqual({x:1,y:.5});
    expect(samplePixelTravel(route,1400,duration).at).toEqual({x:1.5,y:1});
    expect(samplePixelTravel(route,duration,duration).done).toBe(true);
  });
  it("accounts for diagonal distance and arrives within observed remaining time",()=>{
    const diagonal=[{x:0,y:0},{x:1,y:1},{x:2,y:2},{x:3,y:3}];
    expect(pixelRouteLength(diagonal)).toBeCloseTo(Math.sqrt(2)*3);
    expect(pixelTravelDuration(diagonal)).toBeLessThan(2500);
    const duration=pixelTravelDuration(diagonal,350);
    expect(duration).toBe(350);
    expect(samplePixelTravel(diagonal,350,duration).at).toEqual({x:3,y:3});
    expect(pixelTravelDuration(diagonal,-1)).toBe(0);
  });
  it("keeps gait phase with travel even when the remaining interval compresses",()=>{
    const normal=samplePixelTravel(route,420,1680),compressed=samplePixelTravel(route,210,840);
    expect(normal.distance).toBe(compressed.distance);
    expect(pixelStrideFrame(normal.distance,8)).toBe(pixelStrideFrame(compressed.distance,8));
    expect(samplePixelTravel(route,-.5,1680).at).toEqual(route[0]);
    expect(pixelStrideFrame(-.5,8)).toBe(0);
    expect(samplePixelTravel(route,0,0).at).toEqual(route.at(-1));
  });
});
