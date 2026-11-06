import * as React from 'react';
import { SelectionOutline, CanvasBadge } from './SvgNodes';
function Box({x,y,w=210,h=107,title,subtitle,role,selected=false,external=false,db=false}:{x:number;y:number;w?:number;h?:number;title:string;subtitle:string;role:string;selected?:boolean;external?:boolean;db?:boolean}){
 return <g transform={`translate(${x} ${y})`} className={`c4-node${external?' c4-external':''}`}>
  <rect className="c4-box" width={w} height={h} rx="5"/><rect className="c4-box-strip" width="4" height={h} rx="1"/>
  {db&&<><ellipse className="c4-database" cx="29" cy="29" rx="10" ry="5"/><path className="c4-database" d="M19 29v9c0 7 20 7 20 0v-9"/></>}
  <text className="c4-role" x={db?50:17} y="27">{role.toUpperCase()}</text>
  <text className="c4-title" x="17" y="55">{title}</text>
  <text className="c4-subtitle" x="17" y="77">{subtitle}</text>
  {selected&&<SelectionOutline x={-6} y={-6} w={w+12} h={h+12}/>}
 </g>
}
export function C4Diagram(){return <svg className="specimen-diagram c4-specimen" viewBox="0 0 1060 650" role="img" aria-label="C4 container architecture diagram, illustrative commerce platform">
 <CanvasBadge x={42} y={31} text="C4 · CONTAINER LEVEL"/>
 <defs><marker id="c4arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M1 1 7 4 1 7" fill="none" stroke="var(--diagram-edge)" strokeWidth="1.1"/></marker></defs>
 <rect x="286" y="91" width="580" height="484" rx="11" className="c4-boundary"/>
 <text x="307" y="119" className="boundary-title">COMMERCE PLATFORM</text><text x="307" y="139" className="boundary-subtitle">System boundary</text>
 <g className="c4-person"><circle cx="139" cy="219" r="24"/><path d="M99 281c0-49 80-49 80 0"/><text x="139" y="314" textAnchor="middle" className="c4-title">Customer</text><text x="139" y="334" textAnchor="middle" className="c4-subtitle">[Person]</text></g>
 <g className="c4-links"><path d="M181 254h151" markerEnd="url(#c4arrow)"/><path d="M544 255h67" markerEnd="url(#c4arrow)"/><path d="M715 308v76" markerEnd="url(#c4arrow)"/><path d="M638 308v42H502v34" markerEnd="url(#c4arrow)"/><path d="M818 385h70" markerEnd="url(#c4arrow)"/></g>
 <g className="c4-edge-label"><text x="220" y="241">Uses</text><text x="566" y="241">HTTPS / JSON</text><text x="724" y="352">Reads / writes</text><text x="531" y="338">Publishes</text></g>
 <Box x={332} y={197} w={212} title="Web application" subtitle="React · client interface" role="Container"/>
 <Box x={611} y={197} w={212} title="Order API" subtitle="Rust · REST service" role="Container" selected/>
 <Box x={398} y={384} w={208} title="Event broker" subtitle="Message queue" role="Container"/>
 <Box x={611} y={384} w={207} title="Orders database" subtitle="PostgreSQL" role="Data store" db/>
 <Box x={887} y={385} w={104} h={105} title="Payment" subtitle="External" role="System" external/>
 <g transform="translate(692 160)" className="diagram-mini-action"><rect width="134" height="26" rx="4"/><text x="10" y="17">+ Add relation</text></g>
 </svg>}
