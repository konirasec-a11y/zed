import * as React from 'react';
import { CanvasBadge, SelectionOutline } from './SvgNodes';
function Topic({x,y,w,h,title,subtitle,variant='neutral',selected=false}:{x:number;y:number;w:number;h:number;title:string;subtitle?:string;variant?:string;selected?:boolean}){return <g transform={`translate(${x} ${y})`} className={`mind-topic mind-${variant}`}><rect width={w} height={h} rx="9"/><rect x="0" width="4" height={h} rx="2" className="mind-accent"/><text className="mind-topic-title" x="20" y={subtitle?32:h/2+5}>{title}</text>{subtitle&&<text className="mind-topic-sub" x="20" y="54">{subtitle}</text>}{selected&&<SelectionOutline x={-6} y={-6} w={w+12} h={h+12}/>}</g>}
export function MindMapDiagram(){return <svg className="specimen-diagram mind-specimen" viewBox="0 0 1060 650" role="img" aria-label="Mind map example, product strategy with topic branches">
 <CanvasBadge x={42} y={31} text="MIND MAP"/>
 <g className="mind-edges"><path d="M390 322C334 322 349 179 276 179" className="mind-blue"/><path d="M390 322C312 322 349 455 276 455" className="mind-amber"/><path d="M632 322C706 322 677 145 739 145" className="mind-teal"/><path d="M632 322C703 322 682 324 739 324" className="mind-violet"/><path d="M632 322C707 322 678 466 739 466" className="mind-rose"/>
 <path d="M240 179V106H185" className="mind-blue child-edge"/><path d="M240 179v70h-55" className="mind-blue child-edge"/>
 <path d="M919 145h64v-56" className="mind-teal child-edge"/><path d="M919 324h64v58" className="mind-violet child-edge"/></g>
 <Topic x={390} y={274} w={242} h={96} title="Product strategy" subtitle="Q4 planning · Workshop" variant="root" selected/>
 <Topic x={62} y={146} w={214} h={66} title="User research" subtitle="Interviews · Feedback" variant="blue"/>
 <Topic x={62} y={422} w={214} h={66} title="Delivery" subtitle="Milestones · Releases" variant="amber"/>
 <Topic x={739} y={112} w={208} h={66} title="Opportunities" subtitle="Problems to solve" variant="teal"/>
 <Topic x={739} y={291} w={208} h={66} title="Experience" subtitle="Interaction patterns" variant="violet"/>
 <Topic x={739} y={433} w={208} h={66} title="Success metrics" subtitle="Retention · Adoption" variant="rose"/>
 <g className="mind-caption"><text x="76" y="104">Customer interviews</text><text x="77" y="256">Usability sessions</text><text x="906" y="81">Market signals</text><text x="894" y="396">Accessibility</text></g>
 </svg>}
