# Engine-MDE — ASG, grafos tipados e transformação de modelos

**Decisão arquitetural (2026-11-02).** Isto é um contrato de direção e de
fronteiras, **não** uma declaração de que uma engine AGT/DPO/TGG já foi
implementada. A engine MDE é apenas uma capacidade do **Bellatrix**, que
continua sendo uma plataforma geral de DevSecOps, computação científica e
desenvolvimento componível. Consultar [PLATFORM_VISION.md](PLATFORM_VISION.md).

Este contrato técnico e sua matriz UI/UX detalham o
[plano mestre de implementação MDE](MDE_IMPLEMENTATION_PLAN.md).
As etapas abaixo são o subroteiro de grafos/transformações, não o plano
completo de autoria LOW CODE e integração Zed.

## Objetivo

Modelar uma aplicação como grafo de sintaxe abstrata tipado/atribuído (ASG),
transformar um modelo válido em outro modelo validável (M2M), e somente
então materializar textos, arquivos e diretórios (M2T/publicação). UI UML,
Property Inspector, Quick Edit, operações por evento e geração de Rust
são projeções, regras ou adaptadores **do mesmo núcleo semântico**,
não implementações paralelas de "Class", "Relation" ou "Project".

### Conceitos que não podem ser confundidos

| Conceito | Contrato |
| --- | --- |
| **Metamodelo** | Tipos de vértices/arestas, atributos, incidência, multiplicidade, herança de capacidades, invariantes e versões |
| **Modelo (ASG)** | Uma instância identificada, tipada e atribuída, com vértices/arestas dirigidas e propriedades; nunca um layout de tela |
| **Regra de transformação** | Padrão de correspondência, pré-condições, NACs, preservações, remoções, criações, mapeamentos e pós-condições |
| **Morfismo/correspondência** | Mapa explícito de elementos preservados/derivados entre grafos, evitando inferir rastreabilidade por nomes |
| **View/Notação** | Projeção do ASG para um editor: posicionamento, zoom, desenho, símbolo, hit testing; não define tipo ou direção da aresta |
| **Transação** | Aplicação atômica de regra, checagem de invariantes e publicação versionada via `ProjectService`/Cordis |
| **Plano de geração** | Resultado puro da transformação M2M/M2T, com diff, rastreabilidade, conflitos e políticas de substituição |
| **Materialização** | Efeito explícito que escreve arquivos após validação/autorização, nunca durante um Render do GPUI |

Uma classe representada como retângulo e uma interface representada como
círculo são tipos semânticos diferentes com **capacidades compartilhadas**
e **notações especializadas**. Um grafo de categorias/tipos não é, por si
só, a UI; a apresentação é um functor/projeção implementável de modo
distinto para UML, árvore, documentação e código.

## Pipeline alvo

```text
Metamodelo fonte (UML + domínios extensíveis)       Metamodelos de destino
                   |                                             |
                   v                                             v
       ASG fonte tipado/atribuído ------M2M------>  ASG de aplicação/código
                    |                                     |
           regras de edição                         M2M de infraestrutura
                    |                                     |
       validação + ProjectRevision         ASG de pacotes/arquivos/configuração
                    |                                     |
       projeções (Canvas, Tree,                   validação + trace model
           Inspector, QuickEdit)                         |
                                                          v
                                           M2T puro (conteúdos/patches)
                                                          |
                                                          v
                                          preview/diff + publish controlado
                                                          |
                                                          v
                                             repositório, build, CI etc.
```

O ASG de infraestrutura precisa ter **tipos próprios**: Package, Directory,
File, Dependency, BuildTarget, ConfigurationProperty, Artifact, etc.
Evitar assumir que a existência atual de elementos `fs.*` e `rust.*`
no mesmo `ModelingSection` já satisfaz a transformação M2M.
A separação entre modelos de origem, destino e correspondência é
**trabalho a implementar/testar**, inclusive compatibilidade dos projetos
legados. O `ModelingSection` persistido continua válido durante a migração.

## Formalização das regras (incremental, com evidências)

Priorizar *typed attributed graph rewriting*. Uma regra deverá ter
identidade e versão, tipos de entrada/saída, padrões de correspondência,
restrições (inclusive negativas), efeito esperado, diagnóstico,
estratégia de conflito, rastreamento de identidades e teste de invariantes.

- **DPO — Double Pushout:** modelar, quando apropriado, `L ← K → R`;
  elementos preservados pertencem à interface `K`. Implementações que
  afirmem semântica DPO devem testar condições de aplicação/gluing,
  incluindo **dangling edges** na remoção de vértices. Excluir
  incidentalmente todas as arestas sem prova **não é** implementação DPO.
- **SPO — Single Pushout:** considerar a política alternativa de remoção
  incidente somente onde a regra e seu contrato explicitarem esse efeito.
  SPO e DPO não são "dois algoritmos intercambiáveis" sem escolher sua
  semântica e domínio de aplicação.
