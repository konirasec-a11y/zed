import * as React from 'react';
import { Icon } from '../primitives/Icon';
import { DockPanelHeader } from '../dock/DockPanel';
import { diagrams, elements } from '../../app/diagramData';
import type { DiagramFamily } from '../../app/types';

export interface ModelExplorerPaneProps {
  family:DiagramFamily; open:boolean; onToggle:()=>void; height:number;
}
/** Read-only tree specimen; selected identity is defined by scene data. */
export function ModelExplorerPane({family,open,onToggle,height}:ModelExplorerPaneProps) {
  const node=elements[family],diagram=diagrams.find(d=>d.family===family)!;
  const extras=family==='uml'?['Order','OrderRepository','OrderItem','Product','Customer']
    :family==='c4'?['Web application','Database','Payment gateway']
    :family==='mindmap'?['Research','Audience','Roadmap','Delivery']
    :['Receive request','Persist order','Send event'];
  const style:React.CSSProperties={flex:`0 0 ${open?height:36}px`};
  return <section className="explorer-panel" style={style}>
    <DockPanelHeader title="Model explorer" open={open} onToggle={onToggle} id="explorer-content"/>
    {open&&<div id="explorer-content" className="explorer-inner">
      <div className="explorer-root"><Icon name="chevron-down" size={12}/><Icon name="folder" size={14}/><strong>Commerce platform</strong></div>
      <div className="tree-line level-1"><Icon name="chevron-down" size={12}/><Icon name="folder" size={14}/><span>Architecture</span></div>
      <div className="tree-line level-2"><Icon name="chevron-down" size={12}/>
        <Icon name={family==='uml'?'box':family==='c4'?'layers':family==='mindmap'?'tree':'corner'} size={14}/>
        <span>{diagram.label}</span></div>
      <div className="tree-line level-3 is-selected"><span className="tree-indent-spacer"/><Icon name="box" size={13}/><span>{node.name}</span></div>
      {extras.map(s=><div className="tree-line level-3" key={s}><span className="tree-indent-spacer"/><Icon name="box" size={12}/><span>{s}</span></div>)}
    </div>}
  </section>;
}
