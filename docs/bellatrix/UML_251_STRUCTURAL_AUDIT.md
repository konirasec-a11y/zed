# Auditoria de cobertura UML estrutural existente

## Resultado

O marco de edição dos sete diagramas estruturais **ainda não está concluído**. Há infraestrutura comum real, edição de Classes e um início de Componentes/estrutura interna. Criar sete documentos com nomes UMLDI não equivale a criar, editar, validar e representar os elementos definidores dos sete tipos de diagrama.

Base inicial auditada: `fbf6a69dd20d6a54735d49beb1caafcd3bd97edd`.
Estado atualizado e publicado: `77c3709e51032f8832402ea1302ab401c773cca4`,
árvore `eec21f1c654040b60f6e9fb66c82da2c0347f7fa`, verificada por fetch.
A árvore é idêntica ao checkpoint local `aae25c9` observado durante a auditoria.
As lacunas abaixo permanecem após as correções de edição reativa, painéis e
notação de membros. Linhas referem-se a essa árvore, salvo indicação.
A auditoria foi de leitura; não constitui execução de interação nativa.

Método: leitura de implementação ativa, fluxo de comandos e renderização; confronto com o texto completo do PDF oficial UML 2.5.1 (`UML-2.5.1.txt`) e os XMI oficiais preservados em `docs/bellatrix/standards/uml/20161101`. Nenhum comportamento nativo Windows/GPUI foi executado. Falhas de interação abaixo são consequências do código, não relatos de teste manual realizado.

