import * as React from 'react';
import { CanvasBadge, SelectionOutline } from './SvgNodes';
function Process({x,y,w=200,h=62,title,kind='process',selected=false}:{x:number;y:number;w?:number;h?:number;title:string;kind?:'process'|'terminal'|'decision'|'document';selected?:boolean}){
 return <g transform={`translate(${x} ${y})`} className="flow-node">
  {kind==='decision'?<path className="flow-node-fill" d={`M${w/2} 0 ${w} ${h/2} ${w/2} ${h} 0 ${h/2}z`}/>:kind==='terminal'?<rect className="flow-node-fill flow-terminal" width={w} height={h} rx={h/2}/>:kind==='document'?<path className="flow-node-fill" d={`M0 0h${w}v${h-9}q-${w/4} -15 -${w/2} 0T0 ${h-9}z`}/>:<rect className="flow-node-fill" width={w} height={h} rx="6"/>}
  <text x={w/2} y={h/2+5} textAnchor="middle" className="flow-node-title">{title}</text>
  {selected&&<SelectionOutline x={-7} y={-7} w={w+14} h={h+14}/>} </g>
}
export function FlowDiagram(){return <svg className="specimen-diagram flow-specimen" viewBox="0 0 1060 650" role="img" aria-label="Example flowchart showing request validation and order processing">
 <CanvasBadge x={42} y={31} text="FLOWCHART"/>
 <defs><marker id="flow-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M1 1 9 5 1 9" fill="none" stroke="var(--diagram-edge)" strokeWidth="1.4"/></marker></defs>
 <g className="flow-arrows"><path d="M540 130v54"/><path d="M540 303v74"/><path d="M410 244H280v96"/><path d="M540 439v59"/><path d="M280 402v100"/><path d="M540 560v49H280v-45"/></g>
 <g className="flow-edge-label"><text x="558" y="346">Yes</text><text x="323" y="235">No</text></g>
 <Process x={442} y={72} w={196} h={58} title="Receive request" kind="terminal"/>
 <Process x={410} y={184} w={260} h={119} title="Valid request?" kind="decision" selected/>
 <Process x={170} y={340} w={220} h={62} title="Return 400" kind="process"/>
 <Process x={438} y={377} w={204} h={62} title="Persist order"/>
 <Process x={438} y={498} w={204} h={62} title="Publish event"/>
 <Process x={170} y={502} w={220} h={62} title="End" kind="terminal"/>
 <g transform="translate(693 214)" className="flow-note"><rect width="249" height="94" rx="6"/><text x="17" y="30">Validation gate</text><text x="17" y="54">All branches are static</text><text x="17" y="72">visual examples.</text><path d="M0 46H-29"/></g>
 </svg>}
