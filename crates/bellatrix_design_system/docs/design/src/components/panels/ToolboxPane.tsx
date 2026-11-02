import * as React from 'react';
import { Icon } from '../primitives/Icon';
import { DockPanelHeader } from '../dock/DockPanel';
import { catalog } from '../palette/catalog';
import { ShapeGlyph } from '../palette/ShapeGlyph';
import type { DiagramFamily, PaletteEntry } from '../../app/types';

export interface ToolboxPaneProps {
  family:DiagramFamily; search:string; onSearch:(s:string)=>void;
  chosen:string; onChoose:(id:string)=>void;
  collapsed:Record<string,boolean>; onToggleGroup:(id:string)=>void;
  open:boolean; onToggle:()=>void;
}
const favorites:Record<DiagramFamily,string[]>={
  uml:['Class','Interface','Association','Composition','Package','Comment'],
  c4:['Person','Software system','Container / application','Database','Relationship','System boundary'],
  mindmap:['Central topic','Main topic','Subtopic','Branch connector','Callout'],
  flowchart:['Start / end','Process','Decision','Flow line','Document','Database / storage']
};
/** Symbol vocabulary + one search control; no dependence on canvas or panel host. */
export class ToolboxPane extends React.Component<ToolboxPaneProps,{searchOpen:boolean}> {
  state={searchOpen:false};
  private entry=(i:PaletteEntry,quick=false)=><button key={i.id} title={i.label}
    onClick={()=>this.props.onChoose(i.id)}
    className={`palette-entry ${quick?'quick-entry':''} ${this.props.chosen===i.id?'active':''}`}>
    <span className="palette-glyph"><ShapeGlyph kind={i.kind} size={23}/></span>
    <span className="palette-label">{i.label}</span>
  </button>;
  render() {
    const p=this.props, query=p.search.trim().toLowerCase(), searching=query.length>0;
    const groups=catalog[p.family].map(g=>({...g,items:g.items.filter(i=>i.label.toLowerCase().includes(query))})).filter(g=>g.items.length);
    const all=catalog[p.family].flatMap(g=>g.items);
    const quick=favorites[p.family].map(name=>all.find(i=>i.label===name)).filter((x):x is PaletteEntry=>!!x);
    return <section className={`toolbox-panel ${p.open?'':'dock-closed'}`}>
      <DockPanelHeader title="Toolbox" open={p.open} onToggle={p.onToggle} id="toolbox-contents"
        action={<button className="header-icon" aria-label={this.state.searchOpen?'Close search':'Search tools'}
          title="Search symbols" onClick={()=>this.setState(s=>({searchOpen:!s.searchOpen}))}>
          <Icon name="search" size={14}/></button>}/>
      {p.open&&<div className="toolbox-main" id="toolbox-contents">
        {this.state.searchOpen&&<label className="side-search"><Icon name="search" size={13}/>
          <input autoFocus value={p.search} placeholder="Search symbols…" aria-label="Search symbols"
            onChange={e=>p.onSearch(e.currentTarget.value)}/>
          {p.search&&<button className="clean-button" onClick={()=>p.onSearch('')} aria-label="Clear search"><Icon name="close" size={13}/></button>}
        </label>}
        <div className="palette-scroll">
          {!searching&&<section className="palette-quick"><div className="palette-quick-header">QUICK TOOLS</div>
            <div className="palette-quick-list">{quick.map(i=>this.entry(i,true))}</div></section>}
          <div className="palette-browser-label">{searching?'RESULTS':'SYMBOL LIBRARY'}</div>
          {groups.map(g=><section key={g.id} className="palette-section">
            <button className="palette-group-header" onClick={()=>p.onToggleGroup(g.id)} aria-expanded={searching||!!p.collapsed[g.id]}>
              <Icon name={searching||p.collapsed[g.id]?'chevron-down':'chevron-right'} size={12}/>
              <span>{g.title}</span><span className="group-count">{g.items.length}</span>
            </button>
            {(searching||!!p.collapsed[g.id])&&<div className="palette-entries">{g.items.map(i=>this.entry(i))}</div>}
          </section>)}
          {!groups.length&&<div className="empty-compact">No symbols found.</div>}
        </div>
      </div>}
    </section>;
  }
}
