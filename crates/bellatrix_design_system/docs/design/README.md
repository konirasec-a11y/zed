# StarUML — Modern Workbench UI Study (v3)

**React 19 + TypeScript + Vite. Purely a visual prototype.**

The source of truth for this revision is the user's **latest** `staruml.bmp` screenshot (3439×1396) and the source archive `star(2).zip`. Earlier screenshots and visual treatments are not used to justify layout decisions. The project is a proposed modern front end for a StarUML-like modeling workbench, **not** a functional editor or a React implementation intended for production.

## Run

```bash
npm install
npm run dev
```

The generated `preview/standalone.html` is a self-contained **visual inspection build** (legacy lightweight React preview runtime). It is included to make reviewing the UI possible without npm. `src/` is the authoritative Vite project.

## What changed since v2

- **Latest-source fidelity.** Retains Working diagrams, Toolbox, Model explorer, property/style editors, and the auxiliary Relationships / Markdown documentation / Diagram thumbnails views.
- **Two themes.** Neutral Light and Dark; blue denotes selection/focus. No green theme or gradient treatment.
- **Quieter default.** Six quick tools are visible, detailed symbol groups are collapsed; bottom dock and minimap start hidden.
- **Layouts.** Model / Review / Focus presets, collapsible panes, draggable splitters, keyboard-operated separators, panel order inversion and reset.
- **Canvas-first.** Thin chrome, dedicated diagram tabs, selected element indicators, unobtrusive status bar and grid.
- **Consistent UI language.** English labels from the source interface; explanatory documentation is written in Portuguese below.
- **Four design specimens.** UML Class, C4 Container, Mind Map, Flowchart. Symbol catalog covers additional diagram elements; it is a design inventory, not a metamodel.

## Navigating the prototype

Use the left activity rail for the Workbench, Symbol library and Design system. Use the diagram tabs to switch between the four example notations. Drag the thin splitters at the sides and within the panel stacks. Double-click a splitter or press Home on a focused splitter to restore its size; arrow keys adjust it. Collapse individual pane headings; use the small panel icons beside the canvas title for the full side and bottom areas. The **Layout** menu provides presets, side reversal, minimap and reset.

## Source organization

```text
src/
  app/                  layout state, diagram samples, types
  components/
    dock/                DockPanelHeader, ResizeHandle
    layout/              Topbar, ActivityRail, side stack containers, BottomPanel, Statusbar
    panels/              WorkingDiagrams, Toolbox, ModelExplorer, PropertyInspector panes
    canvas/              CanvasShell, document tabs, toolbar
    diagrams/            UML, C4, Mind Map, Flowchart SVG specimens
    palette/             catalog, grouped symbols, shape primitives
    gallery/             symbol library and design system reference
    primitives/          Icon, basic controls
  styles/
    tokens.css           only Dark + Light semantic tokens
    workbench/           original v2 baseline visual treatments
    modern/              v3 shell, panels, canvas and auxiliary overrides
    diagram-*.css        notation-specific drawing styles
preview/
  standalone.html        dependency-free static preview
  v3-*.png               review screenshots
scripts/
  build-offline-preview.cjs
  build-standalone-preview.py
```

## Documents

- `docs/LATEST_STARUML_AUDIT.md` — comparison against the actual 3439×1396 screenshot and supplied source tree.
- `docs/DESIGN_SYSTEM.md` — hierarchy, themes, components, density, states.
- `docs/GPUI_PORT_MAP.md` — GPUI Kit component mapping, native migration boundaries.
- `docs/UX_ACCEPTANCE.md` — browser UI checks and explicit non-goals.
- `docs/ASSET_INVENTORY.md` — inventory of public assets in the original source archives.

## Scope limitations

No drag-create diagrams, free panel docking, auto-layout, file parser, model serialization, pan/zoom engine, business rules, editor commands or runtime integration is implemented. Buttons changing panel state or preview scenes are **navigation for design review**. In GPUI, panel docking should use `Dock` and `Resizable` natively. Original StarUML source is **not** copied into this prototype.
