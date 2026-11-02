# Componentes visuais UML — contrato entre Zed, Engine-MDE e Cordis

**Regra arquitetural obrigatória:** todos os controles visuais específicos de UML devem pertencer a `crates/bellatrix_ui_uml`. O Workbench **não** implementa suas próprias versões de cartões de classe, linhas de atributos, campos inline ou barras de Quick Edit.

Referência de comportamento: arquivos fornecidos pelo mantenedor em `starUml.zip`, especialmente `src/ui/quickedit-manager.js`, `extensions/essential/uml/quickedits/uml.json`, `extensions/essential/uml/metamodel.json` e classes de Views. A implementação é nativa GPUI/Zed e não copia código, CSS ou assets StarUML.

## Escopo atual de Diagrama de Classes

Este documento é do **plugin/engine MDE**, não da identidade visual global
do Bellatrix. Referências normativas da experiência MDE:

- React: `crates/bellatrix_design_system/docs/design/`.
- Comportamento: `crates/bellatrix_design_system/docs/ref/staruml/`
  (Classes Basic, Classes Advanced, Packages e Annotations).
- Filosofia da plataforma: [PLATFORM_VISION.md](PLATFORM_VISION.md).

O catálogo contém 24 ferramentas com comandos próprios (14 tipos de nós e
10 tipos de relação). Class e Interface permanecem com os tipos legados
`rust.struct` e `rust.trait` para compatibilidade. As outras formas usam
tipos UML específicos. O ProjectService continua revisionando as mudanças.

| Grupo | Elementos |
| --- | --- |
| Nós básicos | Class, Interface |
| Nós avançados | Signal, DataType, PrimitiveType, Enumeration, Frame, N-ary Association, Association Class, Template |
| Pacotes e anotação | Package, Model, Subsystem, Comment |
| Relações básicas | Association, Directed Association, Aggregation, Composition, Dependency, Generalization, Interface Realization |
| Relações avançadas | Template Binding, Realization |
| Relação de pacote | Containment |

No Canvas, os nós compartilham a identidade do tema Zed, **não uma
silhueta genérica**: classifiers usam compartments e estereótipos;
packages/model/subsystem usam aba; frame é uma moldura; comment tem dobra;
N-ary Association usa losango; conectores possuem linhas/setas/triângulos/
diamantes conforme o tipo UML. Relações são `Relation` + `View(Path)`,
não `Element` com geometria de classe.

A Toolbox agrupa cada ferramenta por tipo. Soltar nós cria Elements + Views
com `expected_revision`. Para conectores, escolher a ferramenta e clicar
em **origem e destino** cria a Relation; não inventar relação de ownership
ao soltar fisicamente uma figura sobre outra. Relações são inspecionáveis
e seus multiplicities, roles e navigability são propriedades editáveis.

**Limitações que ainda requerem validação/evolução:** interação nativa em
Windows, roteamento ortogonal/múltiplos bends, edição direta dos endpoints,
Association Class com linha pontilhada vinculada a uma Association,
multiplicity labels posicionados nos extremos e metamodelo UML completo.
A presença das ferramentas não constitui certificação UML integral.
Os testes Cargo/GPUI e casos de uso devem ser aprovados antes de declarar
editor de classes homologado.

## Interface — notação circular e Quick Edit obrigatório

Para **Interface**, a referência é o `UMLInterfaceView` do StarUML (lollipop:
**círculo + nome abaixo**, com compartments de atributos, operações e recepções
quando houver membros). Não desenhar uma Interface como um retângulo de classe
apenas acrescentando o texto `«interface»`.

Fontes originais **somente para estudo/comportamento**, sem copiar código:
`crates/bellatrix_design_system/docs/ref/staruml/extensions/essential/uml/quickedits/uml.json`
(definição `uml.interface`),
`.../uml/uml-commands.js` (handlers `handleAddSubInterface`,
`handleAddSuperInterface`, `handleAddRealizingClass`, `handleAddNote`,
`handleAddConstraint`),
`.../uml/grammars/uml-name.pegjs` (expressão de nome).

