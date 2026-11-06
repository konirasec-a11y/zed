# Verificações do protótipo v3

## Executado no Chromium (prévia HTML offline carregada em memória)

- [x] Tela padrão nos temas Light e Dark; nenhum tema verde.
- [x] Splitter horizontal esquerdo: largura 246 → 294 px ao arrastar para a direita.
- [x] Splitter vertical Working diagrams: altura 216 → 252 px ao arrastar para baixo.
- [x] Toolbox abre/recolhe sem desaparecer do workspace.
- [x] Layout **Review** exibe dock inferior e recolhe Toolbox.
- [x] Abas do dock inferior: Relationships, Markdown documentation, Diagram thumbnails.
- [x] Layout Reset restaura a tela padrão e fecha dock inferior.
- [x] Swap side panels troca o Model explorer com o stack de diagramas/toolbox, mantendo grid alinhado.
- [x] Focus mode oculta os side stacks; ao sair, eles reaparecem.
- [x] Quatro cenas ativas: UML, C4, Mind map, Flowchart.
- [x] Symbol library e Design system navegam sem erro de renderização.
- [x] Tema Escuro → Claro troca os tokens corretamente.
- [x] Viewport compacta 1080×720 inicia com inspetor fechado.
- [x] Proporção ultrawide 1900×771 renderizada para comparar com a referência 3439×1396.
- [x] Nenhum `pageerror` durante os cenários acima.
- [x] Sintaxe TS/TSX verificada nos 28 arquivos via compilador TypeScript (`transpileModule`).
- [x] CSS parseado via PostCSS em 26 arquivos sem erros.

## Acessibilidade prevista

- Separadores têm `role="separator"`, orientação, valor atual e uso por setas de teclado; Home e double-click restauram dimensão.
- Collapse/expand expõe `aria-expanded`; tabs auxiliares expõem `role="tab"`.
- Ações importantes têm texto ou rótulo acessível; seleção tem contorno visível.

## Limites da verificação

- **Build Vite + checagem de tipos completa não executadas neste ambiente**, porque dependências React/Vite/@types não estão instaladas localmente. Executar `npm install && npm run build` no ambiente de destino.
- Não foi implementado drag-and-drop de símbolos, engine de viewport, canvas editing, docking arbitrário, serialização, metamodelo nem backend.
- O `standalone.html` utiliza um runtime React reduzido específico para inspeção. O projeto oficial é `src/` + Vite.
