# Inspector MDE: contrato de objeto, metamodelo e edição de propriedades

> **Prioridade de implantação:** [Quick Edit contextual por tipo de elemento](quick-edit.md) — ações, grupos, atalhos e comandos do HTML/JSON StarUML fornecido pelo mantenedor. O Inspector e seu Property Editor são superfícies **complementares**, não substituem o Quick Edit. Nenhum SVG autoral substitui os controles reais.

![Ações contextuais por Class View — referência das opções do StarUML](quick-edit-class.svg)

**Estado:** contrato de implementação da Engine-MDE, com validação funcional e UX
no Zed ainda pendente. A autoridade semântica é o PDF e o XMI oficiais UML 2.5.1,
não este documento. As capturas StarUML, o desenho React Bellatrix e as ilustrações
abaixo são referências de UX; não substituem tipos, propriedades ou restrições
UML e não implicam copiar código proprietário.

Ilustrações complementares (secundárias ao Quick Edit): [Inspector e propriedades](inspector-reference.svg), [fluxo entre MDE, Cordis e persistência](inspector-contract.svg). A segunda ilustração foi corrigida para ser um SVG bem-formado.

## 1. Definição obrigatória

**Inspector é uma capacidade do domínio MDE: inspeção das propriedades e operações do elemento selecionado.** O Property Editor é sua representação visual editável. O Inspector **não** é um formulário fixo de nome e caminho de projeto.

| Responsabilidade    | Contrato                                                                      | Onde aparece                       |
| ------------------- | ----------------------------------------------------------------------------- | ---------------------------------- |
| Model Explorer      | Objetos semânticos + estrutura de propriedade/contensão `owns`                | Painel lateral                     |
| Inspector           | Projeção tipada do objeto atualmente selecionado                              | Contrato da engine, sem GPUI       |
| Property Editor     | Campos de texto, booleano, enum, número, documentação, referências e coleções | Abaixo do Model Explorer           |
| Project Panel       | Worktrees, arquivos e diretórios                                              | Painel nativo do Zed; não duplicar |
| Project persistence | Save/Load, ID, revisão, snapshots e eventos                                   | Plataforma Bellatrix/Cordis        |
| Canvas              | Views, posição, geometria e seleção                                           | `workspace::Item` do Zed           |

A seleção de uma classe no Canvas ou no Model Explorer mostra **propriedades da classe**, não apenas seu nome. A seleção de um atributo/operação mostra um conjunto diferente de propriedades. A UI é uma projeção dinâmica do tipo do objeto.

## 2. Metamodelo e composição em Rust

O estado semântico canônico é `ModelingSection.registry`, um `ruml::Registry` de
contratos concretos, como `Contract::Class`, `Contract::Property`,
`Contract::Operation`, `Contract::Association` e `Contract::Generalization`.
As referências são IDs dos contratos. `ownedAttribute`, `ownedOperation`,
`nestedClassifier`, `memberEnd` e `ownedEnd` mantêm seus papéis distintos.
Uma ponta de associação não se torna atributo da classe apenas por ser Property.

`ModelingProjection.contracts` oferece uma cópia imutável do registro do snapshot
para a UI consultar tipos e papéis concretos. Não é um segundo estado editável.
Os registros genéricos `Element`/`Relation` ainda usados por adaptadores são
projeções descartáveis, não a autoridade persistente. Inspector, Canvas, Explorer
e Quick Edit enviam comandos pelo mesmo fluxo e reconciliam a seleção com o novo
snapshot. O estado de edição transitória não altera diretamente os contratos.

Posição, tamanho, geometria, estilo e rótulos de apresentação pertencem à `View`,
não a `ruml::Class`. Isso corresponde à separação UMLDI/DI/DC: `Shape.bounds` usa
`Bounds` com `x`, `y`, `width` e `height`; UMLDI associa figuras aos elementos UML.
O contrato nativo `View`/`Geometry`/`Style` preserva esses dados separadamente.
Descrição e demais metadados de aplicação também permanecem separados; não são
declarados como atributos UML de toda metaclasse.