| Quick Edit da Interface | Comportamento exigido |
| --- | --- |
| `uml.name-expression` | Campo **no próprio nome**. Aceita `<< stereotype >> [+/#/-/~] name`; Enter confirma via Cordis, Esc cancela |
| `uml.visibility` | Ícone/menu nativo: `public`, `protected`, `private`, `package` |
| `uml.add-note` | Cria Note com sua View e NoteLink para a Interface |
| `uml.add-constraint` | Cria Constraint de propriedade semântica e View com link para a Interface |
| `uml.add-attribute` | Cria membro Attribute com relação `owns` |
| `uml.add-operation` | Cria membro Operation com relação `owns` |
| `uml.add-reception` | Cria membro Reception tipado e mostra seu compartment |
| `uml.add-subinterface` | Cria uma nova Interface abaixo e Generalization **sub → selecionada** |
| `uml.add-superinterface` | Cria uma nova Interface acima e Generalization **selecionada → super** |
| `uml.add-realizing-class` | Cria Class e Interface Realization **classe → interface** |

**Layout do Quick Edit:** visibilidade no alto à esquerda; Note/Constraint à
esquerda na segunda linha; Attribute, Operation, Reception na primeira linha à
direita; Sub-Interface, Super-Interface e Realizing Class na segunda linha à
direita. Apenas ícones com tooltips e rótulos acessíveis, sem criar um painel
textual. As ações aparecem ao entrar na edição do elemento, não na seleção
simples. A relação/entidade é criada em **uma só operação versionada**
`ProjectReplace`; se a validação falhar, não pode sobrar um nó órfão.

O modelo mantém a View de Interface com espaço para o nome e os membros;
as relações se prendem à circunferência do lollipop, não ao retângulo
invisível usado para hit testing. Todas as alterações são ordens tipadas
encaminhadas ao Cordis/ProjectService e não alterações locais em GPUI.

Testes: `bellatrix_mde_cordis/tests/interface_quick_edit.rs` verifica as
seis ações de criação, mais Name Expression, Attribute, Operation,
relacionamentos reais e rejeição de revisões obsoletas.

## Ligações UML — desenho e interação no Canvas

Uma ligação não é um `Element` renderizado como caixa. A engine persiste
uma `Relation { kind, from, to, properties }` e uma `View` de
`Geometry::Path`, no mesmo `ProjectReplace(expected_revision)`.

- **Renderização:** `crates/bellatrix_ui_uml/src/relationship.rs` usa
  `gpui::canvas`, `PathBuilder::stroke/fill` e `window.paint_path`, com
  cores semânticas do tema Zed. **Não** depender de SVG dinâmico sem cor
  de texto resolvida, pois ele pode registrar hitbox sem pintar o traço.
- **Notação:** Association é linha contínua; Directed Association tem seta;
  Dependency e Interface Realization são tracejadas; Generalization e
  Interface Realization terminam em triângulo vazado; Aggregation e
  Composition usam diamantes na extremidade do agregado/todo.
  A associação especializada usa `end1Aggregation` para diferenciar
  `shared` e `composite`, preservando o tipo semântico `uml.association`.
- **Trajetória:** todos os segmentos `Line` e `Cubic` da View são
  projetados com pan/zoom; o Canvas não reduz uma polilinha à última ponta.
  O roteamento inicial e a atualização após mover/redimensionar um nó
  pertencem ao backend MDE.
- **Interação Toolbox:** selecionar uma ferramenta `ModelingTool::Relation`
  arma o modo de ligação e destaca o item na Toolbox. O primeiro clique
  em um nó registra a origem; o segundo clique registra o destino e
  envia `ModelingCreateRelationRequested` via bridge Cordis. O modo pode
  ser cancelado sem alterar o projeto. A ação contextual Quick Edit é
  apenas outro caminho para o mesmo domínio, não o único meio de criar relações.
- **Validação:** testar Association entre `TaskList` e `Task`, escolher
  e alterar os demais tipos de conexão, confirmar a linha visível e
  selecionável, criar múltiplas ligações, movimentar as duas classes,
  conferir origem/destino no Inspector, persistência e reabertura.

Esta implementação ainda precisa ser compilada e aprovada visualmente pelo
mantenedor. Não abrir PR nem disparar CI como substituto dessa validação.

## Responsabilidades

