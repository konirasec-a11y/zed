# Auditoria semântica do núcleo UML e diagrama de classes

## Base e método

- Base pré-atualização inspecionada: `fbf6a69dd20d6a54735d49beb1caafcd3bd97edd` (`HEAD` na inspeção). As referências `arquivo:linha` abaixo são dessa baseline. Durante o fechamento foi publicado `77c3709e51032f8832402ea1302ab401c773cca4`; a comparação dos três crates auditados mostra alterações em `bellatrix_mde/src/{mde,presentation}.rs`, `bellatrix_mde_cordis/src/ui_bridge.rs` e dois arquivos de testes Cordis. O avanço de apresentação é separado abaixo; as estruturas e validadores semânticos citados permanecem iguais. Não confundir o avanço de integração/painéis com correção desses contratos.
- Fontes normativas: PDF oficial **UML 2.5.1**, disponível localmente em `uml-spec-reference/UML-2.5.1.txt`, e `docs/bellatrix/standards/uml/20161101/UML.xmi`. Referência pública: https://www.omg.org/spec/UML/2.5.1/PDF . O XMI foi conferido diretamente para as violações principais, não tratado como prova de implementação.
- Inspeção estática de `bellatrix_mde`, `bellatrix_mde_cordis` e `bellatrix_project`. Nenhum build, teste novo, execução de testes, edição de produção, commit ou publicação foi feito nesta auditoria. Os cenários abaixo são consequências dos caminhos de código inspecionados, não resultados de execução da GUI.
- **Correto no recorte** significa que a regra específica está implementada coerentemente. **Parcial** significa representação/editor/validação incompletos. **Incorreto** identifica um estado ou comportamento concreto contrário à norma. **Ausente** significa sem contrato ativo, mesmo que o nome exista no catálogo/XMI.

## Mapa pelo índice oficial da especificação

- **Cláusula 2, Conformance:** itens 17–18, separando armazenamento, validação e intercâmbio.
- **Capítulo 7, Common Structure:** itens 1–4, 11–12, 14–18. Element, NamedElement, Namespace, MultiplicityElement, Type/TypedElement, Dependency/Realization e templates.
- **Capítulo 8, Values:** itens 5–6 e 11. Default e limites ainda são strings/valores JSON; não há contrato completo de ValueSpecification para esse recorte. Não se declara suporte às demais metaclasses do capítulo.
- **Capítulo 9, Classification:** itens 2, 5–6, 9–13 e 16. Classifier, Property, Operation, Parameter, Generalization e templates de Classifier.
- **Capítulo 10, Simple Classifiers:** itens 7–8 e 15. Interface, InterfaceRealization, Reception e Signal necessário à Reception.
- **Capítulo 11, Structured Classifiers:** itens 3, 5–6, 8–10 e 13. Class, Association, AssociationClass. Port/Part/Connector estão na auditoria estrutural separada.
- **Capítulo 12, Packages:** itens 3–4, 13 e 17. Ownership, Namespaces, visibility dos packagedElements e restrição de Stereotype. Diagramas Package/Profile estão na auditoria estrutural separada.
- **Capítulo 19, Deployments:** Artifact é reconhecido no catálogo e comandos, mas Deployment/Node e os demais contratos desse capítulo são tratados na auditoria estrutural separada; esta auditoria não certifica sua implementação.

## Resultado principal

O projeto tem uma base real de identidade, ownership, criação de membros, herança de metaclasses e algumas validações UML. **Element existe como registro genérico de domínio**, mas isso ainda não implementa todo o contrato da metaclasse UML Element e de suas especializações. Não é necessário usar herança de structs Rust: referências/relacionamentos/adapters podem cumprir o contrato, desde que representem e validem sua semântica. **Ainda não possui um modelo UML 2.5.1 completo nem garante a validade UML dos elementos que o editor já cria.** As lacunas estão também no básico: owner de Class, visibilidade de Interface, Reception, tipagem de Property, Parameters de Operation e memberEnds de Association.

### 1. Identidade e separação entre objeto e apresentação: correto no recorte

- `Element.id`, `kind`, `name` e `label` são distintos; relações e Views referenciam IDs. O grafo rejeita IDs vazios/duplicados e referências inexistentes. Renomear não reescreve IDs. Evidências: `crates/bellatrix_mde/src/model/graph.rs:6–22,53–60`; `src/graph.rs:34–56,78–96`; `src/commands.rs:209–230`; `src/presentation.rs:102–130`.
- Isso é compatível com separar o Element do nome opcional de NamedElement (§7.8.6; §7.8.9.5). A sintaxe particular dos IDs é uma decisão Bellatrix, não uma exigência UML.
- Limite: a identidade normativa de Relationship também é Element (§7.8.15.3), mas o DTO separa `elements` e `relations`; endpoints e `owns` só podem apontar para `elements` (`graph.rs:46–56`). Não há integração geral de relações como membros/ownedElements de Namespaces.

