# Engine-MDE — ASG, grafos tipados e transformação de modelos

**Decisão arquitetural (2026-11-02).** Isto é um contrato de direção e de
fronteiras, **não** uma declaração de que uma engine AGT/DPO/TGG já foi
implementada. A engine MDE é apenas uma capacidade do **Bellatrix**, que
continua sendo uma plataforma geral de DevSecOps, computação científica e
desenvolvimento componível. Consultar [PLATFORM_VISION.md](PLATFORM_VISION.md).

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

### Separação de crates / fronteiras concretas

- `bellatrix_mde_model`: contratos serializáveis, identidades, versões,
  `Element`, `Relation`, `View`, e futuramente metamodelos e traces.
- `bellatrix_mde_modeling`: **regras puras de grafos** e invariantes,
  `GraphIndex`, validação, resolução de relacionamentos e, futuramente,
  pattern matching, rewrite engine, transformações M2M.
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
