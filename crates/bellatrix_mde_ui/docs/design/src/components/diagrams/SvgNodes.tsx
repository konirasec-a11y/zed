import * as React from 'react';
export function SelectionOutline({x,y,w,h}:{x:number;y:number;w:number;h:number}) {
 const points=[[x,y],[x+w/2,y],[x+w,y],[x,y+h/2],[x+w,y+h/2],[x,y+h],[x+w/2,y+h],[x+w,y+h]];
 return <g className="selection-outline"><rect x={x} y={y} width={w} height={h} fill="none" strokeWidth="1.7" stroke="var(--selection)"/>{points.map(([a,b],i)=><rect key={i} x={a-4} y={b-4} width="8" height="8" fill="var(--paper-white)" stroke="var(--selection)" strokeWidth="1.7" rx="1"/>)}</g>;
}
export function ClassNode({x,y,w=220,title,stereotype,attributes=[],operations=[],selected=false,muted=false}:{x:number;y:number;w?:number;title:string;stereotype?:string;attributes?:string[];operations?:string[];selected?:boolean;muted?:boolean}) {
 const head=stereotype?53:37, row=23;
 const aH=Math.max(32,attributes.length*row+11),oH=Math.max(30,operations.length*row+10);
 const height=head+aH+oH;
 return <g transform={`translate(${x} ${y})`} className={muted?'diagram-node muted-node':'diagram-node'}>
  <rect width={w} height={height} rx="3" className="node-fill"/>
  <rect width={w} height={head} rx="3" className="class-header-fill"/>
  <path d={`M0 ${head}h${w}M0 ${head+aH}h${w}`} className="node-divider"/>
  {stereotype&&<text x={w/2} y="20" className="node-stereotype" textAnchor="middle">«{stereotype}»</text>}
  <text x={w/2} y={stereotype?39:24} className="node-title" textAnchor="middle">{title}</text>
  {attributes.map((a,i)=><text key={a} x="14" y={head+22+row*i} className="node-detail">{a}</text>)}
  {operations.map((o,i)=><text key={o} x="14" y={head+aH+22+row*i} className="node-detail">{o}</text>)}
  {selected&&<SelectionOutline x={-6} y={-6} w={w+12} h={height+12}/>}
 </g>;
}
export function Relation({d,label,labelX,labelY,dashed=false,arrow=true}: {d:string;label?:string;labelX?:number;labelY?:number;dashed?:boolean;arrow?:boolean}){
 return <g className="diagram-relation"><path d={d} className="relation-path" strokeDasharray={dashed?'6 5':undefined}/>{arrow&&null}{label&&<g><rect x={(labelX||0)-8} y={(labelY||0)-18} width={label.length*6.3+16} height="22" className="relation-label-bg" rx="3"/><text x={labelX} y={labelY} className="relation-label">{label}</text></g>}</g>;
}
export function CanvasBadge({x,y,text}:{x:number;y:number;text:string}){
 return <g transform={`translate(${x} ${y})`}><rect width={text.length*8+20} height="26" rx="5" className="canvas-badge-bg"/><text x="10" y="17" className="canvas-badge-label">{text}</text></g>;
}
