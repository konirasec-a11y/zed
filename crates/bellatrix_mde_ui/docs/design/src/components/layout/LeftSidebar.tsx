import * as React from 'react';
import type { DiagramFamily } from '../../app/types';
import { ResizeHandle } from '../dock/ResizeHandle';
import { WorkingDiagramsPane } from '../panels/WorkingDiagramsPane';
import { ToolboxPane } from '../panels/ToolboxPane';

interface Props {
  family:DiagramFamily; onFamily:(family:DiagramFamily)=>void;
  search:string; onSearch:(search:string)=>void; chosen:string; onChoose:(id:string)=>void;
  collapsed:Record<string,boolean>; onToggleGroup:(id:string)=>void;
  projectsOpen:boolean; toolboxOpen:boolean;
  onProjectsToggle:()=>void; onToolboxToggle:()=>void;
  projectsHeight:number; onProjectsResize:(delta:number)=>void; onProjectsReset:()=>void;
}
/** A stack is only composition; its panes can be placed in any future native Dock. */
export function LeftSidebar(p:Props) {
  return <aside className="left-sidebar" aria-label="Diagrams and tools">
    <WorkingDiagramsPane family={p.family} onFamily={p.onFamily}
      open={p.projectsOpen} onToggle={p.onProjectsToggle} height={p.projectsHeight}/>
    {p.projectsOpen&&p.toolboxOpen&&<ResizeHandle axis="vertical" label="Working diagrams height"
      onDelta={p.onProjectsResize} onReset={p.onProjectsReset} value={p.projectsHeight} min={90}/>}
    <ToolboxPane family={p.family} search={p.search} onSearch={p.onSearch} chosen={p.chosen} onChoose={p.onChoose}
      collapsed={p.collapsed} onToggleGroup={p.onToggleGroup} open={p.toolboxOpen} onToggle={p.onToolboxToggle}/>
  </aside>;
}
