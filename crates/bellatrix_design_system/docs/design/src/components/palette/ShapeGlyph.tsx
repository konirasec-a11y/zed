import * as React from 'react';
import type { ShapeKind } from '../../app/types';

export function ShapeGlyph({kind,size=30}:{kind:ShapeKind;size?:number}) {
 const base={fill:'none',stroke:'currentColor',strokeWidth:1.5,strokeLinejoin:'round' as const,strokeLinecap:'round' as const};
 const line=<><path d="M4 15h32"/><path d="m30 11 6 4-6 4"/></>;
 let shape:React.ReactNode;
 switch(kind){
  case 'class':shape=<><rect x="6" y="4" width="28" height="22" rx="1"/><path d="M6 11h28M6 19h28"/><path d="M10 7h12M10 15h8M10 22h12"/></>;break;
  case 'interface':shape=<><circle cx="20" cy="9" r="6"/><path d="M20 15v12M11 27h18"/></>;break;
  case 'enum':shape=<><rect x="6" y="4" width="28" height="22"/><path d="M6 15h28M13 9h14M11 20h15"/></>;break;
  case 'association':case 'relation':case 'message':shape=line;break;
  case 'aggregate':shape=<><path d="m4 15 7-5 7 5-7 5zM18 15h18"/></>;break;
  case 'composition':shape=<><path d="m4 15 7-5 7 5-7 5z" fill="currentColor"/><path d="M18 15h18"/></>;break;
  case 'inheritance':shape=<><path d="m5 15 10-6v12zM15 15h21"/></>;break;
  case 'dependency':shape=<><path d="M4 15h32" strokeDasharray="4 3"/><path d="m30 11 6 4-6 4"/></>;break;
  case 'actor':case 'person':shape=<><circle cx="20" cy="6" r="4"/><path d="M20 10v11m-8-6h16m-8 6-6 7m6-7 6 7"/></>;break;
  case 'usecase':case 'terminator':shape=<ellipse cx="20" cy="15" rx="16" ry="8"/>;break;
  case 'lifeline':shape=<><rect x="9" y="3" width="22" height="9"/><path d="M20 12v17" strokeDasharray="4 3"/></>;break;
  case 'package':shape=<path d="M5 8h13l3 4h14v15H5zM5 8V4h13v4"/>;break;
  case 'note':case 'callout':shape=<path d="M6 4h21l7 7v17H6zM27 4v7h7M10 16h16M10 21h11"/>;break;
  case 'state':shape=<><rect x="5" y="8" width="30" height="16" rx="7"/><path d="M11 16h18"/></>;break;
  case 'fork':shape=<><rect x="5" y="13" width="30" height="5" fill="currentColor"/><path d="M20 3v10M12 18v10M28 18v10"/></>;break;
  case 'activity':case 'process':shape=<rect x="5" y="6" width="30" height="19" rx="3"/>;break;
  case 'decision':shape=<path d="M20 2 37 15 20 28 3 15z"/>;break;
  case 'component':shape=<><rect x="8" y="6" width="27" height="20"/><rect x="5" y="10" width="8" height="5" fill="var(--surface-2)"/><rect x="5" y="18" width="8" height="5" fill="var(--surface-2)"/></>;break;
  case 'deployment':shape=<><path d="M8 9 20 2l13 7v14l-13 7-12-7zM8 9l12 8 13-8M20 17v13"/></>;break;
  case 'boundary':shape=<><rect x="4" y="4" width="32" height="23" rx="2" strokeDasharray="4 3"/><path d="M9 10h12"/></>;break;
  case 'system':shape=<><rect x="4" y="5" width="32" height="22" rx="3" fill="currentColor" opacity=".18"/><rect x="4" y="5" width="32" height="22" rx="3"/><path d="M10 13h20M10 18h13"/></>;break;
  case 'container':shape=<><rect x="5" y="4" width="30" height="23" rx="2"/><path d="M5 10h30M10 17h20M10 21h13"/></>;break;
  case 'database':shape=<><ellipse cx="20" cy="8" rx="13" ry="5"/><path d="M7 8v14c0 7 26 7 26 0V8M7 15c0 7 26 7 26 0"/></>;break;
  case 'external':shape=<><rect x="4" y="5" width="32" height="22" rx="3" strokeDasharray="3 3"/><path d="m15 12 5 5 6-7"/></>;break;
  case 'topic':shape=<rect x="3" y="8" width="34" height="14" rx="7" fill="currentColor" fillOpacity=".2"/>;break;
  case 'branch':shape=<><path d="M2 15h12l7-7h17"/><circle cx="21" cy="8" r="2"/><path d="M25 15h12"/></>;break;
  case 'document':shape=<path d="M6 4h28v22c-10-8-18 8-28 0z"/>;break;
  case 'input':shape=<path d="M11 5h26l-8 21H3z"/>;break;
  case 'connector':shape=<><circle cx="20" cy="15" r="10"/><path d="m16 15 3 3 6-6"/></>;break;
  case 'predefined':shape=<><rect x="4" y="6" width="32" height="19"/><path d="M9 6v19M31 6v19"/></>;break;
  default:shape=line;
 }
 return <svg viewBox="0 0 40 30" width={size} height={size*.75} {...base} aria-hidden="true">{shape}</svg>;
}
