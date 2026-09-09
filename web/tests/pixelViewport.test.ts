import {describe,it,expect} from "vitest";
import {pixelViewport,pixelCamera} from "../src/play/pixelViewport";
import {projectPixel,unprojectPixel} from "../src/play/pixelGeometry";

describe("integer playfield and camera",()=>{
  it("fills Deck and desktop using exact integer pixels",()=>{
    expect(pixelViewport(1280,800)).toEqual({width:640,height:400,scale:2});
    expect(pixelViewport(1920,1080)).toEqual({width:640,height:360,scale:3});
    for(const [w,h]of [[360,640],[1279,799],[1400,1200],[2561,1441]]){
      const v=pixelViewport(w!,h!);expect(Number.isInteger(v.scale)).toBe(true);
      expect(v.width*v.scale).toBeLessThanOrEqual(w!);expect(v.height*v.scale).toBeLessThanOrEqual(h!);
      expect(w!-v.width*v.scale).toBeLessThan(v.scale);
    }
  });
  it("keeps the same cell target across scrolling, letterboxing and both display sizes",()=>{
    const base={origin:{x:26,y:20},step:{x:48,y:32}};
    for(const screen of [[1280,800],[1920,1080],[360,640]])for(const focus of [{x:0,y:0},{x:13.25,y:15.7},{x:26,y:35}]){
      const v=pixelViewport(screen[0]!,screen[1]!);
      const p=pixelCamera(base,focus,{x:v.width,y:v.height},{x:1254,y:1254});
      expect(Number.isInteger(p.origin.x)).toBe(true);expect(Number.isInteger(p.origin.y)).toBe(true);
      for(const cell of [{x:1,y:2},{x:13,y:10},{x:20,y:31}])expect(unprojectPixel(projectPixel(cell,p),p)).toEqual(cell);
    }
  });
});
