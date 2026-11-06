import * as React from 'react';
import { Icon } from '../primitives/Icon';
import type { ThemeName } from '../../app/types';

type Preset = 'modeling' | 'review' | 'focus';
interface Props {
  theme: ThemeName; diagram: string; flipped: boolean; minimap: boolean;
  onTheme: (t: ThemeName) => void;
  onPreset: (p: Preset) => void; onFlip: () => void;
  onReset: () => void; onMinimap: () => void;
}
export class Topbar extends React.Component<Props, { open: boolean }> {
  state = { open: false };
  private menuRef: HTMLDivElement | null = null;
  componentDidMount() { document.addEventListener('pointerdown', this.closeOutside); }
  componentWillUnmount() { document.removeEventListener('pointerdown', this.closeOutside); }
  private closeOutside = (e: PointerEvent) => {
    if (this.menuRef && !this.menuRef.contains(e.target as Node)) this.setState({open:false});
  };
  private run = (f:()=>void) => { f(); this.setState({open:false}); };
  render() {
    const p=this.props;
    return <header className="app-titlebar">
      <div className="brand" aria-label="StarUML"><span className="brand-mark"><Icon name="layers" size={18}/></span><span>StarUML</span></div>
      <span className="title-divider"/>
      <nav className="menu-entries" aria-label="Application menu">
        {['File','Edit','Format','Model','Tools','View'].map(item => <span className="menu-entry" key={item}>{item}</span>)}
      </nav>
      <div className="titlebar-fill"/>
      <div className="project-crumb"><Icon name="folder" size={14}/><span>Commerce platform</span><span className="crumb-slash">/</span><strong>{p.diagram}</strong></div>
      <div className="titlebar-fill"/>
      <div className="toolbar-menu-anchor" ref={node=>{this.menuRef=node;}}>
        <button className={`titlebar-control ${this.state.open?'is-open':''}`} aria-haspopup="menu" aria-expanded={this.state.open}
          onClick={() => this.setState(s=>({open:!s.open}))} title="Organize workspace">
          <Icon name="layout" size={15}/><span>Layout</span><Icon name="chevron-down" size={11}/>
        </button>
        {this.state.open && <div className="workspace-menu" role="menu" aria-label="Organização do workspace">
          <div className="menu-caption">WORKSPACE PRESETS</div>
          <button role="menuitem" onClick={()=>this.run(()=>p.onPreset('modeling'))}><Icon name="panel-left" size={15}/><span>Modeling</span><small>Side panels</small></button>
          <button role="menuitem" onClick={()=>this.run(()=>p.onPreset('review'))}><Icon name="panel-bottom" size={15}/><span>Review</span><small>Bottom panel</small></button>
          <button role="menuitem" onClick={()=>this.run(()=>p.onPreset('focus'))}><Icon name="focus" size={15}/><span>Canvas only</span></button>
          <div className="menu-separator"/>
          <button role="menuitemcheckbox" aria-checked={p.flipped} onClick={()=>this.run(p.onFlip)}><Icon name="arrow-right" size={15}/><span>Swap side panels</span>{p.flipped&&<Icon name="check" size={13}/>}</button>
          <button role="menuitemcheckbox" aria-checked={p.minimap} onClick={()=>this.run(p.onMinimap)}><Icon name="eye" size={15}/><span>Minimapa</span>{p.minimap&&<Icon name="check" size={13}/>}</button>
          <button role="menuitem" onClick={()=>this.run(p.onReset)}><Icon name="undo" size={15}/><span>Reset layout</span></button>
        </div>}
      </div>
      <button className="titlebar-control icon-only" title={`Switch to ${p.theme==='dark'?'light':'dark'}`}
        aria-label={`Theme ${p.theme==='dark'?'dark':'light'}. Toggle`} onClick={()=>p.onTheme(p.theme==='dark'?'light':'dark')}>
        <Icon name={p.theme==='dark'?'sun':'moon'} size={16}/>
      </button>
    </header>;
  }
}
