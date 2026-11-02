# MDE: tipo, capacidades, notação e geometria

Este contrato pertence à engine MDE. Ele não redefine Bellatrix como editor UML nem pretende implementar o futuro runtime público de plugins.

## Referências consultadas

- `crates/bellatrix_design_system/docs/design/src/components/diagrams/SvgNodes.tsx`: compartments e contorno de seleção separado da figura.
- `crates/bellatrix_design_system/docs/ref/staruml/extensions/essential/uml/metamodel.json`: Class e Interface especializam UMLClassifier; Package tem outra linhagem; Constraint tem semântica própria.
- `crates/bellatrix_design_system/docs/ref/staruml/extensions/essential/uml/quickedits/uml.json`: ações compartilhadas e ações específicas por tipo.
- `crates/gpui/src/elements/canvas.rs`, `crates/gpui/src/path_builder.rs` e `crates/gpui/examples/painting.rs`: desenho vetorial nativo no host Zed.

## Responsabilidades

**Tipo semântico.** `UmlType` interpreta os identificadores persistidos sem reescrever projetos existentes. `rust.struct`/`uml.class` e `rust.trait`/`uml.interface` são aliases de domínio já usados pela engine. Mudar o nome de uma figura não muda seu tipo; mudar o desenho não muda sua identidade.

**Capacidades.** `UmlCapability` declara quais operações o tipo aceita. Nota e Constraint não são funções exclusivas de Interface. Attribute e Operation pertencem a classificadores; Reception possui uma restrição adicional. Package pode receber uma anotação, mas não recebe uma Operation por parecer uma caixa na tela. Criar classe realizadora continua específico de um alvo Interface.

**Comando.** `ModelingQuickEdit` recebe `element_id`, ação tipada, diagrama, projeto e revisão. O mesmo executor realiza as ações comuns. Subtipo/supertipo preservam o tipo do classificador selecionado e a direção da generalização. O evento antigo `ModelingInterfaceQuickEdit` fica apenas como adaptador de compatibilidade; não contém outra implementação das mutações.

**Notação.** `UmlClassifier` compõe título, compartments, seleção e edição. A notação retangular e a notação circular usam o mesmo renderer de membros. `UmlClassCard` é apenas um alias público temporário. `UmlFigure` apresenta as notações não classificadoras. Note e Constraint usam o mesmo contorno de folha, sem se tornarem o mesmo objeto semântico.

**View.** Largura, altura, posição e zoom são a entrada da projeção. `interface_layout` é uma função pura, usada tanto pelo desenho quanto pelo roteamento. O raio não é mais uma constante de 14px independente da View. Um círculo permanece circular, limitado pela dimensão disponível e pelo espaço reservado ao nome e aos membros. O redimensionamento não remove membros nem muda seu tipo. O mínimo intrínseco é resolvido igualmente para documentos antigos e novos.

**Canvas.** Posiciona a View e encaminha gestos; não acrescenta um preenchimento retangular por fora da notação. A caixa de seleção não é o símbolo. A folha com dobra é desenhada com PathBuilder nos limites medidos da View, não por um ícone SVG quadrado esticado atrás de um rótulo.

## Invariantes testáveis

1. Class e Interface reutilizam as mesmas definições de ações comuns e o mesmo executor.
2. Uma ação inválida para o tipo não modifica o candidato nem a revisão persistida.
3. Redimensionar Interface altera o círculo e suas ancoragens pela mesma função geométrica.
4. O ponto de conexão pertence à circunferência; não é a quina da caixa de seleção.
5. A folha, a dobra e o texto ficam dentro dos limites da View.
6. Criar um novo tipo relacionado persiste nó, relação e Views em uma única revisão.
7. Descarregar o plugin remove o responder; não remove o documento salvo.

Esses contratos não constituem uma prova formal em teoria das categorias. São fronteiras e invariantes executáveis que evitam confundir a representação visual com o tipo representado. A implementação usa composição explícita em vez de copiar o comportamento de Interface para cada forma.

## Validação local

```powershell
cargo +1.98.1 test -p bellatrix_mde_modeling
cargo +1.98.1 test -p bellatrix_mde_cordis
cargo +1.98.1 check -p bellatrix_ui_uml -p bellatrix_mde_workbench
cargo +1.98.1 run -p zed
```

Comparar antes/depois ao redimensionar uma Interface vazia e outra com membros. Repetir com zoom diferente; criar Note/Constraint a partir de Class, Interface e Package; confirmar os links e a persistência. O código e os testes adicionados ainda exigem compilação e validação visual local.

Ações especializadas ainda pendentes (por exemplo, portas, partes e interfaces requeridas/providas pelo Quick Edit de Class) permanecem indisponíveis; não devem aparecer como botões com implementação fictícia. Nenhum PR ou execução de CI deve ser aberto para substituir essa validação local.
