// Regenerate test data from the actual, read-only Excalidraw checkout.
// Requires Node and pinned esbuild (downloaded by npm exec); no project install.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url),root=process.cwd();
const read=p=>fs.readFileSync(path.join(root,'excalidraw',p),'utf8');
const shape=read('packages/element/src/shape.ts'),bounds=read('packages/element/src/bounds.ts'),utils=read('packages/element/src/utils.ts'),pure=read('packages/utils/src/shape.ts'),colors=read('packages/common/src/colors.ts');
function declaration(source,name,kind='const') {
  const pattern=kind==='function' ? `function ${name}(` : `const ${name} =`;
  const start=source.indexOf(pattern);
  if(start<0) throw Error(`Missing reference declaration: ${name}`);
  const end=source.indexOf(kind==='function' ? '\n}' : '\n};',start);
  if(end<0) throw Error(`Missing end: ${name}`);
  return source.slice(start,end+(kind==='function'?2:3));
}
const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'rsdraw-oracle-'));
const prefix=`
const Rough=require(${JSON.stringify(path.join(root,'assets/vendor/rough-4.6.4.cjs'))});
const {getStroke}=require(${JSON.stringify(path.join(root,'assets/vendor/perfect-freehand-1.2.0.cjs'))});
${fs.readFileSync('assets/vendor/excalidraw-laser-pointer.js','utf8')}
const LaserPointer=ExcalidrawLaserPointer.LaserPointer;
const ROUNDNESS={LEGACY:1,PROPORTIONAL_RADIUS:2,ADAPTIVE_RADIUS:3};
const ROUGHNESS={cartoonist:2},THEME={LIGHT:'light',DARK:'dark'},COLOR_PALETTE={white:'#ffffff'};
const DEFAULT_PROPORTIONAL_RADIUS=.25,DEFAULT_ADAPTIVE_RADIUS=32,DEFAULT_STROKE_STREAMLINE=.5;
const CARDINALITY_MARKER_SIZE=20,CROWFOOT_ARROWHEAD_SIZE=15;
const isTransparent=c=>c==='transparent';
const applyDarkModeFilter=c=>c;
const canChangeRoundness=e=>['rectangle','diamond','line','arrow'].includes(e.type);
const isLinearElement=e=>e.type==='line'||e.type==='arrow';
const clamp=(v,min,max)=>Math.max(min,Math.min(max,v));
const isIframeLikeElement=()=>false,isElbowArrow=()=>false,isPathALoop=()=>false,isEmbeddableElement=()=>false,isIframeElement=()=>false;
const invariant=(value,message)=>{if(!value)throw Error(message)};
const pointFrom=(x,y)=>[x,y],pointFromArray=p=>p;
const degreesToRadians=d=>d*Math.PI/180;
const pointRotateRads=(p,c,a)=>[(p[0]-c[0])*Math.cos(a)-(p[1]-c[1])*Math.sin(a)+c[0],(p[0]-c[0])*Math.sin(a)+(p[1]-c[1])*Math.cos(a)+c[1]];
const assertNever=value=>{throw Error('Unexpected reference branch '+JSON.stringify(value));};
`;
const functions=[
  declaration(shape,'adjustRoughness','function'),
  ...['getDashArrayDashed','getDashArrayDotted'].map(name=>shape.slice(shape.indexOf(`const ${name}`),shape.indexOf(';',shape.indexOf(`const ${name}`))+1)),
  ...['generateRoughOptions','modifyIframeLikeForRoughOptions','generateArrowheadLinesToTip','getArrowheadLineOptions','getArrowheadShapes','_generateElementShape','getFreeDrawSvgPath','getFreedrawStreamline','getVariableWidthFreedrawOutline','createLaserPointer','getConstantWidthFreedrawOutline','getFreedrawOutlinePoints','med','getSvgPathFromStroke'].map(name=>declaration(shape,name)),
  ...['getArrowheadSize','getArrowheadAngle','getArrowheadPoints','getDiamondPoints'].map(name=>declaration(bounds,name)),
  declaration(utils,'getCornerRadius'),declaration(pure,'getCurvePathOps'),
  declaration(colors,'cssHueRotate','function'),declaration(colors,'cssInvert'),
];
// Two helpers are expression-bodied, not block-bodied; terminate at their semicolon.
for(const name of ['getFreedrawStreamline','createLaserPointer']) {
  const index=functions.findIndex(s=>s.startsWith(`const ${name}`));
  const start=shape.indexOf(`const ${name} =`);functions[index]=shape.slice(start,shape.indexOf(';',start)+1);
}
const constants=['VARIABLE_WIDTH_FREEDRAW','CONSTANT_WIDTH_FREEDRAW'].map(name=>{
  const start=shape.indexOf(`const ${name}`);return shape.slice(start,shape.indexOf('} as const;',start)+11);
}).join('\n')+"\nconst TO_FIXED_PRECISION = /(\\s?[A-Z]?,?-?[0-9]*\\.[0-9]{0,2})(([0-9]|e|-)*)/g;";
const source=prefix+'\n'+constants+'\n'+functions.join('\n')+`
module.exports={render(e){
 const element={type:e.tool,width:e.width,height:e.height,seed:e.seed,strokeWidth:e.tool==='freehand'?e.strokeWidth/2:e.strokeWidth,roughness:e.roughness,strokeStyle:e.strokeStyle,strokeColor:'#000000',backgroundColor:e.filled?'#000000':'transparent',fillStyle:e.fillStyle,roundness:e.rounded?{type:e.tool==='rectangle'?3:2}:null,points:e.tool==='freehand'?e.points:[[0,0],e.end],simulatePressure:true,strokeOptions:{variability:e.variability}};
 if(e.tool==='freehand') {
  const p=getFreeDrawSvgPath(element);
  return {drawables:p?[Rough.generator().path(p,{stroke:'none',fill:'#000000',fillStyle:'solid',roughness:0,seed:e.seed})]:[],outline:getFreedrawOutlinePoints(element)};
 }
 const result=_generateElementShape(element,Rough.generator(),{isExporting:false,canvasBackgroundColor:'#ffffff',embedsValidationStatus:null,theme:'light'});
 return {drawables:Array.isArray(result)?result:[result]};
},dark(rgb){const inv=cssInvert(...rgb,93);return cssHueRotate(inv.r,inv.g,inv.b,180);}};
`;
try {
 const input=path.join(temporary,'oracle.ts'),output=path.join(temporary,'oracle.cjs');fs.writeFileSync(input,source);
 execFileSync('npm',['exec','--yes','--package=esbuild@0.25.12','--','esbuild',input,'--format=cjs','--platform=node',`--outfile=${output}`],{stdio:'inherit',env:{...process.env,GOMAXPROCS:'5'}});
 const oracle=require(output),fixtures=[];
 const base={width:210,height:140,end:[210,140],points:[],seed:1234567,strokeWidth:2,roughness:1,strokeStyle:'solid',fillStyle:'solid',filled:false,rounded:true,variability:'constant'};
 for(const tool of ['rectangle','diamond','ellipse','line','arrow']) {
  for(const variant of [{},{rounded:false},{roughness:2},{width:12,height:8,end:[12,8]},{strokeStyle:'dashed'},{strokeStyle:'dotted'},{filled:true,fillStyle:'hachure'},{filled:true,fillStyle:'cross-hatch'},{filled:true},{end:[-210,-140]},{seed:1},{seed:2147483647},{strokeWidth:4},{roughness:0}]) {
   const request={...base,...variant,tool};if(tool==='ellipse')request.rounded=false;
   fixtures.push({request,expected:oracle.render(request)});
  }
 }
 // Exercise combinations, not just one changed property at a time.
 let random=0x1234567;
 const next=()=>{random=(Math.imul(random,1664525)+1013904223)>>>0;return random/4294967296;};
 for (const tool of ['rectangle','diamond','ellipse','line','arrow']) {
  for(let i=0;i<36;i++) {
   const width=i===0?0:next()*700,height=i===1?0:next()*480;
   const request={...base,tool,width,height,end:[width*(i%2?-1:1),height*(i%3?-1:1)],
    seed:Math.floor(next()*2147483646)+1,roughness:[0,.5,1,2,3][i%5],strokeWidth:[.5,1,2,4,12][i%5],
    strokeStyle:['solid','dashed','dotted'][i%3],rounded:tool!=='ellipse'&&i%2===0,
    filled:true,fillStyle:['solid','hachure','cross-hatch'][Math.floor(i/3)%3]};
   fixtures.push({request,expected:oracle.render(request)});
  }
 }
 const traces=[[],[[0,0]],[[0,0],[12,7]],Array.from({length:40},(_,i)=>[i*7,Math.sin(i*.23)*35]),
  [[0,0],[0,0],[20,0],[20,0],[30,0],[15,0],[-30,0]],
  [[0,0],[20,0],[20,30],[40,30],[40,-20],[-10,-20]],
  Array.from({length:128},(_,i)=>[i*100,i%2?200:0]),
  Array.from({length:256},(_,i)=>[Math.cos(i/100)*80,Math.sin(i/100)*80])];
 for(const variability of ['constant','variable']) {
  for(const trace of traces) {
   const origin=trace[0]||[0,0];
   const points=trace.map(p=>[p[0]-origin[0],p[1]-origin[1]]);
   for(const strokeWidth of [.5,2,4,12]) {
    const request={...base,tool:'freehand',variability,points,strokeWidth};fixtures.push({request,expected:oracle.render(request)});
   }
  }
 }
 const palette=[[30,30,30],[255,255,255],[224,49,49],[47,158,68],[25,113,194],[240,140,0],[232,89,12],[12,133,153]];
 const commit=execFileSync('git',['-C','excalidraw','rev-parse','HEAD'],{encoding:'utf8'}).trim();
 fs.writeFileSync('tests/fixtures/excalidraw.json',JSON.stringify({referenceCommit:commit,roughjs:'4.6.4',perfectFreehand:'1.2.0',fixtures,colors:palette.map(rgb=>({rgb,expected:oracle.dark(rgb)}))})+'\n');
 console.log(`Generated ${fixtures.length} reference cases and ${palette.length} color cases from ${commit}`);
} finally {fs.rmSync(temporary,{recursive:true,force:true});}
