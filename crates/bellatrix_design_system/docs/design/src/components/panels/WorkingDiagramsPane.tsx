import * as React from 'react';
import { Icon } from '../primitives/Icon';
import { diagrams } from '../../app/diagramData';
import type { DiagramFamily } from '../../app/types';
import { DockPanelHeader } from '../dock/DockPanel';

export interface WorkingDiagramsPaneProps {
  family: DiagramFamily;
  onFamily: (family: DiagramFamily) => void;
  open: boolean; onToggle: () => void; height: number;
}
/** Reusable working-diagram dock content. Contains no resize or placement logic. */
export function WorkingDiagramsPane({family,onFamily,open,onToggle,height}:WorkingDiagramsPaneProps) {
  return <section className="left-project-panel" style={{flex:`0 0 ${open?height:36}px`}}>
    <DockPanelHeader title="Working diagrams" open={open} onToggle={onToggle} id="working-diagrams"/>
    {open&&<div id="working-diagrams" className="left-project-content">
      <div className="project-group"><Icon name="chevron-down" size={12}/><Icon name="folder" size={14}/><span>Commerce platform</span></div>
      <div className="diagram-list">{diagrams.map(d=><button key={d.family} onClick={()=>onFamily(d.family)}
        className={`diagram-item ${family===d.family?'active':''}`} aria-current={family===d.family?'page':undefined}>
        <Icon name={d.family==='uml'?'box':d.family==='c4'?'layers':d.family==='mindmap'?'tree':'corner'} size={14}/>
        <span>{d.label}</span></button>)}</div>
    </div>}
  </section>;
}
