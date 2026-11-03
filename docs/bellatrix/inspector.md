# Inspector MDE: contrato de objeto, metamodelo e edição de propriedades

> **Prioridade de implantação:** [Quick Edit contextual por tipo de elemento](quick-edit.md) — ações, grupos, atalhos e comandos do HTML/JSON StarUML fornecido pelo mantenedor. O Inspector e seu Property Editor são superfícies **complementares**, não substituem o Quick Edit. Nenhum SVG autoral substitui os controles reais.

![Ações contextuais por Class View — referência das opções do StarUML](quick-edit-class.svg)

**Estado:** especificação normativa mínima da Engine-MDE. A projeção tipada e a primeira implementação de edição escalar foram publicadas no fork; não considerar homologada até passar nos testes Rust e na validação UX no Zed. O desenho de referência vem das capturas StarUML compartilhadas pelo mantenedor, do protótipo React Bellatrix e do código de referência StarUML. As ilustrações abaixo foram produzidas para documentar o contrato, sem copiar código proprietário.

Ilustrações complementares (secundárias ao Quick Edit): [Inspector e propriedades](inspector-reference.svg), [fluxo entre MDE, Cordis e persistência](inspector-contract.svg). A segunda ilustração foi corrigida para ser um SVG bem-formado.

## 1. Definição obrigatória

**Inspector é uma capacidade do domínio MDE: inspeção das propriedades e operações do elemento selecionado.** O Property Editor é sua representação visual editável. O Inspector **não** é um formulário fixo de nome e caminho de projeto.

| Responsabilidade | Contrato | Onde aparece |
| --- | --- | --- |
| Model Explorer | Objetos semânticos + estrutura de propriedade/contensão `owns` | Painel lateral |
| Inspector | Projeção tipada do objeto atualmente selecionado | Contrato da engine, sem GPUI |
| Property Editor | Campos de texto, booleano, enum, número, documentação, referências e coleções | Abaixo do Model Explorer |
| Project Panel | Worktrees, arquivos e diretórios | Painel nativo do Zed; não duplicar |
| Project persistence | Save/Load, ID, revisão, snapshots e eventos | Plataforma Bellatrix/Cordis |
| Canvas | Views, posição, geometria e seleção | `workspace::Item` do Zed |

A seleção de uma classe no Canvas ou no Model Explorer mostra **propriedades da classe**, não apenas seu nome. A seleção de um atributo/operação mostra um conjunto diferente de propriedades. A UI é uma projeção dinâmica do tipo do objeto.

## 2. Metamodelo e composição em Rust

O elemento-base existente é `bellatrix_mde::Element`:

```rust
pub struct Element {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub annotations: Vec<Annotation>,
    pub implementations: Vec<Implementation>,
    pub content: Option<FileContent>,
}
```

Os tipos especializados são **descritores compostos**: identidade + atributos básicos + atributos específicos. Não é necessário simular herança clássica em Rust. O modelo mantém identidade única; `kind` determina o schema. Referências a propriedades complexas e membros devem preservar identidades, não copiar estruturas em textos na UI.

StarUML confirma a abordagem de metamodelo: `extensions/essential/uml/metamodel.json` define `UMLModelElement` (superclasse de metamodelo com stereotype e visibility); `UMLClassifier` declara coleções `attributes` e `operations`, `isAbstract`, `isFinalSpecialization` e `isLeaf`; `UMLClass` acrescenta `isActive`. `UMLAttribute` e `UMLOperation` têm seus próprios atributos. A implementação em Rust é independente do código StarUML.

### Esquema mínimo por tipo

| Origem | Propriedades editáveis | Editor/observações |
| --- | --- | --- |
| Base `Element` | `name`, `label`, `description` | Texto; documentação multiline; `id` e `kind` só leitura |
| Classifier | `stereotype`, `visibility`, `isAbstract`, `isLeaf` | Texto/referência, enum, booleans |
| Class | `isFinalSpecialization`, `isActive` | Booleans específicos de Class |
| Attribute | `declaration`, `type`, `defaultValue`, `visibility`, `isStatic`, `isReadOnly` | Propriedades específicas do atributo |
| Operation | `signature`, `visibility`, `isStatic`, `isAbstract`, `isQuery` | Propriedades específicas da operação |
| Extensões | `Element.properties` adicionais | Mostrar valores primitivos; objetos/listas exigem editor especializado |
| Membros | `attributes`, `operations` | Coleções de **Element IDs** ligadas por `Relation.kind = "owns"` |

Os defaults tipados não alteram o documento ao abrir o Inspector. Editar um campo grava a propriedade explícita no estado persistente. O tipo é dinâmico/aberto a novas engines, mas **nenhuma propriedade pode ser editada sem schema e validação**.

### Projeto de API já publicado

