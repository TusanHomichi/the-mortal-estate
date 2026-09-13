import {describe,it,expect} from 'vitest';
import * as T from 'three';
import {FigurePlayback} from '../src/play/dungeon/playback';
import {inPlaceClips,type FigureAsset} from '../src/play/dungeon/assets';

function fixture(){
  const scene=new T.Group(),bone=new T.Bone();bone.name='root';scene.add(bone);
  const clips=['guard','walk','flying_kick','jab_left'].map((name,i)=>new T.AnimationClip(name,2,[
    new T.NumberKeyframeTrack('root.position[x]',[0,2],[i*10,i*10+2]),
  ]));
  const asset:FigureAsset={scene,clips,idle:'guard',stride:2};
  return {scene,bone,asset,playback:new FigurePlayback(scene,asset,0)};
}
describe('elapsed figure playback',()=>{
  it('samples actual pose and fades from elapsed time under irregular frame delivery',()=>{
    const {playback,bone}=fixture();playback.play('flying_kick',0,3);
    for(const now of [150,780,1500,2400]){
      playback.update(now);
      expect(playback.diagnostics().phase).toBeCloseTo(now/3000);
      expect(bone.position.x).toBeCloseTo(20+now/1500);
      expect(playback.diagnostics().weights).toEqual([{clip:'flying_kick',weight:1}]);
    }
    playback.dispose();
  });
  it('expires and recovers through hidden time without keeping the old pose in a punch',()=>{
    const {playback,bone}=fixture();playback.play('flying_kick',0,3);playback.update(300);
    playback.update(4300);
    expect(playback.diagnostics()).toMatchObject({clip:'guard',elapsedMs:1300,weights:[{clip:'guard',weight:1}]});
    expect(bone.position.x).toBeCloseTo(1.3);
    playback.play('jab_left',4300,1.5);playback.update(4600);
    expect(bone.position.x).toBeCloseTo(30.4);
    expect(playback.diagnostics().weights).toEqual([{clip:'jab_left',weight:1}]);playback.dispose();
  });
  it('keeps a fresh cue at phase zero after a long interval without a rendered frame',()=>{
    const {playback}=fixture();playback.play('jab_left',5000,1.5);playback.update(5150);
    expect(playback.diagnostics().phase).toBeCloseTo(.1);playback.dispose();
  });
  it('samples walking by rendered distance while transition fades still use elapsed time',()=>{
    const {playback,bone}=fixture();playback.play('walk',0);playback.update(700,.5);
    expect(bone.position.x).toBeCloseTo(10.5);playback.update(1700,.5);expect(bone.position.x).toBeCloseTo(10.5);
    playback.play('guard',1700);playback.update(1900);
    expect(bone.position.x).toBeCloseTo(.2);expect(playback.diagnostics().weights).toEqual([{clip:'guard',weight:1}]);playback.dispose();
  });
});

describe('martial source displacement',()=>{
  it('removes planar root travel without changing vertical lift, articulated tracks or source clips',()=>{
    const scene=new T.Group(),root=new T.Bone();root.name='motion';root.position.set(.5,0,-.5);scene.add(root);
    const translation=new T.VectorKeyframeTrack('motion.position',[0,1],[1,2,3,4,5,6],T.InterpolateDiscrete);
    const rotation=new T.QuaternionKeyframeTrack('motion.quaternion',[0,1],[0,0,0,1,0,1,0,0]);
    const source=new T.AnimationClip('jump',1,[translation,rotation]),[clip]=inPlaceClips(scene,[source],'motion');
    expect([...clip!.tracks[0]!.values]).toEqual([.5,2,-.5,.5,5,-.5]);
    expect(clip!.tracks[0]!.getInterpolation()).toBe(T.InterpolateDiscrete);
    expect([...clip!.tracks[1]!.values]).toEqual([...rotation.values]);
    expect([...translation.values]).toEqual([1,2,3,4,5,6]);
  });
  it('refuses missing roots and missing, duplicate or partial root translations',()=>{
    const scene=new T.Group(),root=new T.Bone();root.name='motion';scene.add(root);
    const track=new T.VectorKeyframeTrack('motion.position',[0,1],[0,0,0,1,1,1]);
    expect(()=>inPlaceClips(scene,[],'absent')).toThrow('root');
    for(const tracks of [[],[track,track],[new T.NumberKeyframeTrack('motion.position[x]',[0,1],[0,1])]])
      expect(()=>inPlaceClips(scene,[new T.AnimationClip('jump',1,tracks)],'motion')).toThrow('translation');
  });
});