Autoridade: [OMG UML 2.5.1](https://www.omg.org/spec/UML/2.5.1), [PDF normativo](https://www.omg.org/spec/UML/2.5.1/PDF), [UML.xmi](https://www.omg.org/spec/UML/20161101/UML.xmi), [UMLDI.xmi](https://www.omg.org/spec/UML/20161101/UMLDI.xmi). A cláusula 2 distingue conformidade de sintaxe abstrata, sintaxe concreta, intercâmbio de modelo, intercâmbio de diagrama e semântica. O catálogo de nomes/herança sozinho não satisfaz essas categorias. A mesma cláusula estabelece precedência dos arquivos normativos consumíveis por máquina em caso de conflito com o texto.

## Matriz dos sete diagramas

Legenda: **Correto no recorte** = implementação conferida apenas para a regra indicada; **Parcial** = alguma capacidade real, faltando contratos; **Ausente** = não há fluxo correspondente na implementação ativa; **Incorreto** = comportamento representado diverge do contrato normativo indicado. Ausência de uma opção permitida pela norma não foi tratada automaticamente como erro.

| Diagrama | Criação do documento/contexto | Elementos e edição real | Relações e validação | Veredito |
|---|---|---|---|---|
| Class | Correto no recorte: identidade `UMLClassDiagram`, contexto vazio | Criação de Class/Interface e alguns classificadores; membros, nomes, propriedades escalares, movimento e redimensionamento | Generalization, Association binária, Dependency, InterfaceRealization e outros comandos; regras selecionadas. Não é modelo completo de Property/Association, nem validador completo | **Parcial**, com erros de notação de Interface descritos abaixo. Auditoria de fundamentos separada detalha Class/Property/Association/Reception |
| Object | Correto no recorte: `UMLObjectDiagram`, contexto vazio | **Ausente**: criação de InstanceSpecification, classifier(s), Slot, ValueSpecification e notação de objeto sublinhada | **Ausente**: links de instâncias, correspondência Slot/definingFeature e suas regras | **Ausente como editor de objetos**, embora o documento vazio exista |
| Package | Correto no recorte: `UMLPackageDiagram`, contexto vazio | Package e Model criáveis; folder/keyword Model corretos no recorte. Ownership explícito está disponível no comando genérico, mas o gesto normal só solicita owner para Component/Artifact | Falta PackageImport/ElementImport/PackageMerge; ferramenta Containment não altera ownership; verificação public/private só segue `owns` | **Parcial**, com **inconsistência de Containment** |
| Component | Correto no recorte: `UMLComponentDiagram`, contexto vazio | Component criável com Package/Model owner explícito; defaults e retângulo «component» corretos no recorte. Propriedades/membros herdados, interfaces, Port/Part por Quick Edit | InterfaceRealization/Usage existem; não há ComponentRealization, Connector/ConnectorEnd assembly/delegation ou Manifestation; validações específicas incompletas | **Parcial**, não um editor completo de Componentes |
| Composite Structure | Correto para contexto StructuredClassifier; contexto InstanceSpecification emite warning de suporte ausente | Port/Part com `owns`, posicionamento dentro/no limite e edição escalar existem; Collaboration apenas reconhecida, sem notação suportada | **Ausente** Connector/ConnectorEnd; composição dos Port/Part não é materializada/validada; não há compartimento próprio de internal structure no renderer | **Parcial**, com contratos centrais faltando |
| Deployment | Correto no recorte: `UMLDeploymentDiagram`, contexto vazio | Artifact criável, retângulo «artifact» correto; Node/Device/ExecutionEnvironment reconhecidos semanticamente, mas não criáveis na paleta nem renderizados como Nodes | **Ausente** Deployment, Manifestation, CommunicationPath e DeploymentSpecification como contratos ativos | **Ausente como fluxo de Deployment**, com apenas Artifact reutilizável |
| Profile | Correto no recorte: `UMLProfileDiagram`, contexto vazio | Profile e Stereotype possuem mapeamento/estratégia de desenho, mas **não criação de paleta/factory**; se não existir uma View anterior, PlaceElementView também não encontra factory | **Ausente** Extension/ExtensionEnd/ProfileApplication/metaclassReference/metamodelReference e instâncias de estereótipos; somente regra de Generalization entre Stereotypes existe | **Parcial como renderer de dados pré-existentes; ausente como editor de Profiles** |

### Evidências transversais da matriz

- Sete identidades: `crates/bellatrix_mde/src/KERNEL/uml/validation/structural_diagrams.rs:9–43`; criação de diagramas vazios `commands/diagrams.rs:6–36`; menu real `crates/bellatrix_mde_workbench/src/UI/panels.rs:321–405`. O menu de diagramas não muda o conjunto de ferramentas para implementar sete editores especializados.
- Contexto: `structural_diagrams.rs:165–207`. Conferido com UML 2.5.1 §§B.7.6.3, B.7.10.3, B.7.11.3, B.7.12.3, B.7.26.3, B.7.27.3, B.7.28.3. XMI: `UMLDI.xmi:101–114,182–233,247–290`. O `modelElement` do diagrama é contexto, não a lista de elementos desenhados. Correto não impor um contexto Package ao Class diagram.
- A taxonomia do Anexo A, pp.683–685, permite misturar tipos de elementos. A ausência de filtros rígidos por diagrama **não é, por si, erro UML**.
- Factory/paleta real: `commands/kinds.rs:4–39,60–79`, somente 16 variantes. Não contém InstanceSpecification, Slot, Node, Device, ExecutionEnvironment, DeploymentSpecification, Profile, Stereotype ou Collaboration. Port/Part são Quick Edit, não itens dessa enumeração.
- Tipos de relacionamento criáveis: `relationships.rs:7–57`. Não contém Connector, ComponentRealization, PackageImport, ElementImport, PackageMerge, Deployment, Manifestation, CommunicationPath, Extension ou ProfileApplication.
- Mapeamento semântico não é renderer: `uml_types.rs:72–143`; notações reais `uml_notation.rs:18–42`. Node, Device, ExecutionEnvironment, Collaboration e InstanceSpecification terminam em Unsupported. A UI exibe explicitamente “Unsupported notation” em `bellatrix_mde_workbench/src/UI/workbench.rs:1707–1722`.
- Mesmo uma notação registrada não garante colocação: `commands/views.rs:45–77` copia uma View existente, mas, na ausência dela, exige uma factory de `ModelingElementKind`; Profile/Stereotype não possuem tal factory.
- Inspector real: `inspector.rs:69–113,182–216`. Há schemas limitados e edição de escalares existentes. Collections/objetos desconhecidos ficam read-only. Um saco JSON não fornece os seletores de referências, associação inversa, ownership, cardinalidades e validação que faltam.

## Problemas concretos prioritários

### 1. Containment desenha membership, mas não cria membership

**Norma:** §12.2.4, p.248, usa linha com círculo/mais para mostrar membros do Package; §12.4.5.6 define `Package::packagedElement`. XMI `Package-packagedElement`, `UML.xmi:8648`.

**Código:** `relationships.rs:56,75,79–86` cria o tipo `uml.containment`; `commands.rs:395–405` persiste apenas essa relação. Hierarquia semântica só considera `owns` em `hierarchy.rs:24–26`; unicidade/ciclos de ownership só consideram `owns` em `graph.rs:59–66,94–96`; validação de visibilidade de Package só segue `owns` em `structural_rules.rs:94–108`.

**Efeito:** desenhar a relação de Containment entre Package e Class não coloca a Class naquele Package na hierarquia e não lhe aplica as regras de packagedElement. A mesma figura pode contradizer o modelo semântico. Trata-se de uma lacuna de mapeamento concreta, não apenas ausência de um ícone. É preciso distinguir relação gráfica editorial de projeção de ownership real.

### 2. Interfaces fornecidas/requeridas usam uma notação híbrida incorreta

**Norma:** §10.4.4, p.172–173: retângulo «interface» com as dependency arrows apropriadas é uma opção; ball/lollipop fornecido e socket requerido são outra, ligados ao Classifier por linha sólida. §11.6.4 reaproveita essas opções em Componentes.

**Código:** `uml_notation.rs:32` força toda Interface a `InterfaceBall`; `bellatrix_ui_uml/src/interface.rs:78–88` sempre desenha círculo completo, sem socket. Quick-create usa InterfaceRealization para provided e Usage para required (`relationships.rs:754–762`). O desenho dessas relações continua tracejado com triângulo/seta (`relationships.rs:565–569`). `interface.rs:130–164` ainda coloca compartimentos de membros abaixo do círculo, em vez de oferecer o retângulo expandido.

**Efeito:** “Add Required Interface” cria uma bola com Usage tracejado, não um socket com linha sólida; “Provided Interface” combina bola com seta tracejada de realização. Falta uma estratégia coerente por View: retângulo expandido ou notação simplificada do relacionamento.

### 3. Port/Part não materializam a composição obrigatória

**Norma:** §11.8.14.7 `port_aggregation` exige aggregation=composite; §11.2.3.2 e §§11.8.15.5–6 definem parts como ownedAttributes com `isComposite=true`. `isComposite` deriva de aggregation=composite. XMI: `Port-port_aggregation` (`UML.xmi:4764–4774`), `StructuredClassifier-part.1-spec` (`5029–5037`), `Property-isComposite.1-spec` (`17562–17570`).

**Código:** `commands/features.rs:388–429` cria `uml.port`/`uml.part`, ownership e apenas visibility, isService=false para Port, type vazio para Part. `uml_types.rs:134–135` mapeia para Property/Port. Não há materialização/derivação de aggregation composta no adapter nem regra correspondente em `structural_rules.rs:27–140`.

**Efeito:** o rótulo “Part” e o quadrado “Port” existem, mas a propriedade normativa que os caracteriza não está representada/validada. Se os defaults UML forem aplicados a esse Property, aggregation padrão none não satisfaz o contrato. Sem um mapeamento documentado que implique composite pelo kind, não é correto declarar os objetos conformantes.

Nota: `isService=false` não é valor proibido. Difere do default true de §11.8.14.4/XMI `Port-isService`, mas pode ser política do editor. Não deve ser apresentado como violação UML isolada.

### 4. Estrutura interna não tem Connectors nem o compartimento próprio

**Norma:** §11.2.3.3 distingue Connector entre roles de Association entre tipos; §§11.2.4 e 11.3.4 definem sua notação. §11.2.4 chama o compartimento de internal structure de obrigatório para ferramentas com conformidade de sintaxe concreta. XMI `Connector`/`ConnectorEnd`, `UML.xmi:4490,4598`.

**Código:** relacionamentos reais limitam-se à enumeração `relationships.rs:7–57`; `uml.connector` tampouco tem adapter e relações UML desconhecidas são erro `UML_UNKNOWN_RELATIONSHIP` (`structural_diagrams.rs:348–355`). Part/Port são Views independentes posicionadas em coordenadas no card (`commands/features.rs:432–463`). O renderer do classifier cria apenas atributos, operações e receptions (`bellatrix_ui_uml/src/class.rs:536–563`).

**Efeito:** é possível colocar pequenas caixas e Ports, mas não ligar roles com Connector/ConnectorEnd, resolver partWithPort, assembly/delegation ou validar compatibilidade. Association não é substituto semântico de Connector. Part type é editável, mas o renderer do Part recebe somente `element.name` (`workbench/src/lib.rs:1662–1668`), sem projeção própria de roles.

## Lacunas normativas específicas a fechar

- **Object:** §§9.8.3–4 e 9.9.9: InstanceSpecification pode ter zero ou vários classifiers, slots e specification; link é InstanceSpecification classificado por Association. Não inventar um metaclass separado “Object” ou modelar todo link como Association entre tipos. XMI `InstanceSpecification`/`Slot` em `UML.xmi:16425,17954`.
- **Package:** §7.4.3.3/§7.4.4.3 para imports, §12.2.3.2/§12.2.4/§12.4.6 para merges. Dependency genérico com um texto não implementa a semântica de importação/merge. XMI `ElementImport:12683`, `PackageImport:13663`, `PackageMerge:8871`.
- **Component:** §§11.6.3–4, §§11.8.6–7 para Component/ComponentRealization. Implementar ownership e realizingClassifiers/abstraction, derivação de required/provided e conexões internas. O retângulo «component» já é opção normativa válida; não é obrigatório acrescentar o pequeno ícone se o keyword existe. `commands.rs:281–295,331–362` já implementa o recorte com owner Package/Model e defaults corretos. Restringir owner a Package/Model é capacidade limitada do editor, não regra que proíba outros owners válidos em UML.
- **Deployment:** §§19.2.3–4, 19.3.3–4, 19.4.3–4 e 19.5: tipos/instâncias de Nodes/Artifacts, Deployment e Manifestation, CommunicationPath e DeploymentSpecification. XMI `Deployment:12062`, `Device:12231`, `ExecutionEnvironment:12238`, `Manifestation:12246`, `Node:12262`. O retângulo «artifact» atual é permitido por §19.3.4; o problema é ausência dos demais contratos, não falta de ícone de documento.
- **Profile:** §§12.3.3.3–4/6–7, §12.3.4 e §§12.4.1–2/7–9: referências de metamodelo, Extensions/ExtensionEnds, aplicar/remover Profile, criar instâncias de Stereotype e validar tagged values. XMI `Extension:8322`, `ExtensionEnd:8432`, `ProfileApplication:8963`. `properties.stereotype` é texto para exibição (`uml_notation.rs:45–50,76–99`), não aplicação semântica. Profile-tab e Stereotype-rectangle são notações permitidas, mas não bastam para esse fluxo.

## Catálogo, Frame e persistência: não confundir com cobertura

1. **Catálogo de 242 metaclasses:** `uml/metamodel.rs:1–8,504–520` informa nomes, herança e disponibilidade de propriedades; explicitamente não avalia OCL/constraints. Gerador `script/bellatrix_mde/bellatrix_uml_catalog.py:2,19–28`. Isso é infraestrutura correta, não 242 metaclasses editáveis.
2. **Galeria de símbolos:** contém Connector/Node/Device/Deployment/Manifestation/InstanceSpecification/Slot/Profile/Extension (`bellatrix_mde_ui/src/assets/symbols/uml.rs:415–544`). `workbench/src/symbol_library.rs:1–3,58–60` é somente preview de SymbolGallery. A paleta operacional usa outra enumeração. Mostrar uma amostra de ícone não implementa criação, propriedades ou validação.
3. **Frame:** `commands/kinds.rs:13,71` cria `uml.frame` como Element; `uml_types.rs:142` corretamente não o mapeia a metaclass UML. A figura existe em `ui_uml/src/figures.rs:343–387`. O Frame normativo é propriedade/apresentação de UMLDiagram, com heading, conforme Anexo A e §§B.2.2/B.7.13.4–6, não uma metaclass semântica “Frame”. O `Diagram` atual não tem isFrame/heading (`model/diagram.rs:4–19`). A figura editorial extra é permitida pela cláusula 2, mas deve ser identificada como anotação e não contar como suporte ao Frame UMLDI.
4. **Persistência nativa:** `model/io.rs:4–10,30–51` só implementa JSON/BLX1-Protobuf. `Element`/`Relation` são kind+property-map/from/to (`model/graph.rs:7–23,54–60`); `View` tem subject opcional e geometria própria (`model/diagram.rs:40–74`). A separação ID de modelo/ID de View e as sete identidades são avanços corretos no recorte, mas não constituem import/export UML XMI ou UMLDI. Cláusula 2 e Anexo E exigem intercâmbio correspondente para essas alegações de conformidade.
5. **Validação:** `structural_diagrams.rs:260–372` + `structural_rules.rs:27–140` verificam recortes conhecidos. Um resultado sem erros significa somente ausência dos erros implementados. Component/Deployment/Profile/Port/Connector não ganham todas as suas regras por constarem no catálogo. Não é necessário implementar um motor OCL para cumprir regras, mas é necessário implementá-las de alguma forma.

## Saída mínima sugerida para o marco estrutural

Para concluir primeiro o marco estrutural, sem reduzir o escopo UML solicitado, fechar cenários verificáveis de ponta a ponta para os sete tipos: criar os elementos próprios, editar referências tipadas e ownership, criar relações próprias, salvar/reabrir preservando semântica e múltiplas Views, e validar casos positivos e negativos vinculados às cláusulas acima. Primeiro corrigir a base comum (ownership/Containment, Property/Association e notações Interface/Port/Part), depois completar Component/Composite, Object, Package, Deployment e Profile. Validar no Windows a interação de cada fluxo; checks estáticos/headless não substituem isso. Declarar separadamente que XMI/UMLDI interchange não foi implementado, se continuar fora do marco.

## Atualização focada: Port::port_aggregation — 2026-11-03

A lacuna de `Port-port_aggregation` identificada acima foi tratada sobre a base
`1fb60ef`, sem ampliar a UI ou declarar suporte completo a Port/CompositeStructure.
O validador compartilhado aplica a regra de UML 2.5.1 §11.8.14.7 usando o valor
escalar autorado ou o default herdado do metamodelo gerado: `composite` satisfaz
a regra; `none`, `shared` e ausência (default `none`) recebem
`UML_PORT_AGGREGATION`. Valor explicitamente malformado continua sendo erro do
contrato escalar e não é substituído por default; a avaliação da regra fica
não verificada quando a leitura do valor não é possível.

QuickEdit Port e criação direta Port passam a materializar `aggregation=composite`; Part já fazia isso.
A criação direta preserva as precondições existentes: owner compatível e sua
View presente no diagrama. `CreateElementOrVisualDraft` cria Port semântico
quando essas precondições são satisfeitas; owner incompatível continua
rejeitado e o fallback mantém apenas uma View draft com diagnóstico.
A correção não altera o default normativo global de Property, não reescreve
rascunhos importados e não relaxa as outras regras UML. A API funcional existente
`validate_uml_model` expõe os diagnósticos sem um novo formato ou tipo público.
Testes focados: `tests/port_aggregation.rs` (3 casos, incluindo repetição A-B-A e
concorrência) e `created_port_is_composite_and_noncomposite_edits_are_atomic`.
Continuam fora deste fechamento as outras restrições de Port, suas interfaces
provided/required, Connector/ConnectorEnd, notação e fluxo GUI completo.

## Atualização focada: Port::default_value — 2026-11-03

O validador compartilhado implementa `Port-default_value` (UML 2.5.1
§11.8.14.7, XMI `type.oclIsKindOf(Interface) implies defaultValue->isEmpty()`).
Port com tipo Interface e defaultValue local tipado/canonicamente owned recebe
`UML_PORT_DEFAULT_VALUE`. Default ausente/nulo satisfaz o consequente; tipos
conhecidos não-Interface não recebem essa proibição. Texto legado, referências
externas e papéis de ValueSpecification não resolvidos ficam não verificados,
sem inferir tipos por nome ou confundir lowerValue/upperValue com defaultValue.
Referências locais inválidas continuam recebendo os erros de contrato existentes.

`tests/port_default_value.rs` cobre cenários positivos/negativos, referências
não resolvidas, distinção de bounds, preservação JSON/Protobuf de drafts,
rejeição atômica de edição e recuperação. Não há mudança de API pública ou UI;
esta regra não estabelece conformidade completa de Port/CompositeStructure.
A edição legada de `defaultValue` por `SetProperty` ainda aceita somente texto e
rejeita objetos `$ref` antes da validação semântica (`expected string`). O teste
registra essa rejeição sem mutação; o rollback normativo é exercitado pela troca
do tipo de um Port com default existente de Class para Interface. Este fechamento
não resolve a autoria de referências defaultValue pelo editor legado.

## Atualização focada: Port::encapsulated_owner — 2026-11-03

A validação compartilhada agora executa `Port-encapsulated_owner`
(UML.xmi:4786–4796), usando a derivação de `EncapsulatedClassifier::ownedPort`
(XMI:4736–4744): ownedAttributes cujo tipo de metaclasse é Port. O proprietário
semântico canônico precisa coincidir com essa projeção; posição visual e
`encapsulatedClassifier` autorado não substituem `owns`. Herança de metaclasses
permite Class/Component/Node/Device/ExecutionEnvironment sem uma lista especial
por tipo. Owner não encapsulado gera violação; owner ou papel sem resolução
permanece não verificado. Port sem owner não viola artificialmente a igualdade
null=null, mas continua inválido pela regra independente Element::mustBeOwned.

Código em `semantics/classifiers.rs`, com três fluxos em
`tests/port_ownership.rs`: API funcional, estados positivos/negativos,
referências de papel não resolvidas, tentativa de forjar a inversa, reabertura
JSON/Protobuf, rejeição atômica da mudança de owner e reparo de draft. O teste
atômico usa Interface com Feature pública para exigir rejeição pela regra de
Port, sem confundi-la com erro genérico de ownership ou visibilidade.

Gate focado: ownership **3/3**, aggregation **3/3**, defaultValue **6/6** e
API **4/4**; log local `/tmp/port-owner-final.log`. Sem mudança de UI ou geração.
Esta regra não conclui os demais contratos de Port, interfaces e Connectors.

Revisão da prova de membership: o helper compartilhado de papéis compostos
agora distingue inclusão local comprovada de resolução integral da coleção.
`ownedAttribute` com Port local e `href`, em ambas ordens, comprova o papel sem
apagar o warning da referência externa. Coleções malformadas, null proibido e
referências duplicadas continuam inválidos, mesmo com um membro local válido.
O fluxo adicional de Port inclui essas combinações e reabertura.

Gate combinado após a revisão de membership: **33/33** — Port ownership 4,
aggregation 3, defaultValue 6, API 4, ownership/namespace 4, inherited capabilities
5 e Component constraints 7. Log `/tmp/structural-owner-component-gate.log`;
`git diff --check` passou. Esses números correspondem ao estado testado antes
das ampliações posteriores de Component, não substituem o gate final por SHA.

## Atualização focada: AssociationClass::cannot_be_defined — 2026-11-03

A regra `AssociationClass-cannot_be_defined` (UML.xmi:3927–3938) agora verifica
que nem os endTypes nem os allParents desses tipos contêm a própria
AssociationClass. Reutiliza contratos de memberEnd/Type e consultas canônicas
de herança. Uma violação local comprovada prevalece mesmo ao lado de end ou
Type externo não resolvido; referências malformadas continuam inválidas.
Não retorna coleções parciais como se fossem completas.

A regra integra a validação global e é executada antes do fallback de suporte
especializado no gate de associações. O aviso agregado de contratos restantes
de AssociationClass foi preservado: a alteração não declara autoria completa
nem remove bloqueios preexistentes a edições que dependem desses contratos.
O fluxo comprova rejeição atômica de self-end e reparo por draft/reabertura;
uma edição válida ainda bloqueada pelo fallback não foi contabilizada como
comando de autoria aprovado.

Código em `semantics/associations.rs`; três fluxos em
`tests/association_class_constraints.rs` cobrem referência própria, ancestral
direto/transitivo, estados inconclusivos, violação com externos em ambas
ordens, malformados, API e reabertura JSON/Protobuf. Gate focado **11/11**:
AssociationClass 3, association kernel 2, specialization 2 e API 4.
Log local `/tmp/association-class-final.log`; sem alteração de UI ou geração.

Revisão adversarial: a prova compartilhada `has_ancestor` preserva um caminho
canônico conhecido quando outro ramo contém declarações externas sem resolução;
`resolve_ancestry` continua inconclusiva nesses casos e nunca fornece fechamento
parcial. Ausência de caminho com ramo desconhecido permanece não verificada;
ciclos/entradas inválidas prevalecem. A validação global emite a regra específica
uma única vez, mantendo sua execução no gate de comando e as outras verificações
de Association. Teste adicional cobre ordens dos ramos, contagem exata, caminho
removido e ciclo. Gate de revisão **33/33**: AssociationClass 4, association kernel
2, specialization 2, API 4 e templates 21. Log `/tmp/association-class-review.log`.

## Atualização focada: Connector::types — 2026-11-03

O Kernel agora aplica `Connector-types` conforme UML.xmi: com `type` local
resolvido, Connector.end e Association.memberEnd precisam ter a mesma quantidade
e cada `end.role.type` deve conformar ao tipo da ponta correspondente, preservando
a ordem das duas coleções. `type` ausente/null torna esta implicação satisfeita;
isso não elimina os contratos independentes de ends, roles ou ownership.
O estado canônico usa Connector/ConnectorEnd como objetos, referências tipadas e
`owns`. Ausência da coleção ordenada não inventa ordem a partir da apresentação.

Reutiliza contratos de propriedades, ReferenceIndex e consultas de herança;
referências externas permanecem não verificadas. Comparações/posições conhecidas
podem provar violação ao lado de desconhecidos. Ambos os lados de cada comparação
são examinados para não esconder Tipo malformado atrás de referência externa.
Não retorna coleções parciais como completas nem altera o helper de hierarquia.

Código em `semantics/connectors.rs`, registro em `uml.rs`/`root.rs` e cinco
fluxos em `tests/connector_types.rs`: ordem/cardinalidade, identidade e herança,
type opcional, ownership, tipos errados, resultados inconclusivos, rejeição
atômica, referências mistas nas duas ordens e reabertura JSON/Protobuf.
A ausência de end ou memberEnd não esconde um contrato explicitamente inválido
na coleção oposta: ambas são avaliadas antes de combinar os resultados.
Casos simétricos cobrem null, escalar, membro malformado, duplicação e href.
Gate focado **11/11**: Connector 5, association kernel 2 e API 4.
Log local `/tmp/connector-types-symmetric-review.log`.

Permanecem fora deste incremento `Connector-roles`/`StructuredClassifier::allRoles`,
as demais invariantes de ConnectorEnd, derivação de kind, adapter binário de
Connector e criação/edição visual. Esta regra não significa suporte completo a
Composite Structure ou Connectors.

## Atualização focada: CommunicationPath::association_ends — 2026-11-03

A regra `CommunicationPath-association_ends` (UML.xmi:12038–12050) verifica
`endType->forAll(oclIsKindOf(DeploymentTarget))` sobre os memberEnd/Types
canônicos, sem usar cache de endType ou endpoints visuais como prova.
Referências malformadas prevalecem sobre violações conhecidas, que prevalecem
sobre desconhecidos; a consulta completa `end_types` não retorna conjuntos
parciais. Types ausentes não contribuem ao conjunto derivado, mas continuam
rejeitados pela regra herdada `Association::ends_must_be_typed`.

CommunicationPath declara uma única constraint própria e nenhuma propriedade
ou operação própria no XMI. Seu fallback genérico de subtipo foi retirado após
as verificações herdadas; outros subtipos mantêm seus limites. O aviso legado
`MDE_UML_METACLASS_UNSUPPORTED` permanece na camada de adapters: esta entrega
cobre objetos/memberEnd canônicos, sem criação de ferramenta visual. A
classificação dos diagnósticos herdados de Association não foi alterada.

Código em `semantics/associations.rs`; sete BDD/API em
`tests/communication_path.rs` cobrem tipos Node/Device/ExecutionEnvironment,
n-ário, externos/malformados, Unknown/Invalid, JSON/Protobuf e rejeição atômica
com reparo por comando. Gate **15/15**: CommunicationPath 7, AssociationClass 4,
association kernel 2 e specialization 2. Log `/tmp/communication-path-gate.log`.
Sem alteração de UI ou geração.

## Limites normativos remanescentes: ParameterSet — 2026-11-03

A implementação de `same_parameterized_entity` não significa suporte completo
às constraints de ParameterSet. No UML.xmi 20161101, `ParameterSet-input`
(linhas 17138–17153) navega `behavioralFeature`, mas ParameterSet não declara
essa propriedade e seu owner pode ser também Behavior. `two_parameter_sets`
(linhas 17154–17165) quantifica s1 e s2 sem excluir s1 = s2 e compara `s1->size()`
e `s2->size()`, não os tamanhos de seus conjuntos de Parameters. A leitura
literal pode rejeitar o próprio conjunto, contrariando a descrição de dois
conjuntos distintos. Ambas permanecem pendentes de decisão normativa; não se
substituiu silenciosamente o OCL por uma interpretação da prosa.

`Parameter-object_effect` agora valida os contratos de type e effect antes de
avaliar a implicação. Tipo externo/desconhecido com efeito presente permanece
Unknown; efeito ausente prova a consequência, mas não esconde dados Invalid.
DataType, PrimitiveType e Enumeration são reconhecidos por metaclasse, não por
herança de classificadores do usuário. Sete BDD/API cobrem esses casos,
JSON/Protobuf e rejeição atômica/reparo via SetProperty. Gate **28/28** com
ParameterSet (10), ClassBehavior (6) e Reception (5), registrado em
`/tmp/parameter-object-effect-gate.log`. Sem UI, geração ou novo suporte completo
de ParameterSet.