- **M-adhesive:** fundamento para generalizações categóricas e garantias
  de composição/rewrite. Não anunciar que o grafo/linguagem Bellatrix
  forma uma categoria M-adhesive sem definição e demonstração.
- **TGG — Triple Graph Grammars:** candidato futuro para
  *sincronização bidirecional* entre fonte, correspondência e destino.
  **Não obrigatório** para geração unidirecional M2M→M2T do MVP.
- **Condições e conflitos:** definir invariantes de domínio (ex.
  herança, multiplicidade, roles, containment, tipagem de atributos)
  separadamente de validador de topologia, resolução de nomes e estilo
  visual. Toda rejeição preserva o grafo/revisão anteriores.

### Domínio único e fronteiras dos adaptadores

- `bellatrix_mde`: domínio único com contratos serializáveis, identidades,
  versões, `Element`, `Relation`, `View`, **regras puras de grafos**,
  `GraphIndex`, validação e resolução de relacionamentos. Os módulos internos
  `model`, `graph` e `relationships` não são modelos ou crates independentes.
  Pattern matching, rewrite engine, transformações M2M e traces permanecem
  trabalho futuro.
- `bellatrix_mde_cordis`: eventos tipados, lifecycle Cordis, revisão,
  `ProjectReplace`, autenticação/autorização da transação. Não reescrever
  domínios ou recalcular geometricamente arestas nesse adaptador.
- `bellatrix_mde_rust_generator` / `bellatrix_mde_html_exporter`:
  consumidores de um plano/intermediário validado; atualizar gradualmente
  sem quebrar o pipeline atual. O gerador não é dono das regras UML.
- `bellatrix_mde_workbench`, `bellatrix_ui_uml`: projeções Zed GPUI,
  editor de gestos, controles e desenho; **não** validam relações com
  regras copiadas para `match`s individuais.
- `bellatrix_project`: revisões, estado e persistência. Não
  reconstruir um `ProjectService` paralelo por engine.
- Bellatrix core: SDK, composição espaço-temporal e capacidades gerais
  são transversais a vários plugins. Uma engine UML não pode moldar
  a API global por acidente.

A normalização de relações do editor está em
[GRAPH_RELATIONSHIPS.md](GRAPH_RELATIONSHIPS.md). Atributos de notação
e direções source/target ficam no modelo, não nos `IconButton`s.

## Renderização Tera 2.4.0 já integrada ao gerador Rust

A primeira etapa **M2T** utiliza o crate `bellatrix_mde_templates`
para interpolação textual a partir de `GenerationModel` e dos dados
resolvidos pelo adaptador Rust. Consulte
[TERA_GENERATION_PIPELINE.md](TERA_GENERATION_PIPELINE.md).
Essa integração é incremental: o metamodelo de destino com `TargetGraph`
e o `TraceGraph` da transformação M2M ainda precisam ser formalizados,
implementados e testados; não confundir o plano de geração atual
com uma implementação algébrica DPO/SPO/TGG concluída.

## Evolução prática, por critérios de aceitação

1. **Grafo tipado do metamodelo:** declarar `VertexType`,
   `EdgeType`, papéis das extremidades, propriedades, cardinalidade,
   constraints e derivação de capacidades; validar fixtures UML e Rust.
2. **Regra atômica de edição:** criar, renomear e remover com match,
   plano de alterações e invariantes, incluindo casos adversariais de
   arestas pendentes e ciclos. Aplicar apenas por transação Cordis.
3. **M2M de verdade:** `SourceGraph → TargetGraph + TraceGraph`
   determinístico, sem efeitos; gerar IR de código/infraestrutura
   validável, a partir de metamodelos com versionamento.
4. **M2T e diff:** produzir artefatos imutáveis com rastreabilidade,
   checar conflitos/caminhos e permitir publicação explícita;
   repetir a transformação deve ser determinístico e idempotente
   segundo uma política documentada.
5. **Sincronização opcional:** só considerar TGG quando for exigido
   preservar edições no modelo-alvo e propagar alterações bidirecionais;
   testar criação, remoção, conflitos, convergence e inversões.
6. **SDK prova:** duas engines/plugins independentes usando as mesmas
   primitivas, instaláveis/removíveis sem alterar o host Zed,
   respeitando os efeitos temporais do Cordis.

### Frente obrigatória: revisão UI/UX e aceite do editor MDE

Esta frente integra as etapas acima e bloqueia a declaração de aceite
ponta a ponta do editor; não substitui o roteiro semântico/M2M/M2T.
**Baseline histórica:** incremento nativo `b2ef40e`, estado
`IMPLEMENTED_NOT_GUI_VALIDATED`. Código integrado não significa GUI
aprovada. O aceite deve registrar o SHA efetivamente testado, sem fixar
esse commit como requisito para versões futuras.

