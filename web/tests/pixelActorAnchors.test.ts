import {expect,it} from 'vitest';
import {pixelActorAnchors,projectPixel} from '../src/play/pixelGeometry';

const projection={origin:{x:123,y:81},step:{x:48,y:32}};
const player={id:'player',at:{x:4,y:5},target:{x:4,y:5}};
const center=projectPixel(player.at,projection);

it('centers a lone actor regardless of observer identity or occupants of other cells',()=>{
 const other={id:'creature',at:{x:5,y:5},target:{x:5,y:5}};
 for(const observer of ['player','another-player']){
  expect(pixelActorAnchors([player],observer,projection).get(player.id)).toEqual(center);
  expect(pixelActorAnchors([player,other],observer,projection).get(player.id)).toEqual(center);
 }
});

it('waits for arrival before sharing and recenters as the other actor leaves or disappears',()=>{
 const arriving={id:'creature',at:{x:4.1,y:5},target:{x:4,y:5}};
 expect(pixelActorAnchors([player,arriving],player.id,projection).get(player.id)).toEqual(center);
 const arrived={...arriving,at:{...player.at}};
 const shared=pixelActorAnchors([player,arrived],player.id,projection);
 expect(shared.get(player.id)).toEqual({x:center.x-11,y:center.y});
 expect(shared.get(arrived.id)).toEqual({x:center.x+11,y:center.y});
 const leaving={...arrived,target:{x:5,y:5}};
 expect(pixelActorAnchors([player,leaving],player.id,projection).get(player.id)).toEqual(center);
 expect(pixelActorAnchors([player],player.id,projection).get(player.id)).toEqual(center);
});

it('keeps a moving actor on its interpolated route while its destination is occupied',()=>{
 const walking={...player,at:{x:3.5,y:5}};
 const other={...player,id:'creature'};
 expect(pixelActorAnchors([walking,other],player.id,projection).get(player.id)).toEqual(projectPixel(walking.at,projection));
});
