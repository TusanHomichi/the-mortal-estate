import { describe, expect, it } from "vitest";
import { projectPixel, unprojectPixel, pixelSceneryRect, TEMPLE_PROJECTION } from "../src/play/pixelGeometry";
import {pixelCamera} from "../src/play/pixelViewport";
import { exteriorLayerCovers } from "../src/play/pixelExterior";

describe("pixel exterior projection and occlusion",()=>{
  it("keeps pointing and grounded scenery aligned throughout a moving camera",()=>{
    for(const focus of [{x:13,y:12},{x:14.35,y:11.25},{x:16,y:9}]) {
      const projection=pixelCamera(TEMPLE_PROJECTION,focus,{x:640,y:400},{x:1254,y:1254});
      expect(projection.step).toEqual(TEMPLE_PROJECTION.step);
      for(const cell of [{x:13,y:10},{x:17,y:9},{x:12,y:12}])expect(unprojectPixel(projectPixel(cell,projection),projection)).toEqual(cell);
      const root=projectPixel({x:17,y:10},projection);
      const rect=pixelSceneryRect(root,{x:.48,y:.97},176,1.02);
      expect(rect.x+rect.width*.48).toBeCloseTo(root.x);
      expect(rect.y+rect.height*.97).toBeCloseTo(root.y);
      expect(rect.depth).toBe(root.y);
    }
  });
  it("occludes by actual silhouette including transparent canopy holes and half-open boundaries",()=>{
    const alpha=new Uint8ClampedArray(16);alpha[3]=255;alpha[15]=255;
    const layer={id:"tree",x:10,y:20,width:100,height:80,depth:100,
      source:{image:{} as HTMLImageElement,width:2,height:2,alpha}};
    expect(exteriorLayerCovers(layer,{x:15,y:25})).toBe(true);
    expect(exteriorLayerCovers(layer,{x:65,y:25})).toBe(false);
    expect(exteriorLayerCovers(layer,{x:65,y:75})).toBe(true);
    for(const point of [{x:9,y:25},{x:110,y:25},{x:15,y:100}])expect(exteriorLayerCovers(layer,point)).toBe(false);
  });
});