- `crates/bellatrix_mde/src/view-model/inspector.rs`: `InspectorProperty`, `PropertyEditor`, `ElementInspector`, `inspect_element(section, id)`.
- `crates/bellatrix_mde/src/KERNEL/inspector_edit.rs`: `set_inspected_property(section, id, path, value)`; os contratos compartilhados de edição ficam em `KERNEL/property_edit.rs`.
- `crates/bellatrix_mde_cordis/src/set_property.rs`: `ModelingSetPropertyRequested`, evento `bellatrix.mde.set-property.v1`, validação e `ProjectReplace` com revisão esperada.
- `crates/bellatrix_mde_workbench/src/property_editor.rs`: Zed GPUI consome os descritores e escolhe controles; a edição usa `MdeUiBridge` e não modifica `ModelingSection` diretamente.

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

Se o tipo da propriedade for booleano, salvar `true/false`; não escrever `"true"` em string. Se for enum, validar opções. Proibir alterações genéricas de `id`, `kind` ou relações por JSON path. Ações de criar/deletar membros são comandos de domínio separados. Operações da classe são elementos relacionados (ex.: `rust.function`); clicar no membro abre seu Inspector.

**Espaço-temporal:** Zed controla a UI, Dock, foco e estado efêmero. Cordis controla composição e eventos de backend; a UI não inventa barramento. Fechar a janela ou descarregar uma engine não remove documentos persistidos.

## 4. UX — mínimo reproduzível

A UI acompanha o tema ativo do Zed. A árvore do Model Explorer contém somente objetos do domínio; a árvore de arquivos do Zed permanece no Project Panel. O Inspector ocupa o **Property Editor**, separado do botão de salvar projeto. Ao mudar a seleção, o esquema inteiro muda; textos em edição não são substituídos a cada render.

O resultado visual mínimo deve reproduzir o comportamento observado nas referências: selecionar `Class1` na árvore ou no Canvas mostra, no painel de propriedades, `name`, `stereotype`, `visibility`, `isAbstract`, `isFinalSpecialization`, `isLeaf`, `isActive`, `Documentation`. Selecionar Attribute/Operation muda a especialização. A classe pode mostrar Attributes/Operations e permitir navegar a seus IDs.

### Critérios de aceitação verificáveis

1. Class → propriedades comuns + especializadas corretas, incluindo visibilidade como enum e flags como booleans.
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

| Reference capability | Current implementation and remaining work |
| --- | --- |
| Font family and size | Live per-view inputs, rendered through shared appearance; theme inheritance when empty. Native font picker/presets remain pending. |
| Fill, line, text color; inherit | Compact icon-first inputs and swatches, immediate receipts and theme reset. Full native color-picker popover remains pending. |
| Outline width and opacity | Shared persisted style, existing renderer projection, compact live inputs. |
| Horizontal and vertical text alignment | Six icon controls, persisted optional enums, shared classifier/figure/relationship-label projection. Model JSON/protobuf and receipt-state tests cover values; native visual acceptance remains pending. |
| Send to back / bring to front | View-local stack command; also forward/backward by one layer. One sorted canvas child list interleaves edges and nodes so paint and hit-test stacking agree. Core view-local/semantic-isolation/roundtrip tests added; native pointer acceptance remains pending. |
| Straight/oblique, rectilinear, rounded rectilinear, curve | Straight, automatic orthogonal, and curve already routed by the model and now icon controls. Rounded orthogonal corners remain pending. |
| Solid, dotted, dashed strokes | Optional common `LineMode`, live icon controls and explicit inherit action; core accepts per-view patterns without modifying relationships. View-local persistence flows pass. Shared stroke resolution and rectangular-outline helper project patterns to relationship/figure paths and classifier/action/state/package/port/part/frame outlines. Native rendering/pointer acceptance remains pending. |
| Flat/stick/solid head and tail | View-only overrides pending. Default UML-required relationship markers must remain intact; changing a visual marker must never mutate relation kind. |
| Auto resize | Pending shared view-format command and renderer-measured bounds. |
| Show shadow and word wrap | Common optional Style flags with live icon controls and shared native surface/text helpers. Shadow applies to the view frame; SVG exports a drop-shadow filter. Native wrapping uses renderer text layout; SVG word wrapping is not implemented and is documented as an export limitation. |
| Stereotype display (none/label/decoration/icon variants) | Common inherited/label/hidden modes implemented for applied labels; required UML keywords remain visible. Decoration/icon variants remain pending actual stereotype image mapping. |
| Show visibility, namespace, property, type | Visibility/type/property-annotation flags share the existing typed member/operation formatter in native canvas and SVG; original semantic values remain unchanged. Namespace remains pending its separate header-line projection, rather than being spliced into names. |
| Suppress attributes, operations, receptions, literals, columns | Common attribute/operation/reception flags filter existing classifier compartments without changing members or view bounds. Literal/column compartments are not yet projected and their controls remain pending. |
| Align selected objects left/center/right/top/middle/bottom | Pending multi-view selection and geometry transaction. Text alignment is distinct and does not implement these operations. |
| Equal horizontal/vertical spacing | Pending multi-view geometry transaction. |
| Equal width/height/size | Pending multi-view geometry transaction. |

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