### 2. Herança de metaclasses e capacidades: correto no recorte, não conformidade completa

- O catálogo preserva herança múltipla e transitiva, por exemplo Class → BehavioredClassifier + EncapsulatedClassifier, Interface → Classifier, AssociationClass → Class + Association. Consulta reflexiva/transitiva, com proteção de ciclos e rejeição de nomes desconhecidos: `src/uml/metamodel.rs:30–44,129–130,502–543`.
- O adaptador reconhece Class/Interface, Property para Attribute/Part, Port, Operation e outros tipos; usa propriedades herdadas para habilitar membros, generalização, portas e templates: `src/uml_types.rs:71–192`. Isso corresponde às generalizações de §§9.9.4.3, 9.9.11.3, 9.9.17.3, 10.5.5.3, 11.8.2.3 e 11.8.3.3.
- Não confundir o catálogo com implementação de atributos tipados, defaults, operações derivadas ou OCL. O próprio arquivo declara essa limitação (`metamodel.rs:1–4,502–518`). `uml.template`, `uml.subsystem` e `uml.frame` não recebem metaclasse normativa (`uml_types.rs:142`); os diagnósticos explicitam adapters não suportados (`structural_rules.rs:34–39`).

### 3. Ownership: parcialmente correto; Class/Interface sem owner são aceitas

- **Correto:** uma relação `owns` por filho e ausência de ciclos são verificadas (`src/graph.rs:59–66,93–96`); apagar um owner remove recursivamente seus ownedElements (`src/commands.rs:547–586`). Isso cobre parte de Element::owner e `not_own_self` (§7.8.6.4–6).
- **Incorreto/incompleto:** o comando exige owner apenas para Component/Artifact. Class e Interface podem ser criadas com `owner_id=None`; o registro é persistido sem relação de ownership (`src/commands/kinds.rs:24–25`; `src/commands.rs:281–295,344–362`). O teste existente até explicita o comportamento, sem servir como prova de conformidade (`src/commands/structural_creation_tests.rs:135–152`).
- Norma: `Element::mustBeOwned()` retorna true e `has_owner` exige owner (§7.8.6.5–6; XMI `Element-has_owner`, `Element-mustBeOwned`). Package é a exceção relevante, não Class/Interface. O envelope `Project` não é um `uml.model` e não existe mapeamento implícito de seus filhos para ownership UML (`src/model/project.rs:8–20`; `src/model/section.rs:5–28`).
- Consequência: uma classe comum desenhada no canvas pode parecer válida, mas não constitui um elemento UML completo quanto a ownership. Ownership genérico também não valida todos os tipos de owner/membro; o grafo só restringe um caso Rust/filesystem.

### 4. Nomes e namespaces: parcial, com validação básica ausente

- Há nome canônico e renomeação; membros novos recebem nomes distintos dentro do owner (`src/commands/members.rs:96–113`). Entretanto renomear ou criar novamente um Class com o mesmo nome não avalia distinguibilidade no Namespace (`src/commands.rs:209–230,277–280`; `src/commands/validation.rs:4–29`).
- Norma: `Namespace::members_distinguishable`, §7.8.10.7, usa a distinção de NamedElement (§7.8.9.7). Dois atributos de mesmo nome no mesmo Class podem ser aceitos depois de renomear; sobrecarga de operações não é equivalente a proibir todo nome repetido e dependeria de parâmetros/assinatura semânticos.
- `NamedElement::name` é opcional e `qualifiedName` é derivado (§7.8.9.5–8), mas o DTO exige `name: String`, a UI rejeita nome vazio e não deriva qualifiedName. É uma restrição/incompletude do editor, não regra UML (`model/graph.rs:10`; `inspector.rs:264–266`).
- O comando chamado `SetNameExpression` é um parser de texto de stereotype/visibilidade/nome, não `NamedElement::nameExpression : StringExpression` (§7.8.9.6): `src/commands/name_expression.rs:3–41`.

### 5. Property/Attribute: membro real implementado, contrato de tipo e multiplicidade parcial

