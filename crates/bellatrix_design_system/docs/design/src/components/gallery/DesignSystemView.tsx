import * as React from 'react';
import { Icon, IconName } from '../primitives/Icon';
import { ShapeGlyph } from '../palette/ShapeGlyph';
import type { ThemeName } from '../../app/types';
const surfaceTokens=['--app-bg','--surface-1','--surface-2','--surface-3','--border','--muted','--text','--accent','--canvas','--selection'];
const components:{name:string;icon:IconName;gpui:string}[]=[
 {name:'Dock / Split view',icon:'layout',gpui:'Dock · Resizable'},
 {name:'Diagram tab',icon:'layers-two',gpui:'Tabs'},
 {name:'Tool palette',icon:'shapes',gpui:'Sidebar · Scrollable'},
 {name:'Model explorer',icon:'tree',gpui:'Tree'},
 {name:'Property field',icon:'sliders',gpui:'Input · Select'},
 {name:'Command menu',icon:'command',gpui:'Command · Menu'},
 {name:'Canvas toolbar',icon:'dots',gpui:'Toolbar · Button'},
 {name:'Status bar',icon:'panel-bottom',gpui:'StatusBar'}
];
export function DesignSystemView({theme}:{theme:ThemeName}) {return <main className="design-system-view">
 <div className="gallery-header"><div className="gallery-eyebrow">DESIGN LIBRARY / FOUNDATIONS</div><h1>Interface foundations</h1><p>Semantic tokens, density, component states and naming conventions for the GPUI Kit handoff.</p></div>
 <div className="system-scroll">
  <div className="system-intro"><div className="eyebrow">CURRENT SCHEME</div><div className="current-theme-line"><span className="scheme-visual"/><strong>{theme[0].toUpperCase()+theme.slice(1)}</strong><span>12 px UI typography · 4 px radius · 8 px base spacing</span></div></div>
  <section className="system-section"><div className="system-section-heading"><h2>Color roles</h2><span>Semantic CSS tokens</span></div><div className="token-grid">{surfaceTokens.map(t=><div className="token-card" key={t}><span className="token-swatch" style={{background:`var(${t})`}}/><code>{t.replace('--','')}</code></div>)}</div></section>
  <section className="system-section"><div className="system-section-heading"><h2>Primitive states</h2><span>Visual treatment</span></div><div className="primitives-example"><button className="example-primary">Primary action</button><button className="example-secondary">Secondary</button><button className="example-quiet"><Icon name="plus" size={15}/> Quiet action</button><button className="example-secondary" disabled>Disabled</button><span className="demo-chip">Selected</span><span className="demo-outline">Default</span></div><div className="field-examples"><div><small>INPUT</small><span>OrderService</span></div><div><small>SELECT</small><span>Class <Icon name="chevron-down" size={13}/></span></div><div><small>TOGGLE</small><span>Show labels <span className="mini-toggle"/></span></div></div></section>
  <section className="system-section"><div className="system-section-heading"><h2>Diagram primitives</h2><span>SVG reference specimens</span></div><div className="system-shapes">{(['class','interface','association','person','system','database','topic','decision'] as const).map(k=><div key={k}><ShapeGlyph kind={k} size={49}/><span>{k}</span></div>)}</div></section>
  <section className="system-section"><div className="system-section-heading"><h2>Component anatomy</h2><span>Concept → GPUI Kit</span></div><div className="mapping-grid">{components.map(c=><div className="mapping-row" key={c.name}><Icon name={c.icon} size={17}/><div><strong>{c.name}</strong><span>{c.gpui}</span></div><Icon name="chevron-right" size={13}/></div>)}</div></section>
  <div className="system-footer-note"><Icon name="info" size={14}/> HTML/CSS is the design source here; production GPUI implementation is intentionally excluded.</div>
 </div>
 </main>}
