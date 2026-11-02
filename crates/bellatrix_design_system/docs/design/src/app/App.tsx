import * as React from 'react';
import type { DiagramFamily, ThemeName, InspectorTab, ViewMode } from './types';
import { Topbar } from '../components/layout/Topbar';
import { ActivityRail } from '../components/layout/ActivityRail';
import { LeftSidebar } from '../components/layout/LeftSidebar';
import { RightInspector } from '../components/layout/RightInspector';
import { ResizeHandle } from '../components/dock/ResizeHandle';
import { Statusbar } from '../components/layout/Statusbar';
import { CanvasShell } from '../components/canvas/CanvasShell';
import type { BottomTab } from '../components/layout/BottomPanel';
import { ComponentGallery } from '../components/gallery/ComponentGallery';
import { DesignSystemView } from '../components/gallery/DesignSystemView';
import { catalog } from '../components/palette/catalog';
import { diagrams } from './diagramData';

interface LayoutState {
  leftWidth: number; rightWidth: number; projectsHeight: number; explorerHeight: number; bottomHeight: number;
  flipped: boolean; leftOpen: boolean; rightOpen: boolean; projectsOpen: boolean; toolboxOpen: boolean;
  explorerOpen: boolean; inspectorOpen: boolean; bottomOpen: boolean; focus: boolean; minimap: boolean;
}
interface AppState {
  family: DiagramFamily; theme: ThemeName; view: ViewMode;
  search: string; chosen: string; inspector: InspectorTab;
  bottomTab: BottomTab; collapsed: Record<string, boolean>; layout: LayoutState;
}
const defaultLayout: LayoutState = {
  leftWidth: 246, rightWidth: 292, projectsHeight: 216, explorerHeight: 230, bottomHeight: 206,
  flipped: false, leftOpen: true, rightOpen: true, projectsOpen: true, toolboxOpen: true,
  explorerOpen: true, inspectorOpen: true, bottomOpen: false, focus: false, minimap: false,
};
const LAYOUT_KEY = 'staruml-ui-layout-v3';
const THEME_KEY = 'staruml-ui-theme-v3';
function clamp(value: number, min: number, max: number) { return Math.max(min, Math.min(value, Math.max(min, max))); }
function readLayout(): LayoutState {
  try {
    const raw = localStorage.getItem(LAYOUT_KEY);
    if (!raw) return { ...defaultLayout, rightOpen: window.innerWidth >= 1150 };
    const saved = JSON.parse(raw) as Partial<LayoutState>;
    const valid = Object.entries(defaultLayout).filter(([key, initial]) => typeof saved[key as keyof LayoutState] === typeof initial);
    return { ...defaultLayout, ...Object.fromEntries(valid.map(([key]) => [key, saved[key as keyof LayoutState]])) };
  } catch { return { ...defaultLayout, rightOpen: typeof window !== 'undefined' && window.innerWidth >= 1150 }; }
}
function readTheme(): ThemeName {
  try { return localStorage.getItem(THEME_KEY) === 'light' ? 'light' : 'dark'; } catch { return 'dark'; }
}
export class App extends React.Component<Record<string, never>, AppState> {
  state: AppState = {
    family: 'uml', theme: readTheme(), view: 'editor', search: '', chosen: '',
    inspector: 'properties', bottomTab: 'relationships', collapsed: {}, layout: readLayout(),
  };
  componentDidUpdate(_oldProps: Readonly<Record<string, never>>, previous: Readonly<AppState>) {
    try {
      if (previous.layout !== this.state.layout) localStorage.setItem(LAYOUT_KEY, JSON.stringify(this.state.layout));
      if (previous.theme !== this.state.theme) localStorage.setItem(THEME_KEY, this.state.theme);
    } catch { /* File-scheme previews may disable storage. */ }
  }
  private patchLayout = (patch: Partial<LayoutState>) =>
    this.setState(s => ({ layout: { ...s.layout, ...patch } }));
  private resize = (key: 'leftWidth' | 'rightWidth' | 'projectsHeight' | 'explorerHeight' | 'bottomHeight', delta: number, min: number, max: number) =>
    this.setState(s => ({ layout: { ...s.layout, [key]: clamp(s.layout[key] + delta, min, max) } }));
  private activateFamily = (family: DiagramFamily) => this.setState({ family, search: '', chosen: '' });
  private leftToggle = () => {
    const l = this.state.layout;
    this.patchLayout({leftOpen: !l.leftOpen || l.focus,
      projectsOpen: !l.projectsOpen && !l.toolboxOpen ? true : l.projectsOpen,
      toolboxOpen: !l.projectsOpen && !l.toolboxOpen ? true : l.toolboxOpen, focus: false});
  };
  private rightToggle = () => {
    const l = this.state.layout;
    this.patchLayout({rightOpen: !l.rightOpen || l.focus,
      explorerOpen: !l.explorerOpen && !l.inspectorOpen ? true : l.explorerOpen,
      inspectorOpen: !l.explorerOpen && !l.inspectorOpen ? true : l.inspectorOpen, focus: false});
  };
  private applyPreset = (preset: 'modeling' | 'review' | 'focus') => {
    if (preset === 'focus') this.patchLayout({focus:true});
    else if (preset === 'review') this.setState(s => ({bottomTab:'documentation', layout:{...s.layout, focus:false, leftOpen:true, projectsOpen:true, toolboxOpen:false, rightOpen:true, explorerOpen:true, inspectorOpen:true, bottomOpen:true}}));
    else this.patchLayout({focus:false, leftOpen:true, rightOpen:true, projectsOpen:true, toolboxOpen:true, explorerOpen:true, inspectorOpen:true, bottomOpen:false});
  };
  render() {
    const { family, theme, view, search, chosen, inspector, bottomTab, collapsed, layout: l } = this.state;
    const selected = catalog[family].flatMap(g => g.items).find(i => i.id === chosen);
    const editor = view === 'editor';
    const showLeft = editor && l.leftOpen && !l.focus && (l.projectsOpen || l.toolboxOpen);
    const showRight = editor && l.rightOpen && !l.focus && (l.explorerOpen || l.inspectorOpen);
    const showBottom = l.bottomOpen && !l.focus;
    const leftLimit = () => Math.max(210, window.innerWidth - 48 - (showRight ? l.rightWidth : 0) - 410);
    const rightLimit = () => Math.max(242, window.innerWidth - 48 - (showLeft ? l.leftWidth : 0) - 410);
    const startWidth = l.flipped ? (showRight ? l.rightWidth : 0) : (showLeft ? l.leftWidth : 0);
    const endWidth = l.flipped ? (showLeft ? l.leftWidth : 0) : (showRight ? l.rightWidth : 0);
    const startVisible = l.flipped ? showRight : showLeft;
    const endVisible = l.flipped ? showLeft : showRight;
    return <div className="app-root" data-theme={theme}>
      <Topbar theme={theme} diagram={diagrams.find(d=>d.family===family)!.label} flipped={l.flipped} minimap={l.minimap}
        onTheme={t => this.setState({theme:t})} onPreset={this.applyPreset}
        onFlip={()=>this.patchLayout({flipped:!l.flipped})}
        onMinimap={()=>this.patchLayout({minimap:!l.minimap})}
        onReset={()=>this.patchLayout({...defaultLayout})}/>
      <div className={`main-frame ${l.flipped?'is-reversed':''}`} style={{gridTemplateColumns:
        `48px ${startVisible?`${startWidth}px 5px`:'0px 0px'} minmax(0, 1fr) ${endVisible?`5px ${endWidth}px`:'0px 0px'}`}}>
        <ActivityRail view={view} onView={v => this.setState({view: v})}/>
        {showLeft && <>
          <LeftSidebar family={family} onFamily={this.activateFamily} search={search}
            onSearch={s => this.setState({search:s})} chosen={chosen} onChoose={s => this.setState({chosen:s})}
            collapsed={collapsed} onToggleGroup={id => this.setState({collapsed:{...collapsed,[id]:!collapsed[id]}})}
            projectsOpen={l.projectsOpen} toolboxOpen={l.toolboxOpen}
            onProjectsToggle={() => this.patchLayout({projectsOpen: !l.projectsOpen})}
            onToolboxToggle={() => this.patchLayout({toolboxOpen: !l.toolboxOpen})}
            projectsHeight={l.projectsHeight}
            onProjectsResize={d => this.resize('projectsHeight',d,85,Math.max(145, window.innerHeight-330))}
            onProjectsReset={() => this.patchLayout({projectsHeight:202})}/>
          <ResizeHandle className="layout-split-left" axis="horizontal" label="Largura da toolbox"
            value={l.leftWidth} min={210} max={leftLimit()}
            onDelta={d => this.resize('leftWidth', l.flipped?-d:d, 210, leftLimit())}
            onReset={() => this.patchLayout({leftWidth:246})}/>
        </>}
        <div className="center-column">{editor ? <CanvasShell family={family} onFamily={this.activateFamily}
          leftVisible={showLeft} rightVisible={showRight} bottomVisible={showBottom} focusMode={l.focus}
          onLeftToggle={this.leftToggle} onRightToggle={this.rightToggle}
          onBottomToggle={() => this.patchLayout({bottomOpen: !l.bottomOpen, focus:false})}
          onFocusToggle={() => this.patchLayout({focus: !l.focus})}
          onLayoutReset={() => this.patchLayout({...defaultLayout})}
          bottomHeight={l.bottomHeight} onBottomResize={d => this.resize('bottomHeight', -d, 98, Math.max(180,window.innerHeight-290))}
          onBottomReset={() => this.patchLayout({bottomHeight:206})}
          bottomTab={bottomTab} onBottomTab={tab => this.setState({bottomTab:tab})}
          minimapVisible={l.minimap} onMinimapToggle={() => this.patchLayout({minimap: !l.minimap})}/>
          : view === 'library' ? <ComponentGallery family={family} onFamily={this.activateFamily}
              chosen={chosen} onChoose={s => this.setState({chosen:s})}/>
          : <DesignSystemView theme={theme}/>}</div>
        {showRight && <>
          <ResizeHandle className="layout-split-right" axis="horizontal" label="Largura do inspetor"
            value={l.rightWidth} min={242} max={rightLimit()}
            onDelta={d => this.resize('rightWidth', l.flipped?d:-d, 242, rightLimit())}
            onReset={() => this.patchLayout({rightWidth:292})}/>
          <RightInspector family={family} tab={inspector} onTab={t => this.setState({inspector:t})}
            explorerOpen={l.explorerOpen} inspectorOpen={l.inspectorOpen}
            explorerHeight={l.explorerHeight}
            onExplorerToggle={() => this.patchLayout({explorerOpen:!l.explorerOpen})}
            onInspectorToggle={() => this.patchLayout({inspectorOpen:!l.inspectorOpen})}
            onExplorerResize={d => this.resize('explorerHeight',d,80,Math.max(180,window.innerHeight-300))}
            onExplorerReset={() => this.patchLayout({explorerHeight:263})}/>
        </>}
      </div>
      {view === 'library' && selected && <div className="selected-toast">{selected.label}<span> · exemplo</span></div>}
      <Statusbar family={family}/>
    </div>;
  }
}