- **Correto no recorte:** Add Attribute cria um Element próprio e `owns` para um owner que suporta `ownedAttribute`, em vez de apenas texto de compartimento (`src/commands/members.rs:59–94,135–152`). Attribute/Part são mapeados a Property, não metaclasses inventadas (§9.9.17; `src/uml_types.rs:134`).
- **Parcial:** `type` e `defaultValue` são strings editáveis, não referências verificadas a Type e ValueSpecification. O schema oferece visibility/isStatic/isReadOnly, mas não contrato completo para lowerValue/upperValue, isOrdered/isUnique, redefinition/subsetting, qualifier, association/opposite ou isDerivedUnion (`src/inspector.rs:79–86,95–113,207–216,295–297`). Propriedades extras podem persistir no mapa JSON, sem adquirir semântica por isso.
- Norma: TypedElement::type é referência a Type (§7.8.22.5); Property::defaultValue é ValueSpecification (§9.9.17.6); MultiplicityElement e regras de Property estão em §§7.8.8.5–8 e 9.9.17.8.
- Consequência: escrever `Customer` em Type não liga o atributo ao Element Customer, não garante existência/conformidade e não acompanha renomeação por identidade. Não se pode concluir validade de multiplicidade/derivação/redefinição a partir do campo de texto.

### 6. Operation: identidade e ownership reais; Parameters/retorno semântico ausentes

- Add Operation cria um Element próprio e `owns`, com nome, visibility e flags editáveis isStatic/isAbstract/isQuery (`src/commands/members.rs:85–93,122–152`; `src/inspector.rs:87–93`). Isso cobre uma parte de Operation e Class::ownedOperation (§§9.9.11, 11.8.3.6).
- **Ausente no adapter ativo:** Parameter não existe em `UmlType::from_kind`, nos tipos de membro ou no schema de Operation (`src/uml_types.rs:71–107`; `src/commands/members.rs:6–10`; `src/inspector.rs:87–93,222–250`). O catálogo possuir Parameter não resolve essa ausência.
- Norma: Operation::ownedParameter é coleção ordenada; direction inclui in/out/inout/return; retorno máximo de um Parameter. `/type`, `/lower`, `/upper`, `/isOrdered`, `/isUnique` são derivados do retorno (§§9.9.11.4–7, 9.9.13, 9.9.14). Faltam também contratos de pre/post/bodyCondition e suas restrições.
- Consequência: editar `signature` não cria parâmetros com identidade, tipo, direção ou multiplicidade. Um texto de retorno não é equivalente a um return Parameter. Não se pode verificar sobrecarga, consistência de redefinição nem `at_most_one_return`.
- Na baseline publicada, o label canônico da Operation ainda era apenas nome e tipo opcional, sem `()` (`src/presentation.rs:102–130`). O commit agora publicado `77c3709` corrige esse recorte de apresentação (Operation com `()`, modificadores/default/multiplicidade quando armazenados, sublinhado para Feature estática). Não contar mais o `()` ausente como defeito atual; a alteração não implementa Parameters nem retorno semântico.

### 7. Interface: criação de membros existe, mas aceita visibilidade proibida

- Membros de Interface começam public, o que é correto (`src/commands/members.rs:74–94,123`).
- **Incorreto:** o mesmo Inspector oferece e aceita private/protected/package para Attribute e Operation da Interface, sem verificar o owner (`src/inspector.rs:56,79–93,255–298`). Nenhuma regra equivalente aparece nos validadores ativos.
- Norma: §10.5.5.5 `visibility`: todas as Features de Interface são public; confirmado em `UML.xmi:7961–7973`.
- Consequência: uma alteração normal no Inspector produz uma Interface contrária à norma sem rejeição semântica.

### 8. Reception: comando existente produz representação incompleta e aceita Class passiva

- **Incorreto/incompleto:** Add Reception cria `uml.reception`, nome autogerado e visibility=public, sem Signal ou parâmetros; só consulta se o owner herda `ownedReception` (`src/commands/members.rs:64–73,94,122–152`; `src/uml_types.rs:183`).
- Norma: Reception exige `signal : Signal [1]`, mesmo nome e parâmetros correspondentes aos ownedAttributes do Signal (§10.5.8.4–5; XMI `Reception-signal`, `Reception-same_name_as_signal`, `Reception-same_structure_as_signal`). Class passiva não pode possuir Reception (§11.8.3.8 `passive_class`; XMI `Class-passive_class`).
- Consequência: Add Reception sobre a Class padrão, com isActive ausente/default false, aceita um membro proibido e sem o Signal obrigatório. Não basta tornar a Class active: ainda falta o contrato de Signal/Parameters.

### 9. Association binária: visual e endpoint Types parciais; memberEnds não implementados

- A criação conserva um objeto Relation com duas referências `from`/`to`. O validator exige tipos conformes a Type e permite corretamente associação reflexiva (`src/model/graph.rs:53–60`; `src/structural_rules.rs:60–90`). Isso não deve ser restringido artificialmente a Class.
- Norma: Association também é Classifier/Relationship e exige `memberEnd : Property [2..*]` ordenado; `endType` deriva desses Properties (§11.8.1.3,6–8; §11.5.3.1).
- **Ausente:** identidade dos Properties das pontas, ownedEnd/navigableOwnedEnd, ownership dos ends, opposite, qualifier e relações semânticas de redefinition/subsetting. As strings sourceRole/targetRole/sourceMultiplicity/targetMultiplicity não substituem esse grafo (`src/inspector.rs:140–172`; `src/relationships.rs:656–663`).
- Consequência: uma linha associa visualmente Types, mas não pode representar fielmente uma Property que seja simultaneamente atributo e memberEnd, nem tratar ends n-ários/AssociationClass pelo contrato normal. Uma Association salva como Relation tampouco pode ser endpoint de Generalization, pois endpoints do grafo só aceitam Elements (`graph.rs:46–56`).

