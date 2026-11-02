import * as React from 'react';
import { Icon } from '../primitives/Icon';

export function DockPanelHeader({ title, open, onToggle, action, id }: {
  title: string;
  open: boolean;
  onToggle: () => void;
  action?: React.ReactNode;
  id: string;
}) {
  return <header className="dock-panel-header">
    <button type="button" aria-expanded={open} aria-controls={id}
      title={`${open ? 'Collapse' : 'Expand'} ${title}`}
      className="dock-panel-toggle" onClick={onToggle}>
      <Icon name={open ? 'chevron-down' : 'chevron-right'} size={13}/>
      <span>{title}</span>
    </button>
    {open && action && <div className="dock-panel-actions">{action}</div>}
  </header>;
}
