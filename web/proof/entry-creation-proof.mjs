// Native entry flow, recovered allocation constraints, and durable creation.
// Runs only against a disposable authority supplied by the pixel proof harness.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { launchProofBrowser, PROOF_ENGINES } from "./serve.mjs";
let input=""; for await(const chunk of process.stdin) input+=chunk;
const config=JSON.parse(input); input="";
const engine=PROOF_ENGINES[config.engine]; if(!engine)throw Error("Unrostered engine");
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
const keys=["strength","dexterity","constitution","intelligence","wisdom","charisma"];
const title=key=>key[0].toUpperCase()+key.slice(1);
const errors=[],requests=[],captures=[]; let frame,options,created;
try {
  const context=launched.context || await launched.browser.newContext();
  const page=await context.newPage(); await page.setViewportSize({width:1280,height:800});
  page.on("pageerror",error=>errors.push(error.message));
  page.on("request",request=>{if(request.url().endsWith("/characters/create"))requests.push(JSON.parse(request.postData()));});
  page.on("websocket",socket=>socket.on("framereceived",event=>{const row=JSON.parse(String(event.payload));if(row.frame)frame=row.frame;}));
  const wait=(fn)=>page.waitForFunction(fn,undefined,{polling:30,timeout:45000});
  const button=name=>page.getByRole("button",{name,exact:true});
  // Exercise the player's controls using catalog bounds, without an authored preset.
  const allocateManually=async option=>{
    await button("Reset points").click();
    let remaining=option.attribute_points;
    const values={...option.minimum};
    for(const key of keys){
      const spend=Math.min(remaining,option.maximum[key]-option.minimum[key]);
      for(let n=0;n<spend;n++)await button(`Increase ${title(key)}`).click();
      values[key]+=spend;remaining-=spend;
    }
    assert.equal(remaining,0,"catalog pool can be spent within its bounds");
    for(const key of keys){assert.equal(Number(await page.locator(`#creation-${key}`).textContent()),values[key]);assert(await button(`Increase ${title(key)}`).isDisabled());}
    assert.equal(await page.locator("#creation-budget").textContent(),"0 points remaining");
  };
  const capture=async name=>{
    for(const [width,height] of [[1280,800],[1920,1080]]) {
      await page.setViewportSize({width,height});
      const box=await page.locator(".entry-frame").boundingBox();
      assert(box.x>=0&&box.y>=0&&box.x+box.width<=width&&box.y+box.height<=height,`${name} fits ${width}`);
      const file=`${config.engine}-${name}-${width}.png`;await page.screenshot({path:`${config.output}/${file}`});captures.push(file);
    }
    await page.setViewportSize({width:1280,height:800});
  };
  const login=async()=>{await page.locator("#username").fill(config.username);await page.locator("#password").fill(config.password);await button("Sign in").click();await wait(()=>document.body.dataset.entryScreen==="roster");};
  await page.goto(config.origin+"/");await wait(()=>document.body.dataset.playReady==="true");
  await capture("sign-in");await login();await capture("roster");
  const initialRoster=await page.locator("#character input").count();
  const response=page.waitForResponse(response=>response.url().endsWith("/characters/creation"));
  await button("Create a new character").click();options=(await (await response).json()).options;
  await wait(()=>document.body.dataset.entryScreen==="creation");
  assert.equal(options.length,5);assert.equal(await page.locator("#creation-profile input").count(),5);
  assert.equal(await page.getByRole("button",{name:/suggested|recommended/i}).count(),0,"unsubstantiated allocation presets are absent");
  assert(!await page.getByText(options[0].nationality,{exact:true}).count(),"nationality is deferred");
  for(const option of options) {
    await page.getByRole("radio",{name:option.class_name,exact:true}).check();
    assert.equal(await page.locator("#creation-budget").textContent(),`${option.attribute_points} points remaining`);
    for(const key of keys) {
      assert.equal(Number(await page.locator(`#creation-${key}`).textContent()),option.minimum[key]);
      assert(await button(`Decrease ${title(key)}`).isDisabled());
    }
    await page.locator("#creation-name").fill("New Arrival");assert(await button("Create character").isDisabled());
    for(let n=option.minimum.strength;n<option.maximum.strength;n++)await button("Increase Strength").click();
    assert(await button("Increase Strength").isDisabled());
    await allocateManually(option);
    assert(await button("Create character").isEnabled());
  }
  await button("Back").click();assert.equal(await page.locator("#character input").count(),initialRoster);
  await button("Create a new character").click();await wait(()=>document.body.dataset.entryScreen==="creation");
  await page.getByRole("radio",{name:"Martial Artist",exact:true}).check();
  const martialArtist=options.find(option=>option.class_name==="Martial Artist");assert(martialArtist);
  await page.locator("#creation-name").fill("New Arrival");await allocateManually(martialArtist);
  await button("Decrease Strength").click();assert(await button("Create character").isDisabled());
  await capture("creation");
  await page.setViewportSize({width:1280,height:650});
  await allocateManually(martialArtist);
  await button("Back").scrollIntoViewIfNeeded();
  const back=await button("Back").boundingBox();assert(back.y>=0&&back.y+back.height<=650,"short-window footer remains reachable");
  await page.setViewportSize({width:1280,height:800});
  // The authority commits, but its reply is lost. The UI must retain one draft/id.
  let first=true;
  await page.route("**/characters/create",async route=>{
    if(first){first=false;const response=await route.fetch().catch(()=>{throw Error("Injected-loss upstream request failed");});assert(response.ok());created=(await response.json()).character;await route.abort("failed");}
    else await route.continue();
  });
  await button("Create character").click();await button("Retry creation").waitFor();
  assert(await button("Back").isDisabled());assert(await page.locator("#creation-name").isDisabled());
  assert(await page.locator("#creation-profile input").first().isDisabled());await button("Retry creation").click();
  await wait(()=>document.body.dataset.entryScreen==="roster");
  assert.equal(requests.length,2);assert.deepEqual(requests[0],requests[1]);
  assert.equal(await page.locator("#character input").count(),initialRoster+1);
  assert.equal(await page.locator("#character input:checked").inputValue(),created.character_id);
  await button("Enter world").click();await wait(()=>document.body.dataset.phase==="playing");
  assert(await page.locator("#entry").isHidden());assert(frame);
  assert.deepEqual(frame.character.attributes,requests[0].draft.attributes);
  await button("Reconnect").click();await wait(()=>document.body.dataset.phase==="playing");
  await button("Sign out").click();await wait(()=>document.body.dataset.phase==="signed_out");
  await login();assert.equal(await page.locator("#character input").count(),initialRoster+1);
  await page.getByRole("radio",{name:"New Arrival",exact:true}).check();
  await button("Sign out").click();await wait(()=>document.body.dataset.phase==="signed_out");
  assert.deepEqual(errors,[]);
  await writeFile(`${config.output}/${config.engine}-entry-creation.json`,JSON.stringify({verdict:"PASS",engine:config.engine,classes:options.map(row=>row.class_name),captures,exact_retry:true,durable_roster:true},null,2)+"\n");
} finally {await launched.stop();}
