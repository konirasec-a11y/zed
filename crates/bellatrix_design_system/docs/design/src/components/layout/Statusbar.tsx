import * as React from 'react';
import { Icon } from '../primitives/Icon';
import type { DiagramFamily } from '../../app/types';
export function Statusbar({family}:{family:DiagramFamily}) {
  const label:Record<DiagramFamily,string>={uml:'UML · Class diagram',c4:'C4 · Container diagram',mindmap:'Mind map',flowchart:'Flowchart'};
  return <footer className="statusbar">
    <div className="statusbar-left"><Icon name="layers" size={12}/><span>Model</span>
      <span className="status-sep"/><span>{label[family]}</span></div>
    <div className="statusbar-right"><span>UI study</span><span className="status-sep"/>
      <Icon name="grid" size={12}/><span>20 px</span><span className="status-sep"/><span>100%</span></div>
  </footer>;
}