**Fontes e precedência:** seguir a especificação completa
[UML 2.5.1 (PDF oficial)](https://www.omg.org/spec/UML/2.5.1/PDF), junto aos
arquivos salvos [UML.xmi](standards/uml/20161101/UML.xmi),
[UMLDI.xmi](standards/uml/20161101/UMLDI.xmi),
[PrimitiveTypes.xmi](standards/uml/20161101/PrimitiveTypes.xmi) e
[StandardProfile.xmi](standards/uml/20161101/StandardProfile.xmi).
Conforme a cláusula 2, os arquivos consumíveis por máquina prevalecem
em conflitos com o texto. As **cinco imagens fornecidas são exemplos,
não o limite de cobertura**; os exemplos/tabelas de notação do anexo B.6
não são exaustivos nem definem obrigatoriedade/opcionalidade.

Reutilizar o contrato visual existente nos
[painéis React](../../crates/bellatrix_design_system/docs/design/src/components/panels/)
e nas variantes `preview/v3*.png` do
[preview](../../crates/bellatrix_design_system/docs/design/preview/).
Mockup visual, implementação GPUI nativa, comportamento em runtime e gates
verificados são evidências distintas. Funcionalidades apenas ilustradas
no protótipo não autorizam mutações silenciosas do modelo. Símbolo na
paleta não prova suporte semântico, persistência ou conformidade UML.

#### Matriz de tarefas e critérios de aceitação

Todas as tarefas abaixo permanecem **NÃO ACEITAS** até registrar suas
próprias evidências; o baseline não altera esse estado.

| ID / estado | Revisão e evidência exigida para aceite | Fontes de implementação |
| --- | --- | --- |
| UX-01 / PENDENTE | Working Diagrams: criar, abrir, alternar, renomear e fechar diagramas; seleção, estado vazio e reabertura coerentes com o projeto, sem confundir diagrama com elemento semântico. | [painéis nativos](../../crates/bellatrix_mde_workbench/src/panels.rs), [integração](../../crates/bellatrix_mde_workbench/src/integration.rs) |
| UX-02 / PENDENTE | Toolbox e biblioteca: catálogo compartilhado, busca, agrupamento e disponibilidade por contexto/capacidade; ferramenta indisponível explica o motivo e não altera o modelo. Auditar capacidades herdadas transitivamente, sem listas locais divergentes. | [biblioteca](../../crates/bellatrix_mde_workbench/src/symbol_library.rs), [metamodelo](../../crates/bellatrix_mde/src/uml/metamodel.rs) |
| UX-03 / PENDENTE | Model Explorer: árvore de ownership semântico, não de geometria; expandir, buscar e selecionar mantendo identidade e sincronização com Canvas/Inspector, inclusive elementos sem View. | [hierarquia nativa](../../crates/bellatrix_mde_workbench/src/hierarchy.rs), [hierarquia de domínio](../../crates/bellatrix_mde/src/hierarchy.rs) |
| UX-04 / PENDENTE | Inspector compacto derivado de schema e QuickEdit: propriedades/capacidades herdadas, campos tipados, validação, confirmar/cancelar e consistência entre editores; rejeição não publica revisão nem alteração parcial. | [Inspector](../../crates/bellatrix_mde_workbench/src/property_editor.rs), [QuickEdit](../../crates/bellatrix_mde_workbench/src/quick_edit_view.rs), [schema de inspeção](../../crates/bellatrix_mde/src/inspector.rs) |
| UX-05 / PENDENTE | Canvas: notação, compartimentos, rótulos, geometria, ancoragem, rotas e setas coerentes com source/target; testar zoom, seleção, hit testing, mover/redimensionar e persistir. Criar/editar curvas e pontos de controle pela UI; preservar segmentos ao reancorar, sem conversão silenciosa em reta. | [notação](../../crates/bellatrix_mde/src/uml_notation.rs), [geometria](../../crates/bellatrix_mde/src/uml_geometry.rs), [renderização UML](../../crates/bellatrix_ui_uml/src/), [contrato de relações](GRAPH_RELATIONSHIPS.md) |
| UX-06 / PENDENTE | Acessibilidade e recuperação: navegação por teclado, foco visível e restaurado após fechar/cancelar, rótulos e contraste; erros legíveis, undo/redo e revisões consistentes entre árvore, Canvas e Inspector, inclusive ações repetidas e inválidas. | [workbench](../../crates/bellatrix_mde_workbench/src/), [eventos Cordis](../../crates/bellatrix_mde_cordis/src/) |
| UX-07 / PENDENTE | Dock nativo: redimensionar, recolher e restaurar painéis, mantendo foco, tamanhos e área útil do Canvas; verificar janelas estreitas/largas e temas claro/escuro sem implementar um Dock paralelo ao Zed. | [painéis nativos](../../crates/bellatrix_mde_workbench/src/panels.rs), [Dock do host](../../crates/workspace/src/dock.rs) |
| UX-08 / PENDENTE | Autoria real desde diagrama vazio: criar elementos e relações válidas, editar, remover, desfazer/refazer, salvar e reabrir, verificando IDs, ownership, notação e revisão. Cobrir cada categoria anunciada como suportada; fixtures pré-preenchidas são apenas regressão complementar. Rejeitar relações inválidas com rollback. | [comandos](../../crates/bellatrix_mde/src/commands/), [marco estrutural e fixtures](STRUCTURAL_UML_MILESTONE.md) |
| UX-09 / PENDENTE | Gate nativo: além dos checks focados de domínio, catálogo e contratos, compilar o aplicativo Zed/GPUI completo com a integração MDE e executar os fluxos acima no ambiente nativo do mantenedor. Registrar comando, toolchain, plataforma, SHA, resultado e evidências visuais/interativas. Corrigir falhas e repetir o gate afetado antes do aceite. | [verificação existente](STRUCTURAL_UML_MILESTONE.md), [estado de consolidação](MDE_CONSOLIDATION_STATUS.md) |

Em UX-04/05/08, incluir comportamentos herdados, Interface retangular e
ball/socket com sua semântica, suporte a compartimentos versus supressão
opcional de exibição, assinatura de template, frame com cabeçalho pentagonal, aba/nome
de Package com formas aninhadas visíveis, rótulos externos de Port e
ligação de AssociationClass. São casos de revisão, não uma lista exaustiva
nem suporte já demonstrado pelas fixtures. Respeitar alternativas válidas
de notação: não tornar obrigatório ícone de Component ou triângulo de
Model onde a especificação admite keyword.

Para cada linha, registrar requisito/cláusula UML, metaclasse/capacidade,
cenário positivo e negativo, evidência de domínio, de UI nativa e de
persistência, além das lacunas. Para UX-09, `cargo metadata`, checks
estáticos, testes headless isolados ou um mockup React **não substituem**
build completo nem interação no aplicativo. Etapas bloqueadas ou não
executadas devem permanecer identificadas como tais, nunca como aprovadas.

Manter rastreabilidade separada para os cinco eixos da cláusula 2:
**sintaxe abstrata, sintaxe concreta, intercâmbio de modelos, intercâmbio
de diagramas e semântica**, respeitando suas dependências. Testes visuais
não aprovam intercâmbio XMI/UMLDI nem semântica; registrar cobertura e
limitações por eixo, sem um selo agregado de “UML completo”. O aceite do
editor depende dessa matriz e da validação local pelo mantenedor, não da
quantidade de símbolos ou screenshots reproduzidos.

**Intercâmbio não declarado conforme nesta baseline.** Antes de qualquer
alegação, exigir evidência de importação/exportação e roundtrip XMI/DI,
incluindo perfis e versões exigidas pela cláusula 2/anexo E (carregar e
salvar UML 2.4.1 e 2.5 para intercâmbio de modelos). Intercâmbio de
diagramas depende de sintaxe concreta e intercâmbio de modelos; semântica
depende de sintaxe abstrata. A política beta sobre retrocompatibilidade
da persistência própria Bellatrix não dispensa requisitos oficiais de
intercâmbio nem exige, por si só, manter formatos Bellatrix antigos.

**O que já existe hoje:** grafos/índices e validações parciais,
ProjectDocument revisionado, eventos Cordis, projeções UML e geradores
específicos. **O que não existe hoje:** prova de transformação
algébrica DPO/SPO formal, runtime TGG ou metamodelos separados
de origem/IR/destino com traces bidirecionais.

## Fontes para estudar, não para copiar arquitetura integral

- Artigo do projeto: [A Programming Paradigm for Spatiotemporal
  Composability](AProgrammingParadigmforSpatiotemporalComposability2608.25512v1.pdf).
- [Eclipse Henshin](https://projects.eclipse.org/projects/modeling.henshin):
  transformações endógenas/exógenas, regras e semântica de grafos DPO/SPO.
- Hermann, Ehrig, Orejas et al. (2013), *Model synchronization based on
  triple graph grammars: correctness, completeness and invertibility*,
  DOI: 10.1007/s10270-012-0309-1.
- StarUML (referência **apenas da UI e comportamento MDE**):
  `crates/bellatrix_design_system/docs/ref/staruml/`.
- React (contrato visual **apenas da MDE**):
  `crates/bellatrix_design_system/docs/design/`.

**Política de entrega:** commits apenas em branch de trabalho;
validação local pelo mantenedor antes de solicitar PR. Sem PR e sem CI
voluntários sem autorização explícita.