### Esquema mínimo por tipo

| Origem                        | Propriedades editáveis                                                   | Editor/observações                                                                            |
| ----------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| NamedElement                  | `name`, `visibility`                                                     | Apenas para contratos que conformam a NamedElement                                            |
| Classifier/RedefinableElement | `isAbstract`, `isLeaf`                                                   | Herdadas conforme a metaclasse; `isFinalSpecialization` tem suporte pendente abaixo           |
| Class                         | `isActive`                                                               | Boolean específico de Class                                                                   |
| Property                      | `type`, `defaultValue`, `visibility`, `isStatic`, `isReadOnly`           | Referências tipadas e propriedades do contrato; multiplicidade usa lowerValue/upperValue      |
| Operation                     | `visibility`, `isStatic`, `isAbstract`, `isQuery`                        | Parâmetros são contratos próprios; tipo e limites de retorno são derivados                    |
| Association                   | `name`, papéis, multiplicidades e agregações das pontas                  | Os atalhos das pontas modificam seus contratos Property, não campos inventados na Association |
| Apresentação                  | `label`, `description`, estilo e geometria                               | Metadados separados do registro semântico                                                     |
| Membros                       | `ownedAttribute`, `ownedOperation`, `ownedReception`, `nestedClassifier` | IDs com papéis de composição concretos; `owns` é uma projeção desses papéis                   |

Os defaults não alteram o documento ao abrir o Inspector. **Nenhuma propriedade
pode ser editada sem schema, representação no contrato e validação.** Mostrar um
valor genérico não autoriza persistir uma propriedade arbitrária no registro.

Cada objeto tem um único editor `name`. O espelho `properties.name` do registro
não aparece como um segundo campo editável: escrever apenas nesse espelho seria
sobrescrito pelo nome do objeto na conversão. Nas relações NamedElement, o
caminho do editor continua `properties.name`.

### Edição das pontas de Association

O Inspector da Association mostra **Association ends** na ordem de `memberEnd`.
Selecionar uma ponta abre o Inspector do seu contrato Property, reutilizando a
seleção global e os mesmos controles. Em AssociationClass, `ownedEnd` não aparece
na lista Attributes; `ownedAttribute` e `ownedEnd` são disjuntos (§11.8.2.4).

O campo **Navigation knowledge** oferece `navigable`, `notNavigable` e `unverified`
somente nas pontas possuídas pela Association. As duas primeiras escolhas
atualizam `navigableOwnedEnd`. A terceira preserva a falta de decisão no contrato
de autoria separado, sem inventar um terceiro valor Boolean UML. A operação
reutiliza a validação de domínio e não modifica ruml. Pontas pertencentes a um
Classifier permanecem navegáveis e em leitura apenas (§9.9.17.7,
`Property::isNavigable()`). O resultado derivado permanece visível.

Esse editor substitui o antigo Boolean genérico `properties.navigable`, que não
representava a navegabilidade individual das pontas e não podia ser persistido
como campo da Association. Abrir o Inspector não toma uma decisão de navegação.

### Multiplicidade e intercâmbio MDJ

O campo **Multiplicity notation** é um adaptador de edição: escreve referências
`lowerValue`/`upperValue` para contratos `LiteralInteger` e
`LiteralUnlimitedNatural` possuídos pelo MultiplicityElement. Não persiste uma
string `multiplicity` no contrato Property. Os valores derivados aparecem no
Inspector; sem referências, os limites seguem os defaults UML, normalmente 1..1
(§7.8.8.6–7 e XMI `MultiplicityElement-lowerValue`/`upperValue`).