### 10. Aggregation/Composition: diamante coerente; `end1Aggregation` NÃO é a Property UML da mesma ponta

- **Correto no recorte:** ferramentas Aggregation/Composition salvam `uml.association`, não metaclasses separadas (`src/relationships.rs:710–718`; `src/commands.rs:394–405`). Instrução é clicar Whole e depois Part; o diamante é colocado na origem/Whole (`src/relationships.rs:64–65,570–571,614–638`).
- **Diferença obrigatória de interpretação:** na UML, aggregation=composite/isComposite pertence ao Property **tipado pela parte**, enquanto o diamante fica na ponta **oposta**, junto ao todo (§11.5.3.1, §11.5.4; §9.9.17.5–7). Para `from=Whole`, `to=Part`, `end1Aggregation=composite` é uma convenção Bellatrix do lado do diamante. Não pode ser exportado como `aggregation=composite` no Property tipado por `from`.
- **Parcial:** só é validado se o valor pertence a none/shared/composite (`src/structural_diagrams.rs:338–346`). Não se avalia `Property::multiplicity_of_composite`, que limita `opposite.upperBound() <= 1` (§9.9.17.8; XMI `Property-multiplicity_of_composite`). O validator de multiplicidade genérica aceita, por exemplo, `sourceMultiplicity="2"` como intervalo válido, sem aplicar a regra da composição.
- **Já divulgado nos docs:** `docs/bellatrix/STRUCTURAL_UML_MILESTONE.md:119,131–136` diz explicitamente que endpoints representam Types e adia multiplicidade do opposite até haver mapeamento documentado da convenção diamond-side para Property. Os docs não apresentam esse mapeamento como concluído. Não afirmar que o diamante está invertido: o problema é semântica persistida/mapeamento, não a posição visual atual.
- Não impor DAG aos Types ligados por composição: a semântica de composição restringe instâncias/partes (§9.5.3), não proíbe arbitrariamente toda associação recursiva entre tipos.

### 11. Multiplicidade de Association: validator literal correto no recorte; execução incompleta

- `*` → 0..*, limites não negativos, upper >= lower e inteiros arbitrariamente grandes são tratados; valores constantes não avaliados produzem warning de não suporte, em vez de serem chamados de UML inválida (`src/structural_rules.rs:143–201`). Regras normativas: §7.8.8.8 `lower_ge_0`, `upper_is_unlimitedNatural`, `upper_ge_lower`, `value_specification_constant`.
- Limites: não há ValueSpecifications reais e a validação só olha strings de duas pontas de Association. Não cobre multiplicidade de Attributes/Parameters/Ports. Além disso, editar uma multiplicidade de uma relação já existente não aplica o gate semântico descrito abaixo.

### 12. Navegabilidade: campo editável não tem efeito correspondente

- **Incorreto como comportamento do editor:** o Inspector expõe `properties.navigable` Boolean em todas as relações (`src/inspector.rs:140–157`) e aceita persistir seu valor, mas a projeção e estilo da relação não o leem (`src/relationships.rs:563–575,614–667`). O tipo `uml.directed_association` sozinho decide a seta.
- Norma: navegabilidade é definida por Property/ownership ou `Association::navigableOwnedEnd`, separada de aggregation e ownership; uma seta aberta indica ponta navegável (§11.5.3.1, §11.5.4; §9.9.17.7 `isNavigable`).
- Consequência: marcar/desmarcar Navigable pode salvar uma alteração sem alterar a seta nem estabelecer o estado UML prometido. Não há representação independente das duas pontas.

### 13. Generalization: regras centrais corretas, cobertura parcial

- **Correto no recorte:** from=specific, to=general; ambos Classifier; metaclasse específica conforma à geral; pais não finais via isFinalSpecialization, não via isLeaf; restrição de Stereotype; ciclos transitivos rejeitados inclusive fora da View (`src/relationships.rs:67,92–97,566`; `src/structural_diagrams.rs:284–327,358–371`; `src/commands/validation.rs:30–46`). Normas: §§9.9.7.1,5; 9.9.4.7–8; 12.4.9.6.
- A direção e triângulo estão coerentes com UML. Múltipla herança não é proibida artificialmente.
- **Parcial:** o DTO guarda a ligação, não o contrato completo de GeneralizationSet/powertype, isSubstitutable, inheritedMember/redefinition e Association specialization. O catalogo de herança de **metaclasses** não é implementação de herança de **membros do modelo**. Inspector/compartimentos percorrem apenas `owns` direto (`src/inspector.rs:231–246`; `src/uml_geometry.rs:34–53`). Suprimir membros herdados no desenho, isoladamente, não é erro de notação.

