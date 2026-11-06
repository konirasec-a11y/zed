# Design system v3 — StarUML workbench study

## Princípios

- **Canvas first:** chrome só para localizar modelos, ferramentas, propriedades e gerenciar painéis.
- **Disclosure:** as 3 áreas inferiores viram abas e começam fechadas; a toolbox mostra 5–6 atalhos e mantém todas as famílias expandidas sob demanda.
- **Respeitar hábitos de desktop:** redimensionar, restaurar, recolher, navegar em árvore, manter o estado de painéis. Sem aparência de dashboard.
- **Densidade equilibrada:** interface 12px, títulos 13px; áreas de clique 28–38px, divisores com 5px de hit-target, sem sombras pesadas nos painéis.
- **Separar semântica de desenho:** `ClassNode`, `ShapeGlyph`, `CanvasShell`, `DockPanelHeader` não dependem uns dos outros para layout.
- **Somente dois temas:** `light` e `dark`. Azul indica seleção/foco. Sem paleta verde ou derivados para o shell.

## Tamanhos em tela

| Área | Tamanho base |
| --- | --- |
| Topbar | 48px |
| Activity rail | 48px |
| Document tabs | 39px |
| Canvas toolbar | 39px |
| Cabeçalho de painel | 36px |
| Inspector tabs | 36px |
| Statusbar | 25px |
| Left stack | 246px (resize) |
| Right stack | 292px (resize) |
| Divisor de painéis | 5px interativo, hairline de 1px |

## Tokens de superfície

`src/styles/tokens.css` é a fonte de verdade. Os componentes usam **papéis** CSS, e não condicionais `theme === ...` para cores.

| Token | Dark | Light |
| --- | --- | --- |
| `--app-bg` | `#1b1d20` | `#f7f8fa` |
| `--surface-1` | `#222427` | `#fdfdfd` |
| `--surface-2` | `#292b30` | `#f3f5f7` |
| `--border` | `#373b41` | `#dee2e7` |
| `--text-strong` | `#f0f1f3` | `#20262e` |
| `--accent` | `#88b1fa` | `#3974cf` |
| `--canvas` | `#24272c` | `#fafbfc` |
| `--selection` | `#83b5ff` | `#3a7fe2` |

## Componentes com contratos independentes

- `App/Workbench`: organiza dimensões, regiões, visibilidade, tema e preset.
- `Topbar`/`LayoutMenu`: controla presets Modeling/Review/Canvas only, inversão e minimapa.
- `ActivityRail`: seleciona Workbench, Symbol Library e Design System.
- `DockPanelHeader`/`ResizeHandle`: disclosure e tamanho — qualquer conteúdo pode morar dentro.
- `WorkingDiagramsPane`, `ToolboxPane`, `ModelExplorerPane`, `PropertyInspectorPane`: papéis dos stacks, sem conhecimento do canvas.
- `DocumentTabs`/`CanvasToolbar`/`CanvasSurface`: conteúdo do editor (sem motor de edição).
- `BottomPanel`: tabs Relationships, Markdown documentation, Diagram thumbnails; apenas uma aba visível.
- `SymbolGallery`, `PaletteEntry`, `ShapeGlyph`: catálogo de referência independente do canvas.

## Acessibilidade e estados

- Todos os itens com ação têm rótulos ou `aria-label`.
- Splitters operáveis por pointer, setas de teclado e Home; duplo clique restaura dimensão.
- Estados de seleção e foco não dependem apenas de cor: há borda, fundo e peso de fonte.
- Os controles da toolbox e inspector são fáceis de reencontrar mesmo colapsados.
- Em viewport estreito (<1150px), o inspetor inicia fechado. Continua acessível na toolbar do canvas.
- Tamanhos/visibilidade são mantidos em `localStorage` no browser; esta é apenas uma conveniência do wireframe.

## O que este design system não faz

Não implementa metamodelo UML/C4, geração de diagrama, drag-and-drop livre de docks, suporte de temas adicionais ou desenho GPUI real. SVG e HTML são representações portáveis de *aparência*, não contratos de domínio.