Limpar a notação restaura a ausência dos limites. A operação só remove literais
exclusivos sem conteúdo adicional; referências externas, apresentação associada,
expressões não literais e ownership inconsistente bloqueiam a alteração inteira.
Editar um intervalo também não modifica silenciosamente um literal usado por
outro elemento. Expressões UML não avaliáveis permanecem visíveis em leitura
apenas, com o motivo indicado, até haver um editor apropriado.

A importação MDJ promove associações binárias representáveis e suas duas pontas
possuídas para Association/Property. Multiplicidade omitida ou vazia segue a
notação UML de exatamente 1 (§9.5.4); a diferença textual entre campo ausente e
`""` fica na provenance de intercâmbio e é preservada na exportação. Limites
explícitos usam os mesmos contratos e papéis de composição do editor nativo.
Navegação não decidida conserva o contrato de autoria separado descrito acima.

Essa tradução ainda não cobre toda a semântica MDJ: pontas possuídas por
classificadores, qualifiers, expressões de multiplicidade não literais e campos
sem representação no ruml atual, como `Property::isID`, permanecem opacos com
diagnóstico. São capacidades pendentes do adaptador/contrato, não ausências da
especificação UML. A exportação rejeita alterações sem representação reversível
em vez de descartá-las. A validação funcional deste lote depende de publicação e
execução dos fluxos de integração, além da verificação visual no Zed.

### Referências de Generalization e InterfaceRealization

Os campos `specific`/`general` da Generalization e
`implementingClassifier`/`contract` da InterfaceRealization usam uma edição
atômica. Além da referência, o adaptador atualiza os endpoints projetados. Ao
mudar a source, move a composição especializada e seu oposto. Na
InterfaceRealization, os membros adicionais de `client`/`supplier` são
preservados; somente a participação especializada antiga é substituída.

O alvo deve existir, ter a metaclasse exigida e respeitar ownership. Referências
nulas, estados anteriores contraditórios e ciclos de Generalization são
rejeitados sem alterar o documento. A validação final dos contratos continua
obrigatória (UML 2.5.1 §§9.9.7.5 e 10.5.6.4).

Cada ocorrência gráfica conserva seu nome, estilo e geometria. Apenas o vínculo
do lado alterado é atualizado, usando uma View existente do novo alvo quando ela
é inequívoca. Sem essa ocorrência, ou com várias candidatas, o lado fica sem
attachment e a projeção mostra o diagnóstico de rota; nenhuma figura é criada
automaticamente. O vínculo do outro lado permanece intacto. Marcadores que não
possam ser reconciliados com segurança causam rejeição atômica.

O retarget de uma relação com rota Manual ainda é rejeitado explicitamente para
não desenhar o caminho antigo como se estivesse ligado ao novo alvo. Preservar
as curvas e reancorar apenas suas pontas exige um editor adicional; mudar a
referência não autoriza descartar o caminho manual.

### Suporte pendente, sem alterar ruml neste lote

- `Classifier::isFinalSpecialization` existe em UML 2.5.1 §9.9.4.5 e no XMI
  `Classifier-isFinalSpecialization`, mas não em `ruml::Class` atual. O Inspector
  mantém a propriedade visível como `not in current contract` / `Unavailable`,
  sem oferecer uma edição que seria rejeitada. É uma lacuna da implementação,
  não uma limitação da UML.
- `Property::class` existe em §9.9.17.6 e no XMI `Property-class`, mas não no
  contrato Property atual. O papel `Class::ownedAttribute` continua preservado;
  o adaptador não inventa um campo ausente para armazenar a referência oposta.
- `Generalization` especializa DirectedRelationship, não NamedElement (§9.9.7).
  Não possui nome UML; não se deve confundir o rótulo de uma figura com `name`.
  Association, Dependency e InterfaceRealization podem editar seu nome no
  Inspector e inline usando `properties.name`, com o mesmo estado de edição.

As lacunas acima precisam ser resolvidas antes de declarar cobertura integral do
diagrama de classes. Propriedades derivadas ou read-only continuam identificadas
como tal, sem confundir essa restrição normativa com ausência no contrato.