### 14. Dependency/Usage/Realization: direção correta, tipo dos endpoints não validado

- **Correto no recorte:** from=client/implementation, to=supplier/specification; Dependency é tracejada com seta aberta e Realization é tracejada com triângulo (`src/relationships.rs:66,73–75,565–569,633–636`). Norma: §§7.7.3.4, 7.7.4.
- **Incorreto:** `validate_endpoints` não aplica NamedElement a Dependency, Usage ou Realization (`src/relationships.rs:79–102`), e os validadores semânticos tampouco. Comment é Element, não NamedElement; mesmo assim um comentário existente pode ser usado como client/supplier pelo comando. Norma: Dependency::client e supplier são NamedElement [1..*] (§7.8.4.5; XMI `Dependency-client`/`Dependency-supplier`).
- **Parcial:** um único from/to não cobre conjuntos de clients/suppliers e não há contrato completo de Abstraction::mapping ou package ownership. Não há exigência geral UML de aciclicidade de Dependency; o código acerta ao não inventá-la.

### 15. InterfaceRealization: endpoints corretos; não prova implementação do contrato

- **Correto no recorte:** target deve ser Interface, source deve conformar a BehavioredClassifier; rejeita o erro de aceitar qualquer Classifier (`src/relationships.rs:88–91`; `src/structural_rules.rs:41–58`; `src/structural_diagrams.rs:329–336`). Norma: §10.5.6.4 contract/implementingClassifier.
- **Parcial:** é uma ligação com endpoints, sem verificação de operações/contrato implementado. A fonte é o implementingClassifier e poderia fornecer o owner por adaptação; não há contrato geral de ownership de relações no grafo. Fontes não mapeadas geram warning, não certificado de validade.

### 16. Templates: um recorte semântico real, não somente uma caixa desenhada

- Quick Edit cria TemplateSignature/RedefinableTemplateSignature, ClassifierTemplateParameter e parameteredElement próprio, com links de ownership e referência; evita criar extensão herdada incompleta (`src/commands/features.rs:181–327`).
- Há validação de owner, uma signature por template, parâmetros, referência/tipo/ownership de parameteredElement e regras limitadas de ClassifierTemplateParameter; alterações recebem gate próprio (`src/templates.rs:168–448`). Base: §§7.8.17, 7.8.19, 7.8.20, 9.9.5, 9.9.19.
- **Parcial:** constrained substitution é explicitamente não suportada (`templates.rs:369–375`), herança é recusada (`features.rs:222–242`) e TemplateBinding ainda é uma ligação cujo endpoint deve incluir o alias `uml.template` (`relationships.rs:98–100`), sem formal/actual substitutions. Um Class com assinatura canônica não vira alias `uml.template`; a ferramenta de binding não modela o contrato geral de §7.8.16/18.

### 17. Validação de mutações: correção do grafo não garante validade UML

- **Correto como transação:** comandos clonam o modelo, validam grafo e só então publicam; Cordis verifica expected_revision e usa ProjectReplace (`src/commands.rs:109–112,223–230`; `crates/bellatrix_mde_cordis/src/mutation.rs:16–42`). É engenharia de consistência útil, não conformance UML por si.
- **Lacuna concreta:** `new_relationships` retorna imediatamente se não há relações novas e filtra erros pelos IDs das relações adicionadas (`src/commands/validation.rs:4–29`). `SetProperty` só aplica tipo/enum do Inspector (`src/inspector.rs:255–300`). Templates possuem gate adicional, mas as demais regras estruturais não são aplicadas do mesmo modo às alterações.
- Consequências: uma Association existente aceita multiplicidade `3..2`; um pai já generalizado aceita isFinalSpecialization=true; uma Class empacotada aceita visibility=protected. Os diagnósticos de modelo podem detectar esses estados posteriormente, mas a mutação não os rejeita. As normas violadas são respectivamente §§7.8.8.8, 9.9.4.8 e 12.4.5.8.
- Atenção: permitir rascunhos inválidos pode ser política legítima de editor. Nesse caso a UI deve distingui-los explicitamente de modelos UML validados. O que não procede é chamar o sucesso da mutação/roundtrip de aprovação normativa.

### 18. Persistência: formato nativo real, sem intercâmbio UML completo

