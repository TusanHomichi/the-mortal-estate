import { describe, it, expect } from "vitest";
import { spawnSync } from "node:child_process";
import { mkdtempSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

describe("playable renderer cutover",()=>{
  for(const mode of ["first-expedition","pixel-temple","unknown"]){
    it(`refuses ${mode} before creating an output artifact`,()=>{
      const root=mkdtempSync(path.join(tmpdir(),"tme-play-refusal-"));
      try{
        const output=path.join(root,"bundle");
        const result=spawnSync(process.execPath,[path.resolve("proof/build-play.mjs"),output,mode],{encoding:"utf8"});
        expect(result.status).not.toBe(0);
        expect(result.stderr).toContain("Retired or unknown play presentation build mode");
        expect(existsSync(output)).toBe(false);
      }finally{rmSync(root,{recursive:true,force:true});}
    });
  }
});
