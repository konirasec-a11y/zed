export type DiagramFamily = 'uml' | 'c4' | 'mindmap' | 'flowchart';
export type ViewMode = 'editor' | 'library' | 'system';
export type ThemeName = 'dark' | 'light';
export type InspectorTab = 'properties' | 'styles';
export type ShapeKind =
  | 'class' | 'interface' | 'enum' | 'association' | 'aggregate' | 'composition'
  | 'inheritance' | 'dependency' | 'note' | 'package' | 'actor' | 'usecase'
  | 'lifeline' | 'message' | 'activity' | 'state' | 'fork' | 'component'
  | 'deployment' | 'boundary' | 'system' | 'person' | 'container' | 'database'
  | 'external' | 'relation' | 'topic' | 'branch' | 'callout' | 'process'
  | 'decision' | 'terminator' | 'document' | 'input' | 'connector' | 'predefined';
export interface PaletteEntry { id: string; label: string; kind: ShapeKind; }
export interface PaletteGroup { id: string; title: string; items: PaletteEntry[]; }