- JSON/Protobuf guardam Elements/Relations/Views com propriedades; modelo e documento têm versões, IDs e revisões. `with_model` valida integridade do grafo antes de incorporar a seção (`src/model/io.rs:4–51`; `crates/bellatrix_mde_cordis/src/project_integration.rs:8–25`). O envelope da plataforma deliberadamente trata as seções como JSON opaco (`crates/bellatrix_project/src/document.rs:34–45,89–117`).
- **Parcial/ausente para UML:** `encode/decode` verificam formato/versão, não conformance semântica; não são serializadores de instâncias UML/XMI. Relações com Properties inexistentes, strings de Type e Operation sem Parameters continuam assim depois do roundtrip. Os XMIs oficiais salvos são insumo normativo, não importação/exportação de modelos do usuário.
- Cláusula 2 distingue sintaxe abstrata, sintaxe concreta, intercâmbio de modelos, intercâmbio de diagramas e semântica. Os próprios docs recusam declarar intercâmbio conforme nesta baseline (`docs/bellatrix/MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md:223–238`). Não converter testes de JSON/Protobuf ou catálogo em selo UML.

## Prioridade sugerida de correção, sem executar alterações

1. Garantir ou explicitar estado de rascunho para ownership, Interface visibility, Reception e mudanças que violam regras já implementadas. São falhas acessíveis no fluxo normal de autoria.
2. Introduzir contratos identificáveis de Property/Type/Multiplicity/ValueSpecification e Parameter/Operation, preservando identidade e semântica ao editar.
3. Modelar memberEnd/ownedEnd/navigableOwnedEnd de Association e documentar/migrar corretamente `end1Aggregation`; então validar multiplicidade oposta e demais regras de Property.
4. Fechar namespace/distinguibilidade e endpoints de Dependency/Realization; ampliar templates e herança de membros somente com os contratos correspondentes.
5. Somente depois declarar os níveis específicos de conformidade efetivamente verificados. As alterações publicadas de rótulos e integração de painéis não resolvem as lacunas semânticas acima.

## Avanço posterior: contratos consumidos de Root e multiplicidade

As conclusões acima são a auditoria da base indicada, não uma alegação de que
cada lacuna continua inalterada após as correções. O incremento seguinte liga
contratos Rust de Element, NamedElement, Namespace, Relationship e
DirectedRelationship, além de CommentRef, aos registros canônicos existentes
em `crates/bellatrix_mde/src/KERNEL/uml/semantics/root.rs`. Inspector, hierarquia e comandos usam
esses contratos. O root Model possui referência persistida; owner, namespace e
qualifiedName são derivados, não cópias independentes mantidas pela UI.
A mutação rejeita violações novas do recorte implementado, sem bloquear a
correção de um draft por erros anteriores não relacionados.

`uml/types.rs` fornece Integer de precisão arbitrária, UnlimitedNatural
explícito e intervalo de multiplicidade consumido pela validação e pela
apresentação das pontas atualmente representadas. Os cinco nomes de
PrimitiveTypes são identificados. Isso não implementa todos os primitivos,
ValueSpecifications possuídos, Type references ou memberEnds de Association.

Os fluxos existentes foram ajustados à nova raiz e complementados com um
percurso de criação/edição/ownership/comentário/reabertura, além do fluxo de
multiplicidades. Gates headless e lint passaram; interação nativa dessas
mudanças ainda requer verificação. Imports de Namespace, demais restrições de
Root, propriedades completas de Association e os outros contratos estruturais
continuam pendentes. UMLDI e StandardProfile permanecem obrigações separadas.

## Adendo KB-09: redefinições e Slots (2026-11-03)

**Estado: implementação parcial testada; KB-09 não integralmente aceito.**
Este adendo descreve as alterações locais sobre `4e4eb41`, sem substituir a
baseline histórica acima nem afirmar conformidade integral UML 2.5.1. O
comparador de Property descrito abaixo permanece uma pendência normativa.

### Escopo implementado

- `crates/bellatrix_mde/src/KERNEL/uml/semantics/classifiers.rs` calcula
  `Classifier::allSlottableFeatures` conforme
  `Classifier-allSlottableFeatures-spec` e `Classifier-inherit-spec` do
  [UML.xmi versionado](standards/uml/20161101/UML.xmi). A chamada é
  `self.inherit(self.allParents().attribute)`: somente alvos redefinidos pelos
  membros possuídos do Classifier consultado são excluídos dessa parcela.
  Um atributo substituído por um ancestral intermediário pode reaparecer.
  Não há supressão transitiva adicional nem ambiguidade de override importada
  de uma linguagem-alvo. A expressão consumível por máquina prevalece sobre
  a descrição mais ampla de exclusão, conforme a precedência normativa da
  cláusula 2 adotada pelo plano.
