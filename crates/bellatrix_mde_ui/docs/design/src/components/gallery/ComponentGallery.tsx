import * as React from 'react';
import type { DiagramFamily } from '../../app/types';
import { families, diagramTypes } from '../../app/diagramData';
import { catalog, familyTotals } from '../palette/catalog';
import { ShapeGlyph } from '../palette/ShapeGlyph';
import { Icon } from '../primitives/Icon';
export function ComponentGallery({family,onFamily,chosen,onChoose}:{family:DiagramFamily;onFamily:(family:DiagramFamily)=>void;chosen:string;onChoose:(s:string)=>void}){
 return <main className="gallery-view">
  <div className="gallery-header"><div className="gallery-eyebrow">DESIGN LIBRARY / SHAPES</div><h1>Symbol library</h1><p>A reference for diagram primitives. Names and geometry are organized for a direct GPUI component mapping.</p></div>
  <div className="gallery-family-tabs">{families.map(f=><button key={f.id} className={family===f.id?'active':''} onClick={()=>onFamily(f.id)}><span>{f.name}</span><small>{familyTotals[f.id]}</small></button>)}</div>
  <div className="gallery-scroll">
   <div className="library-context"><div><span className="library-title">{families.find(f=>f.id===family)?.name}</span><span className="library-description">{families.find(f=>f.id===family)?.description}</span></div><div className="gallery-meta"><Icon name="box" size={14}/>{catalog[family].length} groups · {familyTotals[family]} symbols</div></div>
   <details className="diagram-types-details"><summary><Icon name="layers-two" size={15}/><span>Diagram types in scope</span><span className="type-count">{diagramTypes[family].length} types</span><Icon name="chevron-down" size={13}/></summary><div className="diagram-type-grid">{diagramTypes[family].map(x=><span key={x}>{x}</span>)}</div></details>
   {catalog[family].map(g=><section className="gallery-section" key={g.id}><div className="gallery-section-head"><h2>{g.title}</h2><span>{g.items.length} items</span></div><div className="symbol-grid">{g.items.map(i=><button className={`symbol-card ${i.id===chosen?'selected':''}`} key={i.id} onClick={()=>onChoose(i.id)}><span className="symbol-preview"><ShapeGlyph kind={i.kind} size={55}/></span><span className="symbol-name">{i.label}</span><span className="symbol-identifier">{i.kind}</span></button>)}</div></section>)}
   <p className="gallery-disclaimer">Catalog entries are visual references. No metamodel, validation rules, serialization, or diagram operations are implemented.</p>
  </div>
 </main>
}
