# Auditoria da referência atual — StarUML

## Referência efetivamente utilizada

- Captura enviada: `staruml.bmp`, **3439 × 1396 px**.
- Arquivo-fonte enviado: `star(2).zip` com `src/`, `resources/`, `extensions/`, `package.json`.
- As capturas históricas e o protótipo v2 servem apenas para identificar o que *não* deve regredir.

## Regiões reconhecíveis na captura

| Captura original | Novo design v3 | Padrão adotado |
| --- | --- | --- |
| Barra superior File / Edit / Format / Model / Tools / View… | Titlebar e menu compactos | sem decoração |
| Working diagrams | Stack superior esquerdo | recolhível, altura independente |
| Toolbox com grupos extensos | Quick tools + biblioteca recolhida | busca progressiva; conteúdo íntegro por notação |
| Canvas quadriculado e abas | Document tabs + canvas | prioridade de espaço, grade discreta |
| Model explorer | Stack superior direito | árvore separada e redimensionável |
| Editors: Styles / Properties / Documentation | Inspetor com abas Style / Properties e detalhes | informação sob demanda |
| Relationships | Uma aba do dock inferior | escondido por padrão |
| Markdown Documentation | Uma aba do dock inferior | escondido por padrão |
| Diagram Thumbnails | Uma aba do dock inferior | escondido por padrão |
| Minimap | Opção no menu Layout | não ocupa canvas sem solicitação |
| Status bar | Barra de status reduzida | estado local e escala |

## Código original consultado

- `src/utils/resizer.js` declara suporte a painéis redimensionáveis e `show/hide/toggle`, persistência de tamanho e eventos de colapso.
- `src/views/sidebar-view.js` agrupa Working diagrams e Toolbox sem torná-los um painel indivisível.
- `src/views/toolbox-view.js` trabalha com grupos e ferramentas específicos das notações.
- `src/views/model-explorer-view.js` contém a árvore do modelo.
- `src/ui/panel-manager.js` coordena o espaço disponível para a área do editor.
- As extensões `relationship-view`, `markdown` e `diagram-thumbnails` fornecem os painéis auxiliares independentes.

## Decisões de UX

1. Reduzir **exposição simultânea**, não retirar capacidades. As três faixas inferiores viram abas em uma área redimensionável.
2. Mostrar poucas ferramentas frequentes por padrão e permitir expandir as demais. Não ocultar o catálogo atrás de ícones sem nome.
3. Preservar semântica de painéis, splits e tabs como componentes independentes; permitir inversão das laterais no protótipo.
4. Manter espaço de diagrama como elemento central. Minimapa, popovers e ações contextuais não ficam permanentemente sobre o desenho.
5. Modo Escuro e Claro exclusivamente. Tons neutros para superficies; azul para seleção/foco.
6. Usar nomes de painéis iguais ou próximos à referência atual, facilitando futuro mapeamento para Rust/GPUI.

**Não foi feita cópia ou migração de código-fonte do StarUML.** Esta é uma interpretação visual estruturada para futura implementação nativa.
