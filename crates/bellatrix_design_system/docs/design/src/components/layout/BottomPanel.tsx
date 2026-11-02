import * as React from 'react';
import { Icon } from '../primitives/Icon';
import { IconButton } from '../primitives/Controls';
import type { DiagramFamily } from '../../app/types';
export type BottomTab = 'relationships' | 'documentation' | 'thumbnails';
const relations: Record<DiagramFamily, string[][]> = {
  uml: [['OrderService', 'creates', 'Order'], ['Customer', 'places', 'Order'], ['Order', 'contains', 'OrderItem'], ['OrderItem', 'references', 'Product']],
  c4: [['Web application', 'calls', 'Backend API'], ['Backend API', 'reads', 'Database'], ['Backend API', 'requests', 'Payment gateway']],
  mindmap: [['Product strategy', 'contains', 'Research'], ['Product strategy', 'contains', 'Roadmap']],
  flowchart: [['Receive request', 'next', 'Validate order'], ['Validate order', 'next', 'Persist order']]
};
const descriptions: Record<DiagramFamily, string> = {
  uml:'Entities, interfaces and relationships in the ordering domain.',
  c4:'Containers, external systems and their dependencies.',
  mindmap:'Product discovery and planning structure.',
  flowchart:'Order processing steps and decisions.'
};
export function BottomPanel({family, onToggle, tab, onTab, height}: {
  family: DiagramFamily; onToggle: ()=>void; tab: BottomTab;
  onTab: (tab:BottomTab)=>void; height:number;
}) {
  const tabs: {id:BottomTab;title:string}[]=[
    {id:'relationships',title:'Relationships'}, {id:'documentation',title:'Markdown documentation'},
    {id:'thumbnails',title:'Diagram thumbnails'}
  ];
  return <section className="bottom-panel" style={{height,flex:`0 0 ${height}px`}}>
    <div className="bottom-tabs" role="tablist" aria-label="Painéis auxiliares">
      {tabs.map(t=><button role="tab" aria-selected={tab===t.id} key={t.id} className={tab===t.id?'active':''}
        onClick={()=>onTab(t.id)}>{t.title}</button>)}
      <div className="bottom-flex"/>
      <IconButton icon="chevron-down" label="Collapse bottom panel" small onClick={onToggle}/>
    </div>
    <div className="bottom-panel-body">
      {tab==='relationships'?<div className="relationship-table">
        <div className="relationship-th"><span>Source</span><span>Relation</span><span>Target</span></div>
        {relations[family].map(([from,action,to])=><div className="relationship-tr" key={from+to}><span><Icon name="box" size={12}/>{from}</span><span>{action}</span><span>{to}</span></div>)}
      </div>:tab==='documentation'?<div className="markdown-panel-preview">
        <div className="markdown-preview-actions"><span>README.md</span><span>Preview</span></div>
        <h3>{family==='uml'?'Domain model':family==='c4'?'System architecture':family==='mindmap'?'Product discovery':'Order processing'}</h3>
        <p>{descriptions[family]}</p>
      </div>:<div className="thumbnail-row">
        <button className="diagram-thumbnail active" aria-label="Miniatura do diagrama atual"><span className="thumbnail-graphic"><i/><i/><i/></span><span>{family==='uml'?'Domain model':family==='c4'?'System architecture':family==='mindmap'?'Product discovery':'Order processing'}</span></button>
        <span className="thumbnail-hint">Diagram structure preview</span>
      </div>}
    </div>
  </section>;
}
