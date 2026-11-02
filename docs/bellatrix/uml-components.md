# Componentes visuais UML — contrato entre Zed, Engine-MDE e Cordis

**Regra arquitetural obrigatória:** todos os controles visuais específicos de UML devem pertencer a `crates/bellatrix_ui_uml`. O Workbench **não** implementa suas próprias versões de cartões de classe, linhas de atributos, campos inline ou barras de Quick Edit.

Referência de comportamento: arquivos fornecidos pelo mantenedor em `starUml.zip`, especialmente `src/ui/quickedit-manager.js`, `extensions/essential/uml/quickedits/uml.json`, `extensions/essential/uml/metamodel.json` e classes de Views. A implementação é nativa GPUI/Zed e não copia código, CSS ou assets StarUML.

## Responsabilidades

| Módulo | Papel e tipo de estado |
| --- | --- |
| `crates/bellatrix_ui_uml/src/class.rs` | View de Class/Interface, título, compartimentos e alvos de clique para Class, Attribute e Operation; não conhece Cordis |
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