- Os papéis de redefinição são descobertos por descritores e cadeias de
  `subsets`/`redefines`, não por uma lista limitada a Property/Operation.
  `isConsistentWith` é despachado no elemento redefinido, com resolução da
  operação efetiva pela herança de metaclasses. Property e Operation reutilizam
  seus comparadores; o corpo base de RedefinableElement retorna `false`.
  Overrides simples de ActivityNode, ActivityEdge e
  RedefinableTemplateSignature têm seus predicados de tipo reconhecidos;
  isso não certifica seus contextos nem todos os contratos comportamentais.
  Overrides/contextos especializados não implementados continuam explícitos
  como `Unsupported`.
- `SetRedefinedElements` escolhe o papel mais específico cujo tipo admite
  todos os alvos. Port para Property não-Port e coleções mistas usam
  `redefinedProperty`; somente Ports podem usar `redefinedPort`. A substituição
  limpa os papéis anteriores, preservando a união derivada sem alvos obsoletos.
- `null` significa ausência apenas em papéis escalares opcionais, com limites
  `0..1`. Coleções `null`, referência escalar no lugar de coleção e `[null]`
  continuam inválidas. Autorreferência, alvo ausente/incompatível, duplicação,
  `isLeaf`, contexto e ciclos continuam verificados pelos mecanismos existentes.

### Evidência executada

Com Rust 1.98.1, dependências offline bloqueadas, dois jobs e sem compilar a UI:

```sh
cargo test --offline --locked -p bellatrix_mde \
  --test kb09_redefinitions --test graph_rules
```

Rodada inicial após as correções de revisão: **7/7** fluxos novos em
`crates/bellatrix_mde/tests/kb09_redefinitions.rs` e **2/2** em
`crates/bellatrix_mde/tests/graph_rules.rs`. O fluxo antigo que exigia
`allSlottableFeatures ambiguous` foi atualizado para verificar o conjunto
normativo positivo, mantendo os demais casos negativos.

Cobertura: cadeia e diamante; redefinidores independentes; redefinições diretas
múltiplas; atributos privados e Slots; ciclos de generalização e redefinição;
alvos inválidos e finais; default `false` de Classifier; coleções malformadas e
reparo por comando; Port para Property/Port/mistos; substituição e limpeza dos
papéis; rejeição sem mutar o original; reabertura JSON e Protobuf.

A rodada final, após a resolução normativa e a revisão dos bounds independentes,
aprovou **9/9** em `kb09_redefinitions`, **5/5** em `inherited_capabilities`
e **4/4** em `kernel_api_validation`. Essa execução final substitui a limitação
da rodada inicial, que ainda não havia repetido a suíte de herança após suas
últimas correções. Não representa gate integral do Kernel. `git diff --check`
passou. Nenhum teste de UI, build do editor ou aceitação nativa faz parte desta
evidência.

### Resolução da divergência Property::isConsistentWith (2026-11-03)