### Pontos de integração

- `crates/bellatrix_mde/src/view-model/inspector/inspector.rs`: `InspectorProperty`, `PropertyEditor`, `ElementInspector`, `inspect_element(section, id)`.
- `crates/bellatrix_mde/src/application/edits/inspector_edit.rs`: `set_inspected_property(section, id, path, value)`; os contratos compartilhados de edição ficam em `application/edits/property_edit.rs`.
- `crates/bellatrix_mde_cordis/src/set_property.rs`: `ModelingSetPropertyRequested`, evento `bellatrix.mde.set-property.v1`, validação e `ProjectReplace` com revisão esperada.
- `crates/bellatrix_mde_workbench/src/UI/property_editor.rs`: Zed GPUI consome os descritores e escolhe controles; a edição usa `MdeUiBridge` e não modifica `ModelingSection` diretamente.

## 3. Contrato de seleção e persistência

```text
Canvas ou Model Explorer seleciona Element ID
    → MDE apresenta ElementInspector (base + propriedades do tipo + membros)
    → Property Editor constrói os controles adequados
    → usuário modifica uma propriedade
    → ModelingSetPropertyRequested {
         project_id, element_id, path, value, expected_revision
       }
    → Cordis Event / schema / Graph::validate
    → ProjectService::ProjectReplace (controle concorrente de revisão)
    → ProjectStateChangedEvent
    → Canvas + Model Explorer + Property Editor atualizam do novo snapshot
```

A UI transmite booleanos como `true/false` e valida enums; o adaptador respeita a
representação concreta atual de ruml. Proibir alterações genéricas de `id`,
`kind` ou relações por JSON path. Criar/deletar membros usa comandos de domínio.
Operações da classe são contratos Operation; clicar no membro abre seu Inspector.

**Espaço-temporal:** Zed controla a UI, Dock, foco e estado efêmero. Cordis controla composição e eventos de backend; a UI não inventa barramento. Fechar a janela ou descarregar uma engine não remove documentos persistidos.

## 4. UX — mínimo reproduzível

A UI acompanha o tema ativo do Zed. A árvore do Model Explorer contém somente objetos do domínio; a árvore de arquivos do Zed permanece no Project Panel. O Inspector ocupa o **Property Editor**, separado do botão de salvar projeto. Ao mudar a seleção, o esquema inteiro muda; textos em edição não são substituídos a cada render.

Selecionar `Class1` na árvore ou no Canvas mostra suas propriedades e documentação.
`isFinalSpecialization` fica visível com o suporte pendente indicado acima, não
como Boolean editável. Selecionar Property/Operation muda a especialização. A
classe mostra atributos/operações por seus papéis concretos e permite navegar aos
IDs. As referências de UX não podem acrescentar propriedades sem respaldo UML.

### Critérios de aceitação verificáveis

1. Class → propriedades comuns + especializadas corretas, incluindo visibilidade como enum, flags suportadas como booleans e lacunas explicitamente indisponíveis.
2. Attribute/Operation → propriedades específicas; nenhuma flag exclusiva de Class deve aparecer.
3. Editar `name`, `label`, documentação, enum e bool → salvar, reabrir e preservar valor e tipo.
4. Edição concorrente ou revisão obsoleta → rejeição, com feedback; não sobrescrever silenciosamente.
5. `id` e `kind` → leitura apenas; referências e listas não podem ser alteradas com texto JSON arbitrário.
6. Attributes/Operations → navegação para o elemento do membro; podem ter seu próprio Inspector.
7. Projeto sem seleção → instrução de seleção, não propriedades de objeto falso.
8. Theme Zed claro/escuro/personalizado → controles nativos herdam cores e foco do tema ativo.
9. `Project file path` não pode ser apresentado como propriedade de uma classe.
10. **Gates pendentes:** documentação multiline verdadeira, referência tipada e list editor com add/remove/reorder, persistência de estado do Inspector e testes visuais nativos. Não declarar esses itens prontos antes de implementar.

