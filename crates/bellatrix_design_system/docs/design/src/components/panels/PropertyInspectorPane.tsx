import * as React from 'react';
import { Icon } from '../primitives/Icon';
import { IconButton, SectionHeading, Toggle } from '../primitives/Controls';
import { elements } from '../../app/diagramData';
import type { DiagramFamily, InspectorTab } from '../../app/types';
import { ShapeGlyph } from '../palette/ShapeGlyph';

export interface PropertyInspectorPaneProps {
  family:DiagramFamily; tab:InspectorTab;
  onTab:(tab:InspectorTab)=>void; open:boolean; onToggle:()=>void;
}
/** Property editor preview; no domain writes or canvas mutation. */
export function PropertyInspectorPane(p:PropertyInspectorPaneProps) {
  const node=elements[p.family];
  return (
    <section className={`inspector-panel ${p.open ? '' : 'dock-closed'}`}>
      <div className="inspector-tabbar">
        <IconButton icon={p.open ? 'chevron-down' : 'chevron-right'} small
          label={p.open ? 'Collapse properties' : 'Expand properties'} onClick={p.onToggle}/>
        <button aria-label="Properties" className={p.tab === 'properties' ? 'active' : ''}
          onClick={() => { p.onTab('properties'); if (!p.open) p.onToggle(); }}>Properties</button>
        {p.open && <button className={p.tab === 'styles' ? 'active' : ''}
          onClick={() => p.onTab('styles')}>Style</button>}
      </div>
      {p.open && <div className="inspector-scroll" id="inspector-content">
        <div className="element-identity">
          <span className="identity-icon"><ShapeGlyph kind={p.family === 'uml' ? 'class' : p.family === 'c4' ? 'container' : p.family === 'mindmap' ? 'topic' : 'decision'} size={28}/></span>
          <div><h3>{node.name}</h3><p>{node.kind} · Selected</p></div>
        </div>
        {p.tab === 'properties' ? <>
          <SectionHeading title="GENERAL"/>
          <div className="property-list"><Prop label="Name"><span className="faux-input">{node.name}</span></Prop>
            <Prop label="Type"><span className="faux-select">{node.kind}<Icon name="chevron-down" size={12}/></span></Prop>
            <Prop label="Namespace"><span className="faux-input muted-field">{node.namespace}</span></Prop>
            <Prop label="Visibility"><span className="faux-select">public<Icon name="chevron-down" size={12}/></span></Prop></div>
          <SectionHeading title="ATTRIBUTES"/>
          <div className="attribute-row"><span>+</span>id: UUID</div><div className="attribute-row"><span>+</span>status: OrderStatus</div>
          <details className="inspector-details"><summary>Documentation</summary><p>{node.notes}</p></details>
          <details className="inspector-details"><summary>Advanced</summary>
            <div className="metadata-row"><span>Abstract</span><Toggle label="Abstract"/></div>
            <div className="metadata-row"><span>Active</span><Toggle checked label="Active"/></div>
          </details>
        </> : <>
          <SectionHeading title="FILL & STROKE"/>
          <div className="appearance-field"><span>Fill</span><div className="swatch dark-swatch"/><div className="faux-input">#FFFFFF</div></div>
          <div className="appearance-field"><span>Stroke</span><div className="swatch stroke-swatch"/><div className="faux-input">#475569</div></div>
          <div className="appearance-field"><span>Width</span><div className="faux-select">1.5 px<Icon name="chevron-down" size={12}/></div></div>
          <SectionHeading title="TYPOGRAPHY"/>
          <div className="appearance-field"><span>Font</span><div className="faux-select">Inter<Icon name="chevron-down" size={12}/></div></div>
          <div className="appearance-field"><span>Size</span><div className="faux-input">13 px</div></div>
          <div className="align-buttons"><button aria-label="Align left"><Icon name="align-left" size={15}/></button>
            <button className="on" aria-label="Align center"><Icon name="align-center" size={15}/></button>
            <button aria-label="Align right"><Icon name="align-right" size={15}/></button></div>
          <SectionHeading title="GEOMETRY"/>
          <div className="position-grid"><span>X</span><div className="faux-input">130</div><span>Y</span><div className="faux-input">95</div>
            <span>W</span><div className="faux-input">218</div><span>H</span><div className="faux-input">132</div></div>
        </>}
      </div>}
    </section>
  );
}
function Prop({label,children}:{label:string;children:React.ReactNode}) {
  return <div className="property-row"><span>{label}</span><div>{children}</div></div>;
}
