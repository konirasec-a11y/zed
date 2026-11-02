# Portabilidade para GPUI / GPUI Kit

**Objetivo:** portar o design system e a composição de painéis. Este projeto React é apenas uma maquete visual com comportamento de layout para validar UX; não constitui uma base de implementação do editor.

Referência de componentes: <https://gpui-kit.com/component/>. Na documentação consultada, GPUI Kit lista `Dock`, `Resizable`, `Collapsible`, `Tree`, `Tabs`, `Toolbar`, `Theme`, `TitleBar`, `StatusBar`, `Input`, `Select`, `Form`, `Tooltip`, `Menu`, `Scrollable` e `Command`. Estes são os **papéis** recomendados, não uma afirmação de que as assinaturas Rust sejam iguais às APIs React.

| Elemento no protótipo | Peças GPUI Kit previstas | Estado nativo necessário |
| --- | --- | --- |
| `App`, `main-frame` | `Dock` + `Resizable` | placement, visibility, workspace preset |
| `Topbar`, `LayoutMenu` | `TitleBar`, `Toolbar`, `Menu` | tema atual, preset, painel invertido |
| `ActivityRail` | `Sidebar` / `Toolbar` | modo ativo |
| `DockPanelHeader` | `Collapsible` | expanded flag para cada pane |
| `ResizeHandle` | `Resizable` | width/height, limits, collapsed, reset |
| `WorkingDiagramsPane` | `Tree`, `Scrollable` | diagrama ativo |
| `ToolboxPane` | `Scrollable`, `Collapsible`, `Input` | família, grupos, busca, ferramenta |
| `ModelExplorerPane` | `Tree`, `Scrollable` | elemento selecionado |
| `PropertyInspectorPane` | `Tabs`, `Form`, `Input`, `Select`, `Toggle` | tab Properties/Style |
| `BottomPanel` | `Dock`, `Tabs` | bottom visible, selected tab |
| `CanvasShell` | `Tabs`, `Toolbar` | active document, viewport |
| `Statusbar` | `StatusBar` | contexto, grid/zoom |
| Tokens Dark/Light | `Theme` | papéis semânticos de cor |
| `ClassDiagram`/`C4Diagram`/… | renderer nativo separado | geometria, hit-testing, event handling **futuros** |

## Separação de responsabilidades

```text
WorkbenchRoot (Theme)
├─ AppHeader (TitleBar + Menu)
├─ ActivityRail (Sidebar)
├─ ComposableDock (Dock + Resizable)
│  ├─ LeftStack [WorkingDiagramsPane, ToolboxPane]
│  ├─ EditorStack
│  │  ├─ DocumentTabs
│  │  ├─ CanvasToolbar
│  │  ├─ DiagramViewport (GPUI renderer)
│  │  └─ AuxiliaryDock [Relationships, Markdown, Thumbnails]
│  └─ RightStack [ModelExplorerPane, InspectorPane]
└─ AppStatusbar
```

## Contratos a preservar

1. O **layout** é dado por `pane_id`, posição de dock, limite mínimo/máximo, tamanho, visibilidade e ordem. Não deve conhecer UML, C4 nem símbolos.
2. Um **panel** expõe seu conteúdo a um container e seu título/capacidade de fechamento. A toolbox não controla seu próprio container de dock.
3. O **tema** define roles (`surface`, `border`, `text`, `canvas`, `selection`), sem opções verdes. Os painéis e SVG specimens não devem consultar variações específicas de tema.
4. A **biblioteca de símbolos** descreve `family`, `group`, `label`, `glyph`; o renderer e metamodelo nativo serão unidades separadas.
5. `LayoutMenu` faz presets, inversão das laterais e reset. Na versão GPUI, `Dock` deve ser a fonte de verdade do placement, inclusive operações de re-dock, que o protótipo **não simula**.
6. A disposição ultrawide e janelas compactas não devem forçar uma única combinação de panes. Reabrir um painel deve restaurar o tamanho que foi salvo.

## Como portar sem acoplamento Web → GPUI

Não copiar o CSS, o DOM, eventos `pointermove` ou estado `localStorage` para Rust. Traduzir cada token para as estruturas de tema GPUI e cada relação entre panes para o Dock/Resizable. Criar storybook/cenas em GPUI para os estados Dark/Light, sides flipped, bottom expanded, toolbox collapsed e foco antes de iniciar o renderer/editor.

Para verificação, repetir os cenários de `docs/UX_ACCEPTANCE.md` no host nativo. O visual deve ser comparável às capturas dentro de `preview/`.
