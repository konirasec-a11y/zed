# Contrato único de relações — Engine MDE

Base e próximos mecanismos de tipagem/AGT:
[MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md).

Esta especificação descreve **o domínio do grafo da engine MDE**, não o host
Zed nem um EventBus da plataforma Bellatrix. A fonte de verdade é sempre
`ModelingSection`, persistido pelo `ProjectService` com revisão.

## Responsabilidades, sem duplicação

- `crates/bellatrix_mde/src/model/graph.rs`, `crates/bellatrix_mde/src/model/diagram.rs`: tipos persistidos
  `Element`, `Relation {id,kind,from,to,properties}`, `Diagram`,
  `View {subject,geometry}`. IDs são estáveis.
- `crates/bellatrix_mde/src/view-model/diagram/relationships.rs`: **único contrato**
  de papéis source/target, tipo de relação, regras de criação, notação UML,
  roteamento, conexão de endpoints, lanes e projeções de relações legadas.
  `RelationshipGraph` consulta as arestas visíveis no diagrama.
- `crates/bellatrix_mde/src/KERNEL/graph/graph.rs` e `index.rs`: integridade
  de IDs, estrutura, ciclos e índice `petgraph` para consultas.
- `crates/bellatrix_mde_cordis/`: transações, eventos públicos, permissões
  futuras, revisão esperada e `ProjectReplace`. Criação via Toolbox e Quick
  Edit **reusa** os contratos do grafo. Mover/redimensionar pede
  `reroute_relationships`. Nenhum módulo Cordis reimplementa geometria.
- `crates/bellatrix_mde_workbench/`: estado de gesto e escolha da ferramenta,
  seleção, pan/zoom, projeção `RelationshipGraph::paths()`, comandos via
  bridge. Não interpreta `relation.kind` para definir seta/diamante.
- `crates/bellatrix_ui_uml/src/relationship.rs`: **somente desenho**
  GPUI `PathBuilder`, cor, segmentos e `RelationshipStyle` recebido do
  domínio. Sem inferência de regra de negócio, nem gestão de projeto.
- `Model Explorer` usa a mesma consulta de arestas
  `RelationshipGraph::visible_relations()` e
  `relationship_presentation().describe(from,to)`; não aplica filtros
  divergentes como `kind.starts_with("uml.")`.

## Direção: ordem do gesto ≠ posição da ponta do símbolo

O grafo armazena **aresta orientada `from → to`**. Para a Toolbox,
primeiro clique define `from`, segundo clique define `to`. O Quick Edit
não recebe dois cliques: cria o novo nó e usa `quick_relation_rule(action)`
para escolher quem ocupa `from`/`to` segundo o papel UML.

| Relação | `from` | `to` | Notação na tela |
| --- | --- | --- | --- |
| Association | Primeiro participante | Segundo participante | Linha contínua sem seta de navegabilidade |
| Directed Association | Origem | Destino navegável | Seta aberta em `to` |
| Dependency (`depends_on`) | Cliente dependente | Fornecedor | Tracejado e seta aberta em `to` |
| Generalization | Subtipo (específico) | Supertipo (geral) | Triângulo vazado em `to` |
| Interface Realization | Implementador | Interface realizada | Tracejado com triângulo em `to` |
| Realization | Implementação | Especificação | Tracejado com triângulo em `to` |
| Template Binding | Cliente vinculado | Template | Tracejado, seta em `to` |
| Aggregation | Todo (`end1`) | Parte (`end2`) | Diamante vazio em **`from`** |
| Composition | Todo (`end1`) | Parte (`end2`) | Diamante cheio em **`from`** |
| Containment | Contêiner | Contido | Círculo de contenção em **`from`** |

Para Aggregation/Composition, o tipo persistido é
`uml.association` com `end1Aggregation=shared|composite`. Tipos legados
`uml.aggregation`/`uml.composition` são interpretados de modo compatível.
**Não inverter as identidades do grafo para conseguir um símbolo visual
diferente!** O triângulo e o diamante têm lados UML definidos,
independentemente da posição horizontal dos nós.

## Rotas e comportamento ao mover

1. Consultar `RelationshipGraph` para todas as arestas com `View(Path)`
   no diagrama ativo, inclusive `ref-task`/`depends_on`, não só `uml.*`.
2. `routing=automatic`: calcular âncoras nas bordas atuais dos nós,
   distribuindo relações paralelas em lanes com caminhos distintos.
3. Caminhos legados/manuais com geometria curva: **preservar os segmentos**
   e deslocar proporcionalmente os pontos de controle para as novas
   âncoras. O vínculo semântico fica `Relation.from/to`, nunca nas
   coordenadas antigas.
4. A consulta `RelationshipGraph::paths()` é **pura**, feita durante a
   renderização; abrir o diagrama não grava dados nem incrementa revisão.
   Somente eventos de edição versionados persistem
   `reroute_relationships` no `ProjectService`.
5. A UI transforma Path em pixels locais usando zoom/pan e pinta com
   `UmlRelationGlyph`; não mantém uma segunda geometria de domínio.

## Tipagem futura de grafos e composibilidade

`Element.kind`, `Relation.kind` e propriedades definem hoje um
metamodelo aberto. A evolução será um catálogo de tipos de vértices e
arestas com contratos explícitos para fonte/alvo, papéis, cardinalidade,
composição de operações (morfismos) e invariantes; as consultas puras
continuarão aqui, **sem atribuir regras MDE ao core geral do Bellatrix**.
`Graph::validate` valida a integridade básica já implementada.
Não afirmar que o sistema de tipagem de grafos ou a composição dinâmica
de plugins estão concluídos.

## Critérios locais de aceitação

- O projeto `todo.json` exibe a Dependency curva entre TaskList e Task
  no Canvas **e** no Model Explorer, sem "(project)".
- Mover TaskList ou Task reancora a curva ao novo lugar,
  mantendo seus controles Bézier e a mesma identidade `ref-task`.
- Criar Dependency + Composition entre os mesmos nós gera caminhos
  separados. O diamante aparece no todo (`from`), a seta Dependency
  no fornecedor (`to`).
- Criar Sub-Class no Quick Edit desenha triângulo no supertipo;
  criar Super-Class também. Realizing Class usa triângulo na Interface.
- Tanto Toolbox quanto Quick Edit persistem a revisão via Cordis;
  nenhum deles possui regras de seta próprias.
- Seleção, inspeção, exclusão e persistência de relações existentes
  seguem a mesma projeção do domínio.

**Fluxo de entrega:** publicar na branch de desenvolvimento. Validação local
pelo mantenedor; **sem PR/merge ou execução manual de CI** sem solicitação.