| Módulo | Papel e tipo de estado |
| --- | --- |
| `crates/bellatrix_ui_uml/src/class.rs` | Classifiers UML (Class, Interface, Enumeration etc.): estereótipos, título, compartimentos, membros e edição |
| `crates/bellatrix_ui_uml/src/figures.rs` | Package, Model, Subsystem, Comment, Frame e N-ary Association com silhuetas distintas |
| `crates/bellatrix_ui_uml/src/relationship.rs` | Apresentação SVG de relações UML e decoração de extremidades; não conhece Cordis |
| `crates/bellatrix_ui_uml/src/inline_edit.rs` | Input Zed posicionado **no lugar do nome**; Enter confirma, Escape cancela; ícones mínimos de aceitar/cancelar |
| `crates/bellatrix_ui_uml/src/quick_edit.rs` | Ações contextuais em botões **somente ícones**, tooltip, acessibilidade e controle de visibilidade via menu nativo Zed |
| `crates/bellatrix_mde_modeling/src/quick_edit.rs` | Descritores tipados por `Element.kind`, disponibilidade e comando de cada ação; sem GPUI |
| `crates/bellatrix_mde_workbench/src/quick_edit_view.rs` | Adapter **sem renderização própria**: ações do modelo → controles visuais de `bellatrix_ui_uml`; callbacks tipados |
| `crates/bellatrix_mde_workbench/src/lib.rs` | Canvas `workspace::Item`, seleção, hit testing, pan/zoom, edição temporária e integração com Zed |
| `crates/bellatrix_mde_cordis` | Eventos `ModelingSetProperty`, `ModelingAddMember`, validação, controle de revisão e persistência por `ProjectService` |

## Fluxo de interação mínimo

1. **Um clique na classe:** selecionar o Element por ID, destacar a View e atualizar o Property Editor. **Não** abrir o Quick Edit automaticamente.
2. **Dois cliques no título da classe** ou **F2** com a seleção ativa: substituir o texto do título por `UmlInlineNameEditor` no próprio Canvas. Focar o campo e selecionar o nome existente. Mostrar apenas os botões contextuais suportados pelo tipo: visibilidade (à esquerda), adicionar atributo e adicionar operação (à direita).
3. **Enter ou ícone Confirmar:** enviar `ModelingSetPropertyRequested {element_id, path:"name", value, expected_revision}` via Cordis. A representação só assume o novo nome depois da revisão confirmada do projeto.
4. **Esc ou ícone Cancelar:** descartar o campo temporário; não alterar `ProjectDocument`.
5. **Clique num atributo/operação:** selecionar seu `Element.id`, não mover a classe. **Dois cliques** editam o nome do membro diretamente dentro de seu compartimento.
6. **Adicionar atributo/operação:** um clique no ícone gera `MdeUiCommand::AddMember`; o backend cria um `Element` real e uma relação `owns`; a View e o Model Explorer são atualizados a partir da revisão publicada.
7. **Visibility:** menu nativo com `public`, `protected`, `private`, `package`; nenhuma lista textual de botões permanentemente visível.
8. **Ações ainda não suportadas:** permanecem no contrato de metamodelo, mas não aparecem como botões enganosamente acionáveis. O progresso completo está em [Quick Edit por elemento](quick-edit.md).

## Critérios UX

- A edição acontece sobre o nome renderizado, não em um painel lateral adicional e não num quadro de 300px com textos de ajuda.
- Os controles contextuais não devem ficar visíveis quando o elemento está apenas selecionado.
- Quick Edit não deve ser confundido com Property Editor. O primeiro reúne gestos e ações sobre a View; o segundo mostra propriedades persistentes do Element.
- A aparência herda o tema Zed; usar `ui::IconButton`, `ui::DropdownMenu`, `ui_input::InputField` e cores semânticas, não uma nova biblioteca de widgets.
- Elementos novos (Enumeration, Association, Note, Package, Port, Part etc.) pertencem ao mesmo crate visual `bellatrix_ui_uml`, com seus próprios componentes, e não devem nascer em `bellatrix_mde_workbench`.
- **Escopo atual:** representação e edição direta de nomes em classes/interfaces e membros Attribute/Operation. Edição da expressão UML completa, menus adicionais por tipo, atalhos e manipulação de relações seguem como etapas ainda não homologadas.
- Compilação Windows, interação de double-click, F2, Enter/Esc, validação/revisão e aceitação visual no Zed são gates obrigatórios antes de declarar conclusão.

## Referências

- [Contrato Quick Edit e inventário por elemento](quick-edit.md)
- [Inspector e Property Editor](inspector.md)
- [Projeção UMLClass e menus](quick-edit-class.svg)
- Referências do mantenedor: `starUml.zip` e screenshots de Quick Edit/Model Explorer/Property Editor enviados na conversa. A documentação de domínio é independente do armazenamento de arquivos do Project Panel do Zed.
