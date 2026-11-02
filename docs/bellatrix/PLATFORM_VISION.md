# Bellatrix — visão de plataforma e fronteiras obrigatórias

> Decisão de produto vigente (2026-11-02). Esta página prevalece sobre descrições legadas
> que apresentam Bellatrix como "a Engine-MDE", um editor de texto ou um editor UML.
> É uma diretriz arquitetural; **não** declara concluído o runtime público de plugins.

## Identidade

**Bellatrix é uma plataforma generalista, extensível e componível para criação de
aplicações pelos próprios usuários e desenvolvedores.** Domínios-alvo incluem
DevSecOps, computação científica e outros sistemas especializados. A plataforma
oferecerá SDKs para contribuições de **lógica e UI**; uma aplicação poderá combinar
várias capacidades sem ter de reimplementar workspace, editor de código, dock,
projetos, temas, transportes e lifecycle.

**Zed e MDE são capacidades integradas ao produto (core-bundled) hoje e deverão
poder ser tratados como plugins futuramente.** Zed é o host/editor visual na
arquitetura atual, **não é a definição do Bellatrix**. MDE é uma **engine**
(bounded context de domínio) enquanto o sistema generalista de plugins ainda
não existe. **Não chamá-la de plugin dinâmico já implementado.**

### Composibilidade espaço-temporal — fundamento, não decoração

Fonte acadêmica obrigatória:
[A Programming Paradigm for Spatiotemporal Composability](AProgrammingParadigmforSpatiotemporalComposability2608.25512v1.pdf)
(Yifan Shi, Wei Zhang, Tianyi Cui, 2026).

- **Temporal:** efeitos pertencentes à ativação de um componente (serviços,
  subscriptions, tarefas e recursos) têm encerramento e recuperação definidos
  pelo runtime, sem apagar os projetos e resultados que o usuário persistiu.
- **Espacial:** componentes declaram dependências; disponibilidade/alteração
  dos serviços pode ativar/desativar componentes por contratos, não por
  descobrir manualmente uma ordem de inicialização.
- **Mediação:** a implementação de backend utiliza os mecanismos efetivamente
  disponíveis de `cordis-rs` (Context, Plugin, InjectSpec, Service, Event,
  FiberHandle), sem criar EngineManager, EventBus ou outro runtime concorrente.

O objetivo é atingir extensibilidade dinâmica comparável em nível de
composição às ideias demonstradas pelos harnesses de agentes, incluindo o
DeepSeek-Harness, **sem afirmar que esse nível já foi atingido**. O artigo
formaliza garantias; usá-lo como inspiração não prova as garantias no código
Rust atual. O SDK deverá demonstrá-las com testes de instalar, remover,
reativar, isolar e compor capacidades independentes.

## Limites de responsabilidade

| Domínio | Responsável atual | Responsabilidade futura |
| --- | --- | --- |
| Plataforma Bellatrix | host + `bellatrix_project` + Cordis-rs | extensão/SDK, contratos, capacidades compartilhadas e políticas |
| UI e edição de código | Zed GPUI, editor, Workspace, Dock, Project/worktree | capacidade core disponibilizada por contrato de plugin |
| Model-Driven Engineering | `bellatrix_mde_*` | engine MDE futuramente publicável como plugin |
| UI da experiência MDE | `bellatrix_mde_workbench` / `bellatrix_ui_uml` | contribuição de UI pertencente ao plugin MDE |
| Design visual da **MDE** | React do estudo MDE abaixo | referência de layout/UX desse plugin, não identidade visual universal da plataforma |
| Comportamento de modelagem | StarUML abaixo | inspiração funcional e comparação, não dependência/cópia de código proprietário |

**Não** confundir `bellatrix_project::ProjectDocument` (estado de domínio
persistido e revisionado) com `project::Project` do Zed (worktrees, arquivos,
buffers). Nem a UI nem o Zed são donos da semântica de uma engine MDE.

## Referências obrigatórias de UI — com escopo explícito

Estas duas fontes estão sob a crate de design system por conveniência de
versionamento, mas representam **o design e o comportamento da experiência
MDE**, não o design de toda a plataforma Bellatrix:

1. **Aparência, layout, interação visível da MDE:**
   `crates/bellatrix_design_system/docs/design/`
   (React: `src/`, `docs/DESIGN_SYSTEM.md`, `docs/UX_ACCEPTANCE.md`,
   `src/components/diagrams/`, `src/components/palette/`, `preview/`).
2. **Comportamento do editor/modelagem:**
   `crates/bellatrix_design_system/docs/ref/staruml/`
   (Electron/StarUML: metamodelo UML, Factory, Engine, Diagram,
   Repository, Toolbox, Quick Edit, conectores). **Consultar, não copiar
   código/recursos proprietários sem autorização.**
3. **Integração nativa/temas/UX do host:** as APIs e implementações de GPUI,
   UI, Workspace, Project e editor presentes **nesta revisão do fork do Zed**.

**Antes de qualquer mudança visual de uma engine**, declarar no PR quais
arquivos React e StarUML foram consultados; confrontar formas, gestos, menus,
acessibilidade e estados com o tema ativo do Zed. Componentes de domínio de
uma engine não podem ser usados como identidade visual genérica da plataforma.

A maquete React não executa o editor GPUI e não fornece metamodelo/drag-drop
real. O StarUML é referência comportamental; a modelagem semântica e
persistência Bellatrix são implementações próprias, tipadas e testadas.

## Critérios verificáveis para declarar a futura composibilidade

1. Capacidade de backend instalável e removível sem reiniciar nem prejudicar
   outra engine independente, respeitando os limites de efeitos rastreados.
2. Dependências declaradas e reavaliadas no runtime Cordis.
3. Contribuição de UI instalável/removível em Workspace/Dock sem outra GUI,
   vazamento de listeners, panes ou tarefas e sem interferir em comandos Zed.
4. Capacidade sem UI funcionando headless; capacidade com UI usando o mesmo
   tema ativo da plataforma.
5. Projetos e dados de usuário sobrevivem a unload/reload. Efeitos de runtime
   e emissões persistidas são tratados de maneira distinta.
6. SDK/documentação, versões, isolamento, permissões e migração de contratos
   definidos e exercitados por **pelo menos dois plugins independentes**.

Até esses testes passarem, usar os termos **engine** e **contribuição integrada**
para Zed/MDE, não afirmar "plataforma de plugins de terceiros concluída".

## Arquitetura científica da Engine-MDE

**Obrigatório para as próximas etapas:** [MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md)
define ASGs tipados/atribuídos, transformação M2M antes de M2T,
regras algébricas, rastreabilidade entre modelos e critérios de aceitação.
O [contrato de relações](GRAPH_RELATIONSHIPS.md) é o primeiro corte
concreto dessa separação entre grafo e notação.

## Escopo de implementação em andamento

A primeira engine MDE inclui diagrama UML de classes como caso real para
provar as APIs: nós com representações diferentes, relações orientadas,
edição contextual, DnD da toolbox, inspector, pan/zoom, save/load. A
representação semântica e os eventos são próprios da MDE; nenhum card GPUI
é a fonte de verdade de um Element.
