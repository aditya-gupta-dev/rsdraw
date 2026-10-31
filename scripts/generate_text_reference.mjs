// Produce a browser oracle using the read-only clone's WOFF2s and font metrics.
// Open the output HTML in Chrome, then save window.reference as text.json.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const dir='excalidraw/packages/excalidraw/fonts/Excalifont';
const index=fs.readFileSync(path.join(dir,'index.ts'),'utf8');
const imports=Object.fromEntries([...index.matchAll(/import (_\d) from "\.\/(.*?)"/g)].map(m=>[m[1],m[2]]));
const css=[...index.matchAll(/uri: (_\d),\s*descriptors: \{\s*unicodeRange:\s*"([^"]+)"/g)].map(m=>{
 const data=fs.readFileSync(path.join(dir,imports[m[1]])).toString('base64');
 return `@font-face{font-family:Excalifont;src:url(data:font/woff2;base64,${data});unicode-range:${m[2]};}`;
}).join('\n');
const fontMetadata=fs.readFileSync('excalidraw/packages/common/src/font-metadata.ts','utf8');
const metrics=fontMetadata.match(/\[FONT_FAMILY.Excalifont\]:\s*\{\s*metrics:\s*\{([^}]+)\}/)[1];
const values=Object.fromEntries([...metrics.matchAll(/(\w+):\s*(-?[\d.]+)/g)].map(m=>[m[1],Number(m[2])]));
const referenceCommit=execFileSync('git',['-C','excalidraw','rev-parse','HEAD'],{encoding:'utf8'}).trim();
const samples=['A little less perfect. A lot more human.','Rectangle','AVATAR Toffee office fi fl','Latin · Ελληνικά · Кириллица','Árvíztűrő tükörfúrógép','Line one\n\nLine three\n','tabs\tand\tspaces','CRLF\r\nCR\rLF\n','  leading and trailing  ',''];
const output=process.argv[2]||'/tmp/rsdraw-text-reference.html';
fs.writeFileSync(output,`<!doctype html><meta charset="utf-8"><title>rsdraw text reference</title><style>${css}body{margin:0;background:white}canvas{display:block}</style><canvas width="1200" height="1200"></canvas><script>
const samples=${JSON.stringify(samples)},metrics=${JSON.stringify(values)};
(async()=>{
 await Promise.all(samples.map(text=>document.fonts.load('20px Excalifont',text)));
 const ctx=document.querySelector('canvas').getContext('2d');
 const fixtures=[]; let y=0;
 for(const fontSize of [12,20,36,63.5]) {
  for(const text of samples) {
   const normalized=text.replace(/\\r\\n?/g,'\\n').replace(/\\t/g,'        '),lines=normalized.split('\\n');
   ctx.font=fontSize+'px Excalifont';
   const width=Math.max(...lines.map(line=>ctx.measureText(line||' ').width));
   const height=lines.length*fontSize*metrics.lineHeight;
   const em=fontSize/metrics.unitsPerEm;
   const baseline=em*metrics.ascender+(fontSize*metrics.lineHeight-em*metrics.ascender+em*metrics.descender)/2;
   fixtures.push({text,fontSize,width,height,baseline});
   if(fontSize===20) {for(let i=0;i<lines.length;i++)ctx.fillText(lines[i],20,y+i*fontSize*metrics.lineHeight+baseline);y+=height+12;}
  }
 }
 window.reference={referenceCommit:${JSON.stringify(referenceCommit)},browser:navigator.userAgent,fixtures};
 document.title='rsdraw text reference ready';
})();
</script>`);
console.log(output);
