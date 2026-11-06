import * as React from 'react';
import { Icon, IconName } from './Icon';

export function IconButton({ icon, label, active, onClick, small, disabled }: { icon:IconName; label:string; active?:boolean; onClick?:()=>void; small?:boolean; disabled?:boolean; }) {
  return <button type="button" className={`icon-button${active?' is-active':''}${small?' is-small':''}`} aria-label={label} title={label} onClick={onClick} disabled={disabled}><Icon name={icon} size={small?14:16}/></button>;
}
export function SectionHeading({ title, count, action }: { title:string; count?:number|string; action?:React.ReactNode; }) {
  return <div className="section-heading"><span>{title}</span>{count!==undefined&&<span className="subtle-count">{count}</span>}{action&&<div className="section-actions">{action}</div>}</div>;
}
export function PanelHeader({ title, right }: { title:string; right?:React.ReactNode; }) {
  return <div className="panel-header"><span>{title}</span><div className="panel-header-actions">{right}</div></div>;
}
export function Kbd({children}:{children:React.ReactNode}) {return <kbd className="kbd">{children}</kbd>;}
export function Toggle({checked, label}:{checked?:boolean;label:string}) {return <span className={`toggle-switch ${checked?'on':''}`} aria-label={label}><span/></span>}