A investigação confirmou a divergência em ambas as fontes publicadas:
`Property-isConsistentWith-spec` no XMI 20161101 e UML 2.5.1,
§9.9.17.7, p.151, repetem `lowerBound()` no consequente da cláusula
condicionada a `upperBound()`. A descrição textual, por outro lado, exige
contenção de intervalos. A [OMG lista PDF e XMI como normativos](https://www.omg.org/spec/UML/2.5.1/About-UML).

A questão primária [OMG UMLR-424](https://issues.omg.org/issues/UMLR-424),
consultada em 2026-11-03, identifica exatamente esse erro e propõe substituir
o segundo consequente por `prop.upperBound() <= self.upperBound()`.
Seu status é **open**, não uma errata aprovada. A evidência não autoriza
apresentar a correção proposta como corpo OCL publicado.

O Kernel passa a executar o OCL publicado nesse recorte. Após resolver limites
válidos e não nulos (incluindo os defaults), ambas as guardas `notEmpty()` são
verdadeiras e os consequentes exigem `prop.lowerBound() >= self.lowerBound()`
e `prop.lowerBound() <= self.lowerBound()`. Portanto, os limites inferiores
precisam ser iguais; os valores dos superiores não são comparados entre si.
Os limites são consultados independentemente, sem exigir previamente um
intervalo completo válido. Um lower decisivamente incompatível torna a conjunção
falsa mesmo com upper opaco. Se o consequente da implicação do upper for
verdadeiro, o upper opaco também não impede esse resultado verdadeiro.
Um limite não avaliável permanece unknown/unsupported, nunca ausência ou default
inventado; o resultado booleano pode ser decisivo sem resolver esse limite.
Bounds inválidos não são convertidos em unknown ou ignorados.

Um `true` de `Property::isConsistentWith` não certifica o intervalo nem substitui
as validações separadas de `upper_is_unlimitedNatural`, do limite inferior e da
ordem dos limites. Por exemplo, o comparador literal pode aceitar lower iguais
sem comparar os valores de upper entre si, enquanto o snapshot ainda recebe um
diagnóstico por upper inválido ou intervalo inconsistente. Tipo, contexto e
composição continuam sujeitos aos seus contratos separados.
O XMI e o metamodelo gerado não foram alterados. O helper de contenção permanece
intacto para os demais contratos.

Contraexemplos com tipo, contexto e composição compatíveis:

- Base `1..4`, redefinidora `2..3`: literal rejeita; contenção aceita.
- Base `1..4`, redefinidora `1..5` ou `1..*`: literal aceita; contenção rejeita.
- Base `1..*`, redefinidora `2..3`: literal rejeita; contenção aceita.
- Base `1..4`, redefinidora `1..3`: ambas aceitam.
- Base `1..4`, redefinidora `0..3`: ambas rejeitam.

`property_consistency_preserves_published_lower_bound_clause_and_roundtrips`
cobre esses casos, defaults, preservação do helper de contenção, comparador,
validação, comando de redefinição, consulta de Slots e roundtrip JSON/Protobuf.
O fluxo positivo em `inherited_capabilities` usa agora `0..3` por `0..2`;
seu negativo discriminante usa `0..3` por `1..2`.
`property_consistency_evaluates_owned_bounds_independently` acrescenta bounds
possuídos, upper opaco, decisão booleana independente e distinção entre o
comparador e os diagnósticos de multiplicidade do snapshot.

Esta resolução estabelece aderência ao corpo OCL publicado no recorte, não
resolve a contradição editorial da OMG nem implica aceite integral de outros
contratos do Kernel. Nenhum teste de UI ou aceitação nativa está incluído.

## Adendo KB-10: aplicação integrada em drafts sem raiz (2026-11-03)

A auditoria encontrou um desvio entre `Model::validate` e a API pública:
`validate_structural_model` mantinha uma lista independente de passes quando
`root_model` estava ausente. Esse caminho omitia namespace/imports, projeções
de extremidades e subsetting de Property. Assim, o mesmo grafo podia perder
diagnósticos ao retirar apenas a designação da raiz, mesmo sem mudar os
papéis semânticos ou desenhar qualquer View.

`Model::validate_contracts` reúne agora os passes existentes, incluindo os
contratos escalares herdados. Tanto a validação completa quanto o caminho de
drafts sem raiz consomem essa lista única. Os checks de raiz e de ownership
completo continuam exclusivos da validação completa; não se exige uma raiz
ou owner para toda Class provisória. Não foi criado outro modelo, catálogo
ou interpretador OCL.

O fluxo público
`kernel_contract_flow_validates_rooted_and_rootless_drafts_without_views`
verifica, com e sem raiz designada:

- snapshot válido sem Views e sem diagnósticos;
- colisão de nomes, subsetting com tipo incompatível e projeção binária fora
  de `Dependency::client`, com seus diagnósticos específicos;
- preservação dos drafts inválidos e diagnósticos em JSON e Protobuf;
- correção até recuperar o resultado original;
- referência externa não resolvida como warning, não falsa conformidade;
- independência entre chamadas e preservação de Class provisória sem owner;
- igualdade completa dos resultados com/sem raiz para casos válidos, inválidos,
  não resolvidos e escalares malformados, sem diagnósticos duplicados;
- escalar `isAbstract: "yes"` diagnosticado também no draft sem raiz.

Gate focado executado no workspace nativo, Rust 1.98.1, Linux, com cache
compartilhado, `CARGO_BUILD_JOBS=2` e `CARGO_INCREMENTAL=0`:

```text
cargo test --offline --locked -p bellatrix_mde \
  --test kernel_api_validation --test ownership_namespace_flow \
  --test namespace_distinguishability_flow --test relationship_roles_flow \
  --test property_invariants_flow --test association_specialization_flow \
  --test property_contract_flow
```

Resultado: **24 testes aprovados**, incluindo o novo fluxo de integração.
Execução sobre a árvore de trabalho de `0f400666` com este delta KB-10; o SHA
final de publicação deve ser registrado no checkpoint. `git diff --check`
aprovado. Não houve build de UI, geração de aplicação ou aceite UML integral.
A resposta pública continua declarando `coverage: implemented_rules_only`.

Após ampliar as verificações de equivalência e escalares e concluir a revisão
dos bounds de KB-09, a suíte de API foi reexecutada: **4/4 aprovados**. A rodada
final conjunta registrou **9/9 KB-09**, **5/5 herança** e **4/4 API KB-10**.
A divergência editorial **UMLR-424 permanece aberta**; estes testes não convertem
sua proposta em errata aprovada nem ampliam a cobertura UML declarada.

Uma rodada posterior acrescentou o caso de precedência de erro estrutural sobre
limite não computável em ambas as ordens: **10/10 testes KB-09 aprovados**.