## 5. Referências e imagens

- [UML 2.5.1, PDF oficial OMG](https://www.omg.org/spec/UML/2.5.1/PDF): §§7.8.9, 9.9.4, 9.9.7, 9.9.17, 11.8.1, 11.8.3 e anexo B.
- [UML.xmi](standards/uml/20161101/UML.xmi) e [UMLDI.xmi](standards/uml/20161101/UMLDI.xmi), com imports [DI.xmi](standards/uml/imports/20131001/DI.xmi) e [DC.xmi](standards/uml/imports/20131001/DC.xmi).
- Capturas StarUML anexadas pelo mantenedor nesta conversa, incluindo o objeto selecionado e seu editor de propriedades. O conjunto original deve ser retido como referência visual de aceitação.
- Arquivo de referência fornecido: `starUml.zip`, `src/views/property-editor-view.js`, `src/views/model-explorer-view.js`, templates `src/static/html-contents/property-editor-view-item-*.html` e `extensions/essential/uml/metamodel.json`.
- Design React Bellatrix: [código e protótipo](https://github.com/konirasec-a11y/bellatrix/tree/main/docs/design), [captura dark v3](https://github.com/konirasec-a11y/bellatrix/blob/main/docs/design/preview/v3-dark.png).
- Diagramas originais **versionados com esta especificação**: [seleção e Inspector](inspector-reference.svg) e [contrato Inspector/Cordis](inspector-contract.svg).

**Aviso:** as imagens do StarUML são referência de comportamento e UX, não autorização para copiar assets ou código proprietário. As duas ilustrações SVG neste diretório são próprias e documentam essa referência.

## Shared view appearance: reference inventory (2026-11-03)

Appearance belongs to a diagram `View`, never its semantic subject. Two views of
one subject can have different appearances. Every visual family uses the same
`Style` contract and shared renderer appearance helpers. Style availability does
not depend on metaclass property descriptors or kernel coverage.

Reference behavior and icon meaning are taken from
`crates/bellatrix_design_system/docs/ref/staruml/src/static/html-contents/style-editor-view.html`
and `src/views/style-editor-view.js` in the same StarUML tree. The supplied
Bellatrix design provides composition and reusable components through
`docs/design/src/components/panels/PropertyInspectorPane.tsx`,
`docs/design/src/components/primitives/Icon.tsx`, and
`docs/design/src/styles/workbench/inspector.css` under the design-system crate.
The design is incomplete: it is not a substitute for the complete StarUML
behavior/icon inventory. Existing repository restrictions prohibit copying
proprietary StarUML source, CSS, or assets into the shipping implementation.
The horizontal alignment vectors use the supplied Bellatrix design geometry;
vertical variants, layer-order, line-pattern, and routing vectors are independently
drawn from the referenced meanings. The bundled StarUML `package.json` names
MKLabs Co.,Ltd. as copyright holder; no blanket permission to redistribute its
proprietary icon assets has been established. Existing theme roles and Zed controls remain
native rather than embedding the reference web interface.

Current implementation status (not a full parity claim):

| Reference capability                                           | Current implementation and remaining work                                                                                                                                                                                                                                                                                                                                                               |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Font family and size                                           | Live per-view inputs, rendered through shared appearance; theme inheritance when empty. Native font picker/presets remain pending.                                                                                                                                                                                                                                                                      |
| Fill, line, text color; inherit                                | Compact icon-first inputs and swatches, immediate receipts and theme reset. Full native color-picker popover remains pending.                                                                                                                                                                                                                                                                           |
| Outline width and opacity                                      | Shared persisted style, existing renderer projection, compact live inputs.                                                                                                                                                                                                                                                                                                                              |
| Horizontal and vertical text alignment                         | Six icon controls, persisted optional enums, shared classifier/figure/relationship-label projection. Model JSON/protobuf and receipt-state tests cover values; native visual acceptance remains pending.                                                                                                                                                                                                |
| Send to back / bring to front                                  | View-local stack command; also forward/backward by one layer. One sorted canvas child list interleaves edges and nodes so paint and hit-test stacking agree. Core view-local/semantic-isolation/roundtrip tests added; native pointer acceptance remains pending.                                                                                                                                       |
| Straight/oblique, rectilinear, rounded rectilinear, curve      | Straight, automatic orthogonal, and curve already routed by the model and now icon controls. Rounded orthogonal corners remain pending.                                                                                                                                                                                                                                                                 |
| Solid, dotted, dashed strokes                                  | Optional common `LineMode`, live icon controls and explicit inherit action; core accepts per-view patterns without modifying relationships. View-local persistence flows pass. Shared stroke resolution and rectangular-outline helper project patterns to relationship/figure paths and classifier/action/state/package/port/part/frame outlines. Native rendering/pointer acceptance remains pending. |
| Flat/stick/solid head and tail                                 | View-only overrides pending. Default UML-required relationship markers must remain intact; changing a visual marker must never mutate relation kind.                                                                                                                                                                                                                                                    |
| Auto resize                                                    | Pending shared view-format command and renderer-measured bounds.                                                                                                                                                                                                                                                                                                                                        |
| Show shadow and word wrap                                      | Common optional Style flags with live icon controls and shared native surface/text helpers. Shadow applies to the view frame; SVG exports a drop-shadow filter. Native wrapping uses renderer text layout; SVG word wrapping is not implemented and is documented as an export limitation.                                                                                                              |
| Stereotype display (none/label/decoration/icon variants)       | Common inherited/label/hidden modes implemented for applied labels; required UML keywords remain visible. Decoration/icon variants remain pending actual stereotype image mapping.                                                                                                                                                                                                                      |
| Show visibility, namespace, property, type                     | Visibility/type/property-annotation flags share the existing typed member/operation formatter in native canvas and SVG; original semantic values remain unchanged. Namespace remains pending its separate header-line projection, rather than being spliced into names.                                                                                                                                 |
| Suppress attributes, operations, receptions, literals, columns | Common attribute/operation/reception flags filter existing classifier compartments without changing members or view bounds. Literal/column compartments are not yet projected and their controls remain pending.                                                                                                                                                                                        |
| Align selected objects left/center/right/top/middle/bottom     | Pending multi-view selection and geometry transaction. Text alignment is distinct and does not implement these operations.                                                                                                                                                                                                                                                                              |
| Equal horizontal/vertical spacing                              | Pending multi-view geometry transaction.                                                                                                                                                                                                                                                                                                                                                                |
| Equal width/height/size                                        | Pending multi-view geometry transaction.                                                                                                                                                                                                                                                                                                                                                                |

`SetViewStyle`, `SetViewRouting`, and `ReorderView` validate identifier uniqueness,
view references, geometry, and style without running semantic kernel validators
or association materialization. Other commands retain full semantic validation.
The same project revision/receipt channel serializes property and appearance
edits. Input admission, selection binding, original target, stale revision,
conflict preservation, and theme reset continue through the existing draft gate.
No Apply/Cancel cycle or parallel semantic store is introduced.

### Export consistency and limits

Native renderers and SVG share `Style::resolve_stroke_pattern`, so explicit
solid/dotted/dashed modes, valid legacy custom patterns, and inherited notation
use one resolver. SVG projects horizontal/vertical text alignment for rectangle
and ellipse labels and preserves view stack order. The export remains a basic
geometry renderer: polygon/path labels, full UML compartments/markers, word wrapping, native
font metrics, and live IDE-theme reproduction are not yet equivalent to the
native canvas. These are limitations, not enabled controls that silently claim
export fidelity. One public integration flow covers edit command, protobuf
save/reload, HTML export artifacts, and unchanged semantic objects.
