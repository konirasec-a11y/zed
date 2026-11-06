import * as React from 'react';
import type { DiagramFamily, InspectorTab } from '../../app/types';
import { ResizeHandle } from '../dock/ResizeHandle';
import { ModelExplorerPane } from '../panels/ModelExplorerPane';
import { PropertyInspectorPane } from '../panels/PropertyInspectorPane';

interface Props {
  family:DiagramFamily; tab:InspectorTab; onTab:(tab:InspectorTab)=>void;
  explorerOpen:boolean; inspectorOpen:boolean; explorerHeight:number;
  onExplorerToggle:()=>void; onInspectorToggle:()=>void;
  onExplorerResize:(delta:number)=>void; onExplorerReset:()=>void;
}
/** Stack positioning can be replaced by GPUI Dock without rewriting pane content. */
export function RightInspector(p:Props) {
  return <aside className="right-inspector" aria-label="Model and properties">
    <ModelExplorerPane family={p.family} open={p.explorerOpen} onToggle={p.onExplorerToggle} height={p.explorerHeight}/>
    {p.explorerOpen&&p.inspectorOpen&&<ResizeHandle axis="vertical" label="Model explorer height"
      onDelta={p.onExplorerResize} onReset={p.onExplorerReset} value={p.explorerHeight} min={80}/>}
    <PropertyInspectorPane family={p.family} tab={p.tab} onTab={p.onTab}
      open={p.inspectorOpen} onToggle={p.onInspectorToggle}/>
  </aside>;
}
