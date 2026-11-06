import * as React from 'react';
export type IconName =
  | 'app' | 'layers' | 'shapes' | 'tree' | 'palette' | 'settings' | 'chevron-down' | 'chevron-right'
  | 'search' | 'plus' | 'more' | 'undo' | 'redo' | 'mouse' | 'hand' | 'box' | 'circle'
  | 'diamond' | 'link' | 'zoom-in' | 'zoom-out' | 'grid' | 'focus' | 'panel-bottom'
  | 'eye' | 'lock' | 'check' | 'info' | 'code' | 'warning' | 'close' | 'sliders'
  | 'download' | 'sun' | 'moon' | 'command' | 'align-left' | 'align-center' | 'align-right'
  | 'ruler' | 'arrow-right' | 'file' | 'folder' | 'type' | 'list' | 'help' | 'pin'
  | 'chevrons-left' | 'chevrons-right' | 'fullscreen' | 'copy' | 'image' | 'dots'
  | 'corner' | 'spark' | 'panel-left' | 'panel-right' | 'monitor' | 'layers-two' | 'layout';
export interface IconProps { name: IconName; size?: number; strokeWidth?: number; className?: string; }
const common = { fill:'none', stroke:'currentColor', strokeLinecap:'round' as const, strokeLinejoin:'round' as const };
export function Icon({ name, size=16, strokeWidth=1.75, className }: IconProps) {
  const paths: Record<IconName, React.ReactNode> = {
    app: <><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><path d="M14 14h7v7h-7z"/></>,
    layers: <><path d="m12 3 9 5-9 5-9-5 9-5Z"/><path d="m3 12 9 5 9-5M3 16l9 5 9-5"/></>,
    shapes: <><rect x="3" y="3" width="9" height="9" rx="1"/><circle cx="17.5" cy="17.5" r="4.5"/><path d="M5 20h7"/></>,
    tree: <><path d="M12 3v5M12 8H5v6M12 8h7v6M5 14v3M19 14v3M12 8v9"/><rect x="2" y="17" width="6" height="4" rx="1"/><rect x="9" y="17" width="6" height="4" rx="1"/><rect x="16" y="17" width="6" height="4" rx="1"/></>,
    palette: <><circle cx="12" cy="12" r="9"/><circle cx="7" cy="11" r="1"/><circle cx="11" cy="7" r="1"/><circle cx="17" cy="9" r="1"/><path d="M12 21c-2-1-2-3 0-4s4-1 5-3"/></>,
    settings: <><circle cx="12" cy="12" r="3"/><path d="M10 2h4l.6 2.3 2.1.9 2-.9 2.8 2.8-.9 2 1 2.2L24 12l-2.4.7-1 2.1.9 2-2.8 2.8-2-.9-2.1.9L14 22h-4l-.6-2.4-2.1-.9-2 .9-2.8-2.8.9-2-1-2.1L0 12l2.4-.7 1-2.2-.9-2 2.8-2.8 2 .9 2.1-.9z" transform="translate(1.2 1.2) scale(.9)"/></>,
    'chevron-down': <path d="m6 9 6 6 6-6"/>, 'chevron-right': <path d="m9 6 6 6-6 6"/>,
    search: <><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></>,
    plus:<path d="M12 5v14M5 12h14"/>, more:<><circle cx="5" cy="12" r="1.2"/><circle cx="12" cy="12" r="1.2"/><circle cx="19" cy="12" r="1.2"/></>,
    undo:<><path d="M9 14 4 9l5-5"/><path d="M4 9h9a7 7 0 0 1 0 14"/></>,
    redo:<><path d="m15 14 5-5-5-5"/><path d="M20 9h-9a7 7 0 0 0 0 14"/></>,
    mouse:<path d="M5 3 19 12l-6.3 1.1L10 20z"/>,
    hand:<><path d="M8 13V6a2 2 0 0 1 4 0v5-6a2 2 0 0 1 4 0v6-4a2 2 0 0 1 4 0v9a6 6 0 0 1-6 6h-3a7 7 0 0 1-5.8-3L2 14a2 2 0 0 1 3-2z"/></>,
    box:<rect x="4" y="4" width="16" height="16" rx="1"/>,circle:<circle cx="12" cy="12" r="8"/>,diamond:<path d="M12 2 22 12 12 22 2 12z"/>,
    link:<><path d="M10 13a5 5 0 0 0 7 .5l3-3a5 5 0 0 0-7-7l-2 2"/><path d="M14 11a5 5 0 0 0-7-.5l-3 3a5 5 0 0 0 7 7l2-2"/></>,
    'zoom-in':<><circle cx="10.5" cy="10.5" r="7"/><path d="M10.5 7v7M7 10.5h7M16 16l5 5"/></>,
    'zoom-out':<><circle cx="10.5" cy="10.5" r="7"/><path d="M7 10.5h7M16 16l5 5"/></>,
    grid:<><path d="M3 3h18v18H3zM9 3v18M15 3v18M3 9h18M3 15h18"/></>,
    focus:<><path d="M8 3H3v5M16 3h5v5M3 16v5h5M21 16v5h-5"/><circle cx="12" cy="12" r="3"/></>,
    'panel-bottom':<><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 15h18"/></>,
    eye:<><path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></>,
    lock:<><rect x="5" y="10" width="14" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/></>,
    check:<path d="m5 12 5 5L20 7"/>, info:<><circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7h.01"/></>,
    code:<path d="m8 8-5 4 5 4m8-8 5 4-5 4m-3-12-3 20"/>,
    warning:<><path d="M12 3 2 21h20L12 3z"/><path d="M12 9v5M12 18h.01"/></>,
    close:<path d="M5 5l14 14M19 5 5 19"/>, sliders:<><path d="M4 7h16M4 17h16"/><circle cx="9" cy="7" r="2.5" fill="var(--surface-2)"/><circle cx="15" cy="17" r="2.5" fill="var(--surface-2)"/></>,
    download:<><path d="M12 3v13m-5-5 5 5 5-5M4 17v4h16v-4"/></>,
    sun:<><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M4.9 4.9l1.5 1.5m11.2 11.2 1.5 1.5M19.1 4.9l-1.5 1.5M6.4 17.6l-1.5 1.5"/></>,
    moon:<path d="M20 15.5A8.5 8.5 0 0 1 8.5 4 8.5 8.5 0 1 0 20 15.5Z"/>,
    command:<><path d="M9 8V5a2 2 0 1 0-2 2h10a2 2 0 1 0-2-2v14a2 2 0 1 0 2-2H7a2 2 0 1 0 2 2z"/></>,
    'align-left':<path d="M4 4v16m4-13h12M8 12h8M8 17h12"/>,
    'align-center':<path d="M12 3v18M4 7h16M7 12h10M4 17h16"/>,
    'align-right':<path d="M20 4v16M4 7h12M8 12h8M4 17h12"/>,
    ruler:<><path d="M3 8 16 21l5-5L8 3z"/><path d="m7 12 3-3m1 7 3-3"/></>,
    'arrow-right':<path d="M4 12h16m-6-6 6 6-6 6"/>,
    file:<><path d="M6 2h8l5 5v15H6z"/><path d="M14 2v6h5"/></>,
    folder:<path d="M3 6h7l2 2h9v12H3z"/>,
    type:<path d="M4 7V4h16v3m-8-3v16m-4 0h8"/>,
    list:<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>,
    help:<><circle cx="12" cy="12" r="9"/><path d="M9.5 9a3 3 0 0 1 5.8 1c0 2-3.3 2.5-3.3 5M12 18h.01"/></>,
    pin:<><path d="m14 3 7 7-3 1-4 4-1 6-3-7-7-3 6-1 4-4z"/></>,
    'chevrons-left':<path d="m11 17-5-5 5-5m7 10-5-5 5-5"/>,
    'chevrons-right':<path d="m6 17 5-5-5-5m7 10 5-5-5-5"/>,
    fullscreen:<path d="M8 3H3v5m13-5h5v5M3 16v5h5m13-5v5h-5"/>,
    copy:<><rect x="8" y="8" width="12" height="12" rx="1"/><path d="M16 8V4H4v12h4"/></>,
    image:<><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8" cy="8" r="2"/><path d="m3 18 6-6 4 4 3-3 5 5"/></>,
    dots:<><circle cx="6" cy="6" r="1"/><circle cx="12" cy="6" r="1"/><circle cx="18" cy="6" r="1"/><circle cx="6" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="18" cy="12" r="1"/><circle cx="6" cy="18" r="1"/><circle cx="12" cy="18" r="1"/><circle cx="18" cy="18" r="1"/></>,
    corner:<path d="M4 4v16h16M4 12h9v8"/>,
    spark:<path d="m12 2 2.6 7.4L22 12l-7.4 2.6L12 22l-2.6-7.4L2 12l7.4-2.6z"/>,
    'panel-left':<><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/></>,
    'panel-right':<><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M15 3v18"/></>,
    monitor:<><rect x="2" y="4" width="20" height="15" rx="2"/><path d="M8 22h8m-4-3v3"/></>,
    'layers-two':<><path d="m12 3 9 5-9 5-9-5zM3 13l9 5 9-5"/></>,
    layout:<><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M9 9v12"/></>
  };
  return <svg className={className} width={size} height={size} viewBox="0 0 24 24" strokeWidth={strokeWidth} {...common} aria-hidden="true">{paths[name]}</svg>;
}
