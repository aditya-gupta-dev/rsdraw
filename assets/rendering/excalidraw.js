/* Native rendering bridge for the pinned Excalidraw reference.
 * Geometry/options adapted from packages/element/src/{shape,bounds,utils}.ts.
 * Copyright (c) 2020 Excalidraw. MIT; see assets/vendor/excalidraw-LICENSE.txt.
 * RoughJS and freehand implementations are bundled unmodified separately.
 */
(() => {
  const generator = Rough.generator();
  function options(e) {
    const max = Math.max(e.width,e.height), min = Math.min(e.width,e.height);
    const linear = e.tool === "line" || e.tool === "arrow";
    const roughness = ((min >= 20 && max >= 50) || (min >= 15 && e.rounded) || (linear && max >= 50))
      ? e.roughness : Math.min(e.roughness / (max < 10 ? 3 : 2),2.5);
    const o = {
      seed:e.seed,
      strokeLineDash:e.strokeStyle === "dashed" ? [8,8+e.strokeWidth] : e.strokeStyle === "dotted" ? [1.5,6+e.strokeWidth] : undefined,
      disableMultiStroke:e.strokeStyle !== "solid",
      strokeWidth:e.strokeStyle === "solid" ? e.strokeWidth : e.strokeWidth+0.5,
      fillWeight:e.strokeWidth/2, hachureGap:e.strokeWidth*4,
      roughness, stroke:"#000000", preserveVertices:(!linear && e.rounded) || e.roughness < 2,
    };
    if (!linear) {
      o.fillStyle=e.fillStyle;
      o.fill=e.filled ? "#000000" : undefined;
      if(e.tool === "ellipse") o.curveFitting=1;
    }
    return o;
  }
  function arrowheads(body,e,o) {
    // getArrowheadPoints(), default endArrowhead='arrow', offsetMultiplier=0.
    const ops=body.sets.find(s=>s.type==="path").ops;
    if (ops.length<2) return [];
    const index=ops.length-1, data=ops[index].data, prev=ops[index-1];
    if(data.length!==6) return [];
    const p3=[data[4],data[5]],p2=[data[2],data[3]],p1=[data[0],data[1]];
    const p0=prev.op==="move" ? prev.data : [prev.data[4],prev.data[5]];
    const equation=(t,i)=> Math.pow(1-t,3)*p3[i]+3*t*Math.pow(1-t,2)*p2[i]+3*Math.pow(t,2)*(1-t)*p1[i]+p0[i]*Math.pow(t,3);
    const tip=p3, near=[equation(.3,0),equation(.3,1)];
    const distance=Math.hypot(tip[0]-near[0],tip[1]-near[1]);
    if(distance===0) return [];
    const size=Math.min(25,Math.hypot(e.end[0],e.end[1])*.5);
    const base=[tip[0]-(tip[0]-near[0])/distance*size,tip[1]-(tip[1]-near[1])/distance*size];
    const lineOptions={...o,roughness:Math.min(1,o.roughness||0)};
    if(e.strokeStyle==="dotted") lineOptions.strokeLineDash=[1.5,4+e.strokeWidth];
    else delete lineOptions.strokeLineDash;
    return [-20,20].map(degrees=>{
      const angle=degrees*Math.PI/180,dx=base[0]-tip[0],dy=base[1]-tip[1];
      const x=tip[0]+dx*Math.cos(angle)-dy*Math.sin(angle),y=tip[1]+dx*Math.sin(angle)+dy*Math.cos(angle);
      return generator.line(x,y,tip[0],tip[1],lineOptions);
    });
  }
  function strokePath(points) {
    if(!points.length) return "";
    const med=(a,b)=>[(a[0]+b[0])/2,(a[1]+b[1])/2];
    return points.reduce((acc,p,i,arr)=>{
      if(i===arr.length-1) acc.push(p,med(p,arr[0]),"L",arr[0],"Z");
      else acc.push(p,med(p,arr[i+1]));
      return acc;
    },["M",points[0],"Q"]).join(" ").replace(/(\s?[A-Z]?,?-?[0-9]*\.[0-9]{0,2})(([0-9]|e|-)*)/g,"$1");
  }
  function generate(e) {
    if(e.tool==="freehand") {
      const width=e.strokeWidth/2; // schema 2.0: medium pen=1, medium shape=2
      let points;
      if(e.variability==="constant") {
        const laser=new ExcalidrawLaserPointer.LaserPointer({size:width*1.4,streamline:.5,simplify:0,sizeMapping:d=>Math.max(.1,d.pressure)});
        e.points.forEach(p=>laser.addPoint([p[0],p[1],1]));
        points=laser.getStrokeOutline().map(p=>[p[0],p[1]]);
      } else {
        points=PerfectFreehand.getStroke(e.points,{simulatePressure:true,size:width*4.25,thinning:.6,smoothing:.5,streamline:.5,easing:t=>Math.sin(t*Math.PI/2),last:true});
      }
      const path=strokePath(points);
      if(!path) return {drawables:[],outline:points};
      return {drawables:[generator.path(path,{stroke:"none",fill:"#000000",fillStyle:"solid",roughness:0,seed:e.seed})],outline:points};
    }
    const w=e.width,h=e.height,o=options(e);
    let body;
    switch(e.tool) {
      case "rectangle":
        if(e.rounded) {
          const r=Math.min(Math.min(w,h)*.25,32);
          body=generator.path(`M ${r} 0 L ${w-r} 0 Q ${w} 0, ${w} ${r} L ${w} ${h-r} Q ${w} ${h}, ${w-r} ${h} L ${r} ${h} Q 0 ${h}, 0 ${h-r} L 0 ${r} Q 0 0, ${r} 0`,o);
        } else body=generator.rectangle(0,0,w,h,o);
        break;
      case "diamond": {
        const topX=Math.floor(w/2)+1,topY=0,rightX=w,rightY=Math.floor(h/2)+1,bottomX=topX,bottomY=h,leftX=0,leftY=rightY;
        if(e.rounded) {
          const vr=Math.abs(topX-leftX)*.25,hr=Math.abs(rightY-topY)*.25;
          body=generator.path(`M ${topX+vr} ${topY+hr} L ${rightX-vr} ${rightY-hr} C ${rightX} ${rightY}, ${rightX} ${rightY}, ${rightX-vr} ${rightY+hr} L ${bottomX+vr} ${bottomY-hr} C ${bottomX} ${bottomY}, ${bottomX} ${bottomY}, ${bottomX-vr} ${bottomY-hr} L ${leftX+vr} ${leftY+hr} C ${leftX} ${leftY}, ${leftX} ${leftY}, ${leftX+vr} ${leftY-hr} L ${topX-vr} ${topY+hr} C ${topX} ${topY}, ${topX} ${topY}, ${topX+vr} ${topY+hr}`,o);
        } else body=generator.polygon([[topX,topY],[rightX,rightY],[bottomX,bottomY],[leftX,leftY]],o);
        break;
      }
      case "ellipse": body=generator.ellipse(w/2,h/2,w,h,o);break;
      case "triangle": body=generator.polygon([[w/2,0],[w,h],[0,h]],o);break;
      case "line": case "arrow": {
        const points=[[0,0],e.end];body=e.rounded ? generator.curve(points,o) : generator.linearPath(points,o);
        break;
      }
      default:throw new Error(`Unsupported tool ${e.tool}`);
    }
    return {drawables:e.tool==="arrow" ? [body,...arrowheads(body,e,o)] : [body]};
  }
  globalThis.rsdrawGenerate=json=>JSON.stringify(generate(JSON.parse(json)));
})();
