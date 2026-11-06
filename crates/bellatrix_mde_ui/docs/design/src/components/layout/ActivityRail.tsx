import * as React from 'react';
import { Icon, IconName } from '../primitives/Icon';
import type { ViewMode } from '../../app/types';
const views: {mode:ViewMode;icon:IconName;title:string}[]=[
  {mode:'editor',icon:'layers',title:'Workspace'},
  {mode:'library',icon:'shapes',title:'Biblioteca de símbolos'},
  {mode:'system',icon:'palette',title:'Design system'}
];
export function ActivityRail({view,onView}:{view:ViewMode;onView:(v:ViewMode)=>void}) {
  return <nav className="activity-rail" aria-label="Navegação principal">
    <div className="rail-main">{views.map(({mode,icon,title})=><button key={mode}
      className={`rail-action ${view===mode?'is-active':''}`} title={title} aria-label={title}
      aria-current={view===mode?'page':undefined} onClick={()=>onView(mode)}><Icon name={icon} size={19}/></button>)}</div>
    <div className="rail-bottom"><span className="rail-sample-mark" title="Estudo de interface">UI</span></div>
  </nav>;
}
