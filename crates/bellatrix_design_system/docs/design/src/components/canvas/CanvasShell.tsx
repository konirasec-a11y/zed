import * as React from 'react';
import { Icon } from '../primitives/Icon';
import { IconButton } from '../primitives/Controls';
import { ResizeHandle } from '../dock/ResizeHandle';
import { diagrams } from '../../app/diagramData';
import type { DiagramFamily } from '../../app/types';
import { ClassDiagram } from '../diagrams/ClassDiagram';
import { C4Diagram } from '../diagrams/C4Diagram';
import { MindMapDiagram } from '../diagrams/MindMapDiagram';
import { FlowDiagram } from '../diagrams/FlowDiagram';
import { BottomPanel, BottomTab } from '../layout/BottomPanel';

export interface CanvasShellProps {
  family:DiagramFamily; onFamily:(family:DiagramFamily)=>void;
  leftVisible:boolean;rightVisible:boolean;bottomVisible:boolean;focusMode:boolean;
  onLeftToggle:()=>void;onRightToggle:()=>void;onBottomToggle:()=>void;onFocusToggle:()=>void;
  onLayoutReset:()=>void;bottomHeight:number;onBottomResize:(delta:number)=>void;onBottomReset:()=>void;
  bottomTab:BottomTab;onBottomTab:(tab:BottomTab)=>void;
  minimapVisible:boolean;onMinimapToggle:()=>void;
}
export class CanvasShell extends React.Component<CanvasShellProps,{tool:'select'|'pan'}> {
  state:{tool:'select'|'pan'}={tool:'select'};
  render() {
    const p=this.props, diagram=diagrams.find(d=>d.family===p.family)!;
    return <main className="workspace-main">
      <nav className="document-tabs" aria-label="Open diagrams">
        <div className="doc-tab-area">{diagrams.map(d=><button key={d.family}
          className={`doc-tab ${p.family===d.family?'active':''}`} aria-current={p.family===d.family?'page':undefined}
          onClick={()=>p.onFamily(d.family)}>
          <Icon name={d.family==='uml'?'box':d.family==='c4'?'layers':d.family==='mindmap'?'tree':'corner'} size={14}/>
          <span>{d.label}</span></button>)}</div>
        <div className="document-tab-overflow" title="Additional open tabs"><Icon name="more" size={15}/></div>
      </nav>
      <div className="canvas-toolbar">
        <div className="canvas-path"><span>Model</span><Icon name="chevron-right" size={11}/><strong>{diagram.label}</strong>
          <span className="canvas-type-label">{diagram.type}</span></div>
        <div className="canvas-toolbar-controls" aria-label="Tools and panels">
          <div className="canvas-tool-pair" role="group" aria-label="Visual tool">
            <button className={this.state.tool==='select'?'selected':''} title="Select" aria-label="Select" aria-pressed={this.state.tool==='select'} onClick={()=>this.setState({tool:'select'})}><Icon name="mouse" size={14}/></button>
            <button className={this.state.tool==='pan'?'selected':''} title="Pan canvas (visual only)" aria-label="Pan canvas" aria-pressed={this.state.tool==='pan'} onClick={()=>this.setState({tool:'pan'})}><Icon name="hand" size={14}/></button>
          </div>
          <span className="tool-divider"/>
          <IconButton icon="panel-left" small label={p.leftVisible?'Collapse left panels':'Expand left panels'} active={p.leftVisible&&!p.focusMode} onClick={p.onLeftToggle}/>
          <IconButton icon="panel-bottom" small label={p.bottomVisible?'Collapse bottom panel':'Open bottom panel'} active={p.bottomVisible&&!p.focusMode} onClick={p.onBottomToggle}/>
          <IconButton icon="panel-right" small label={p.rightVisible?'Collapse right panels':'Expand right panels'} active={p.rightVisible&&!p.focusMode} onClick={p.onRightToggle}/>
          <span className="tool-divider"/>
          <IconButton icon="focus" small label={p.focusMode?'Exit focus mode':'Focus mode'} active={p.focusMode} onClick={p.onFocusToggle}/>
        </div>
      </div>
      <div className="canvas-surface" data-notation={p.family} data-tool={this.state.tool}>
        <div className="canvas-grid-overlay"/>
        <div className="canvas-content" key={p.family}>
          {p.family==='uml'?<ClassDiagram/>:p.family==='c4'?<C4Diagram/>:p.family==='mindmap'?<MindMapDiagram/>:<FlowDiagram/>}
        </div>
        {p.minimapVisible&&<div className="minimap" aria-label="Diagram minimap">
          <div className="minimap-top"><span>OVERVIEW</span><button className="clean-button" onClick={p.onMinimapToggle} aria-label="Close minimap"><Icon name="close" size={12}/></button></div>
          <div className="minimap-preview"><div className="mini-box a"/><div className="mini-box b"/><div className="mini-box c"/><div className="mini-box d"/><div className="minimap-viewport"/></div>
        </div>}
      </div>
      {p.bottomVisible&&!p.focusMode&&<>
        <ResizeHandle axis="vertical" label="Altura do painel inferior" onDelta={p.onBottomResize} onReset={p.onBottomReset} value={p.bottomHeight} min={98}/>
        <BottomPanel family={p.family} onToggle={p.onBottomToggle} tab={p.bottomTab} onTab={p.onBottomTab} height={p.bottomHeight}/>
      </>}
    </main>;
  }
}
