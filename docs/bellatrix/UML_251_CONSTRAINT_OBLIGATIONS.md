# Matriz de obrigações estruturais UML 2.5.1 e plano de fechamento

## Estado e limites da evidência

Levantamento de 2026-11-03 sobre o commit
`08f187df53340c6120bdbd2e09b088ee448dd2a7`. Esta é uma matriz inicial de
obrigações normativas, não um certificado de conformidade. Atualização do lote 1:
o commit `62465b699f9ff9e8bd6da5a6aa2cdc1d73c13d53` publica Port::encapsulated_owner
e as duas invariantes diretas de Component. Essas três linhas foram atualizadas
com evidência específica. Enumeration::immutable também tem implementação no
working tree e gate focado 8/8, descritos em E14, ainda sem SHA de publicação
atribuído. BehavioredClassifier::class_behavior foi publicada em
`d1acb16d7499a2437e25f5f17828acfbc8c174a2`, com evidência focada em E16.
AssociationClass::cannot_be_defined
foi publicada em `867c6afa2a99906936ec23a398a54278ebf5266e`, com gate focado
33/33 e limites de suporte descritos em E15. Connector::types foi publicada em
`8da7a1d891791d2e9ea0462e95cdc5eac36f8afa`, com gate 11/11 descrito em E17.
Connector::roles e DeploymentSpecification::deployment_target foram publicadas em
`17e237ada11fe92e4689b77bdae9f10b7787cd4d`, com recortes e gates focados em
E18/E19. A leitura dessas duas regras usa esse commit, não os deltas posteriores
do working tree. Três invariantes de ConnectorEnd e
DeploymentSpecification::deployed_elements têm recortes publicados em
`c09553f7b0fb11a3580f91bc6e95c553d1582e2b`, descritos em E20/E21.
ConnectorEnd::multiplicity e definingEnd têm recorte publicado em
`211a333d2e4c0b9f468b884a3e77623d868364e2`, descrito em E22.
Connector::kind e CommunicationPath::association_ends têm incrementos publicados
em `bb795fa764adaf5d9724a5a6c4a041557e58d694` e
`f152917ce28279467e1339e9b683ac846bd16cd7`, delimitados em E23/E24.
Property::deployment_target foi publicada em
`4e88d1b9f71f62f2d216130f99b5c978b4c1474a` (E25). As duas invariantes de
ExtensionEnd têm recorte publicado em
`6188ddb14d09d2641fa188c58087f5056de5d18a` (E26).
Nenhum teste novo foi iniciado para elaborar este documento: os resultados
abaixo foram conferidos em logs de gates já executados. Não há percentual de
cobertura, build de UI ou geração de aplicação nesta auditoria.

Este documento complementa, sem substituir, [a auditoria semântica](UML_251_SEMANTIC_AUDIT.md)
e [a auditoria estrutural](UML_251_STRUCTURAL_AUDIT.md). Evidências históricas de
testes nesses documentos não se tornam uma execução nova nesta matriz.

O catálogo guarda descritores e a tabela `UNEVALUATED_CONSTRAINTS`. No documento
UML principal, o inventário registra 674 ocorrências de Constraint, 672 IDs únicos
e 670 IDs não ambíguos. Esses números incluem corpos de operações e não medem
invariantes implementadas. O inventário declara `ConstraintDescriptor (not execution)`.

## Fontes fixadas

Autoridade: [OMG UML 2.5.1](https://www.omg.org/spec/UML/2.5.1).
O corpo do XMI preservado é a fonte desta extração. A cláusula 2 estabelece
precedência dos arquivos normativos consumíveis por máquina em divergências.
Nenhuma errata proposta é incorporada silenciosamente; em particular, UMLR-424
não autoriza substituir o OCL publicado de Property::isConsistentWith.

Os hashes abaixo foram recalculados dos arquivos locais, não apenas copiados
do manifesto. Caminhos relativos à raiz do repositório.

- `Uml`: `docs/bellatrix/standards/uml/20161101/UML.xmi`
  - SHA-256: `e8166c91f51b8a0c015a90101b83d2249d03252e9f9a500a6511654347804f21`.
- `PrimitiveTypes`: `docs/bellatrix/standards/uml/20161101/PrimitiveTypes.xmi`
  - SHA-256: `62d12217fcd26037fc917709e2a896600af574efd6412c110e2a711395a69849`.
- `StandardProfile`: `docs/bellatrix/standards/uml/20161101/StandardProfile.xmi`
  - SHA-256: `b505ebdee791f857da838b5be3394ebd9a857fd044a89625cb71f56badb867bc`.
- `UmlDi`: `docs/bellatrix/standards/uml/20161101/UMLDI.xmi`
  - SHA-256: `5ac8d9414ccfdbca156fc33aeb45b5a36502c33fea391a0fa8bf7b8b1529124c`.
- `PrimitiveTypes20131001`: `docs/bellatrix/standards/uml/imports/20131001/PrimitiveTypes.xmi`
  - SHA-256: `8a2c3944bc4760303350d1220a2bfffcbe75883a343e0d77548904838d3c3347`.
- `Uml20131001`: `docs/bellatrix/standards/uml/imports/20131001/UML.xmi`
  - SHA-256: `f01fe6c97eefed64a3f0fddd19474734f8443fcec1d95e406bb7d4b141ca1dce`.
- `Di`: `docs/bellatrix/standards/uml/imports/20131001/DI.xmi`
  - SHA-256: `f421d673d13fcde22781b2f12a8887af98cf5970b9b910aca13d8b20f27812d4`.
- `Dc`: `docs/bellatrix/standards/uml/imports/20131001/DC.xmi`
  - SHA-256: `e6fc3b4aed32e8a9b07a4188c14f693fa3e52b01e25619d7741d3fcda7459066`.

## Método replicável e escopo do inventário

1. Ler os XML fixados e preservar identidade por documento, xmi:id e ocorrência.
   Não deduplicar definições ambíguas silenciosamente.
2. Para este primeiro inventário, selecionar `uml:Class` diretamente contidas
   nos sete pacotes abaixo e apenas suas `ownedRule` diretas. A seleção produz
   exatamente 127 linhas, não todas as obrigações estruturais possíveis.
3. Reler cada corpo OCL antes de ligar a um predicado. O inventário abaixo
   reproduz o corpo publicado com espaços normalizados; não corrige a fórmula.
4. Seguir o dispatch de validação, consultas chamadas, papéis canônicos e
   testes. Correspondência textual de ID é pista, não prova de implementação.
5. Distinguir regras herdadas, operações auxiliares, contratos de propriedades
   e projeções editoriais. Uma função genérica pode implementar vários contratos,
   mas um descritor não prova que o contrato foi executado.
6. Registrar uma evidência individual antes de promover qualquer estado para
   comprovado. A ausência de diagnóstico não demonstra satisfação de uma regra
   que nunca foi executada.

O escopo ainda precisa de fechamento por herança e dependências de operações.
Values inclui tempo e duração; essas linhas permanecem visíveis, sem decidir
silenciosamente que estão dentro ou fora do marco estrutural. CommonBehavior,
InformationFlows e outros pacotes poderão fornecer dependências. Não usar 127
como denominador de conformidade nem como compromisso de escopo total.

Extração reproduzível, a partir da raiz:

```python
import xml.etree.ElementTree as ET

xmi = "{http://www.omg.org/spec/XMI/20131001}"
packages = {
    "CommonStructure", "Classification", "SimpleClassifiers",
    "StructuredClassifiers", "Packages", "Deployments", "Values",
}
source = "docs/bellatrix/standards/uml/20161101/UML.xmi"
root = ET.parse(source).getroot()
for package in root.iter("packagedElement"):
    if package.get(xmi + "type") != "uml:Package":
        continue
    if package.get("name") not in packages:
        continue
    for classifier in package.findall("packagedElement"):
        if classifier.get(xmi + "type") != "uml:Class":
            continue
        for rule in classifier.findall("ownedRule"):
            body = rule.find("specification/body")
            print(package.get("name"), classifier.get("name"),
                  rule.get(xmi + "id"),
                  "".join(body.itertext()) if body is not None else "")
```

Oito dessas constraints não têm corpo OCL no XMI. Nesses casos, a linha
reproduz a descrição normativa disponível e explicita a ausência de fórmula;
o implementador precisa justificar o predicado a partir dessa descrição.

### Estados

- **R**: predicado relevante inspecionado e teste existente identificado.
  Implementação no recorte, não prova de equivalência integral ao OCL; nenhuma
  nova execução de teste nesta auditoria.
- **P**: implementação parcial ou dependente de valores/representações
  suportados; existem caminhos explicitamente não verificados.
- **U**: implementação declara Unsupported para o contrato especializado.
- **L1**: lacuna identificada na base; lote 1 em andamento, ainda sem aceite.
- **N**: implementação dedicada não localizada após leitura dos passes ativos;
  precisa de implementação ou demonstração individual de cobertura genérica.
- **A**: ainda não auditada individualmente. Não significa ausente, satisfeita
  ou fora do escopo. É uma tarefa aberta de rastreabilidade.

### Evidências de código e testes existentes

Os caminhos de código nesta lista começam em
`crates/bellatrix_mde/src/KERNEL/uml/`; os testes começam em
`crates/bellatrix_mde/tests/`. E01–E11 são pontos de inspeção, não relatórios
de execução nova. E12–E26 possuem os gates delimitados nas seções seguintes.

- **E01**: `semantics/root.rs`, `Model::validate`, e
  `validation/structural_rules.rs`; `ownership_namespace_flow.rs`,
  `kernel_api_validation.rs`. Ownership completo exige raiz; drafts sem raiz
  usam `validate_contracts` e não recebem todos os checks de documento completo.
- **E02**: `semantics/namespace_imports.rs` e `classifiers.rs`;
  `namespace_distinguishability_flow.rs`, `ownership_namespace_flow.rs`.
  Imports externos e consultas incompletas podem permanecer Unsupported.
- **E03**: `validation/structural_diagrams.rs`,
  `validate_structural_model`; `graph_rules.rs` e testes internos do módulo.
  A presença de GeneralizationSet no catálogo não implementa suas invariantes.
- **E04**: `semantics/classifiers.rs`, `validate_feature_rules`;
  `property_invariants_flow.rs`, teste
  `classifier_feature_rules_reject_edits_preserve_drafts_and_reopen`.
- **E05**: `semantics/associations.rs`, `validate_association` e
  `validate_specialization`; `association_kernel.rs`,
  `association_specialization_flow.rs`. Pontas canônicas e ordem importam;
  especializações sem memberEnd materializados permanecem Unsupported.
- **E06**: `semantics/property_contracts.rs`, `validate_property_rules`;
  `property_invariants_flow.rs`. Defaults e qualificadores canônicos são lidos;
  valores malformados não podem ser substituídos por defaults inventados.
- **E07**: `semantics/property_contracts.rs`, `validate_property_rules` e
  `port_default_value`; `port_aggregation.rs`, `port_default_value.rs`.
  Tipo/default externo ou malformado não é prova de satisfação.
- **E08**: `semantics/property_contracts.rs`, `validate_value_rules`, e
  `value_specifications.rs`; `property_contract_flow.rs`.
  Constraint aceita avaliação literal suportada; expressão opaca não prova
  Boolean nem ausência de efeitos colaterais. Não é interpretador OCL geral.
- **E09**: `semantics/operations.rs`, `validate_parameter_relations` e
  `validate_reception`; `inherited_capabilities.rs`,
  `reception_signal_contracts.rs`. Resolver Parameter::end não implementa todas
  as invariantes de ConnectorEnd.
- **E10**: `semantics/templates.rs`, `validate`, e
  `classifier_hierarchy.rs`, `is_substitutable_for`;
  `template_signatures.rs`. A consulta usa Substitutions possuídas, não herança
  ou similaridade estrutural. Argumentos de binding não estão certificados.
- **E11**: `semantics/classifiers.rs`, `redefinition_contexts.rs` e
  `feature_compatibility.rs`; `kb09_redefinitions.rs`.
  Overrides/contextos especializados não implementados continuam Unsupported.
- **U01**: `semantics/associations.rs`, final de `validate_association`:
  regras herdadas são verificadas, mas contratos especializados restantes
  retornam Unsupported. Isso não apaga predicados específicos existentes de
  AssociationClass (E05/E15) nem autoriza autoria completa. CommunicationPath
  tem agora predicado próprio em E24 e deixa esse fallback específico quando
  seus papéis canônicos e contratos herdados são verificados; isso não remove
  o warning independente do adapter legado de metaclasses.
  `association_specialization_flow.rs` testa a fronteira.

- **E12**: `semantics/classifiers.rs`, `validate_feature_rules` e
  `contributes_to_owned_union`; `port_ownership.rs`, quatro testes.
  Owner canônico e papel ownedAttribute definem a projeção de ownedPort.
  Referências externas preservam não verificado, sem ocultar membro local
  comprovado ou erro de coleção. Owner ausente satisfaz a igualdade null=null;
  a regra independente Element::mustBeOwned continua aplicável ao modelo completo.
- **E13**: `semantics/components.rs`, `validate`, `has_owned_role`,
  `has_nesting_class` e `has_forbidden_packaged_elements`, chamado pelo passe
  compartilhado `root.rs::validate_contracts`; `component_constraints.rs`,
  nove testes. A contribuição ao papel nestedClassifier é distinta de
  packagedElement, mesmo quando o owner também conforma a Class. Coleções e
  owners não resolvidos preservam não verificado; consequente vazio pode decidir
  a implicação verdadeira sem resolver o antecedente. Erros estruturais continuam
  diagnosticados independentemente do resultado lógico.

- **E14**: `semantics/enumerations.rs`, `immutable` e `validate`, ligado a
  `root.rs::validate_contracts`; `enumeration_immutability.rs`, oito testes.
  Apenas papéis que contribuem para DataType::ownedAttribute e atributos
  diretamente possuídos participam da regra. Atributos herdados, ownedLiteral,
  ownedOperation e outros papéis não são promovidos a ownedAttribute.
  Default efetivo isReadOnly=false viola a regra; valor não booleano é erro de
  representação, não uma falsa conclusão normativa. Atributo local mutável
  decide o forAll falso mesmo com vizinho externo; coleção inválida não é
  escondida por essa decisão. Coleção vazia comprovada satisfaz; coleção externa
  incompleta permanece não verificada.

- **E15**: `semantics/associations.rs`,
  `association_class_cannot_be_defined`, `validate` e `validate_association`;
  `semantics/classifier_hierarchy.rs`, `has_ancestor`;
  `association_class_constraints.rs`, quatro testes. A própria AssociationClass
  não pode aparecer nos endTypes nem nos allParents desses tipos. Uma prova
  local de inclusão pode decidir a violação mesmo com ramo externo, mas
  `resolve_ancestry` não retorna fechamento parcial como completo. Ausência de
  caminho com ramo não resolvido permanece desconhecida; ciclos e entradas
  inválidas continuam diagnosticados. A regra específica aparece uma vez na
  validação global e também é aplicada no gate de comando. O fallback de
  contratos especializados restantes é preservado.

- **E16**: `semantics/classifiers.rs`, `validate_feature_rules` e
  `role_present`; `class_behavior.rs`, seis testes; teste unitário
  `class_behavior_gate_rejects_new_violation_and_accepts_repaired_draft`.
  A regra segue o Behavior selecionado em classifierBehavior e exige sua
  specification vazia. Behavior possuído mas não selecionado não é inferido
  como classifierBehavior. A regra é herdada por outras metaclasses
  BehavioredClassifier; Class::passive_class continua independente.
  Referências externas permanecem não verificadas e referências inválidas
  mantêm erro de contrato. O editor genérico ainda não escreve
  classifierBehavior: sua rejeição sem mutação não é autoria aprovada.

- **E17**: `semantics/connectors.rs`, `types_rule`, `pair_conforms` e
  `validate`, integrado a `root.rs::validate_contracts`;
  `connector_types.rs`, cinco testes. Com type resolvido, compara aridade e
  conformance de tipos por posição entre Connector::end e Association::memberEnd.
  Type ausente/null satisfaz somente esta implicação. Não infere a ordem de
  coleções ausentes a partir de owns ou da apresentação. Ambos os contratos de
  coleção e lados da comparação são inspecionados: ausência ou referência externa
  de um lado não oculta malformado do outro. Uma incompatibilidade local conhecida
  pode decidir falso ao lado de unknown, sem apresentar listas parciais como
  resultados completos.

- **E18**: `semantics/connectors.rs`, `structured_classifier`, `end_role`,
  `roles_rule` e `validate`, integrado a `root.rs::validate_contracts`;
  `semantics/classifiers.rs`, `all_roles` e `has_role`;
  `connector_roles.rs`, nove testes. Deriva structuredClassifier do owner
  canônico no papel ownedConnector e exige coerência da inversa armazenada.
  allRoles filtra ConnectableElement em allFeatures, usando o motor existente
  de namespace, visibilidade e redefinição. A consulta completa não retorna
  coleção parcial como completa; has_role pode provar membership diretamente
  possuído diante de namespace incompleto, sem inventar membership herdado.
  Cada ponta exige role em allRoles, ou role Port com partWithPort em allRoles.
  Ambos os inputs e todas as pontas são inspecionados mesmo com owner/inversa
  externos; malformados, ciclos e violações locais não são ocultados por unknown.
  Diagnósticos: `UML_CONNECTOR_ROLES` e `MDE_CONNECTOR_ROLES_UNVERIFIED`.

- **E19**: `semantics/deployments.rs`, `opposite_owner`, `invalid_target` e
  `validate`, integrado a `root.rs::validate_contracts`;
  `deployment_specification.rs`, dez testes. Deriva deployment e location das
  relações canônicas de ownership e dos papéis Deployment::configuration e
  DeploymentTarget::deployment; inversas armazenadas não substituem essa prova.
  Deployment ausente comprovado satisfaz o forAll vazio; deployment presente
  exige location canônica DeploymentTarget e sua conformidade a
  ExecutionEnvironment. Location canônica ausente é erro de ownership; owner
  com metaclasse desconhecida permanece não verificado. Inversa externa não
  apaga prova local de alvo inválido nem malformados da location; não basta
  para certificar sucesso quando ainda há evidência desconhecida.
  Diagnósticos: `UML_DEPLOYMENT_SPECIFICATION_TARGET`,
  `MDE_DEPLOYMENT_SPECIFICATION_OWNERSHIP` e
  `MDE_DEPLOYMENT_SPECIFICATION_UNVERIFIED`.

- **E20**: `semantics/connectors.rs`, `role_and_part_with_port`,
  `part_with_port_empty`, `self_part_with_port` e `validate`;
  `connector_end_constraints.rs`, nove testes. partWithPort presente exige role
  Port e membership no Namespace do tipo da parte, consultado pelo motor de
  namespace com herança e visibilidade. Port cujo owner coincide com o owner
  do Connector exige partWithPort vazio; a inversa connector armazenada não
  pode substituir ownership canônico. A própria parte não pode ser Port.
  As implicações preservam antecedente comprovadamente falso sem inventar
  warning pelo outro input desconhecido, mas malformados continuam erros.
  Referências externas e consultas incompletas conservam resultados não
  verificados quando necessários à prova. Os diagnósticos dedicados usam
  `UML_CONNECTOR_END_ROLE_AND_PART_WITH_PORT`,
  `UML_CONNECTOR_END_PART_WITH_PORT_EMPTY` e
  `UML_CONNECTOR_END_SELF_PART_WITH_PORT`, com variantes
  `MDE_CONNECTOR_END_*_UNVERIFIED` para resultados desconhecidos.

- **E21**: `semantics/deployments.rs`, `deployed_components`,
  `reference_targets`, `composite_targets` e `validate`;
  `deployment_deployed_elements.rs`, nove testes. A projeção parte da location
  canônica e percorre todos os seus deployments, incluindo irmãos, seus
  deployedArtifact que conformam a Artifact, manifestações canônicas e
  utilizedElement. Cada elemento resultante deve conformar a Component.
  Objetos deployedArtifact que não são Artifact e Artifact sem manifestações
  não acrescentam elementos à projeção. Papéis referenciais ausentes não são
  inventados por endpoints visuais nem pelo cache derivado deployedElement.
  Coleção comprovadamente vazia pode satisfazer a regra; metaclasses externas
  ou não resolvidas não provam vazio. Evidência local de não Component e
  malformados não desaparecem diante de ramos externos.
  Diagnósticos: `UML_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS`,
  `MDE_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS_INVALID` e
  `MDE_DEPLOYMENT_SPECIFICATION_DEPLOYED_ELEMENTS_UNVERIFIED`.

- **E22**: `semantics/connectors.rs`, `owning_connector`,
  `defining_end_projection`, `defining_end`, `compatible_multiplicity` e
  `end_multiplicity`; `semantics/value_specifications.rs`, avaliação de bounds;
  `connector_end_multiplicity.rs`, dez testes. definingEnd usa a posição do
  ConnectorEnd na coleção canônica explícita Connector::end para selecionar
  Association::memberEnd. Ordem de ownership e cache definingEnd não são prova.
  A consulta pública preserva unknown quando a projeção não está completa;
  internamente, uma posição conhecida ainda permite detectar violação local.
  A comparação exige lower do definingEnd <= lower do ConnectorEnd e upper do
  ConnectorEnd <= upper do definingEnd, ou upper geral ilimitado. Bounds são
  avaliados separadamente: uma incompatibilidade conhecida e valores inválidos
  não desaparecem diante de bound opaco ou referência externa no outro lado.
  Diagnósticos: `UML_CONNECTOR_END_MULTIPLICITY` e
  `MDE_CONNECTOR_END_MULTIPLICITY_UNVERIFIED`.

- **E23**: `semantics/connectors.rs`, `delegation_end`, `kind` e `validate`;
  `connector_kind.rs`, oito testes. A operação `Connector-kind.1-spec` deriva
  Delegation quando existe ponta com role Port, partWithPort vazio e
  isBehavior efetivo falso; sem tal ponta comprovadamente, deriva Assembly.
  O default normativo de isBehavior é aplicado, mas booleano malformado não
  vira default. Examina todas as pontas: uma testemunha local de Delegation
  decide exists mesmo com ramo externo, sem ocultar erro em outra ponta.
  Ausência de prova diante de referências não resolvidas não prova Assembly.
  Cache kind armazenado e ordem de ownership não substituem os papéis
  canônicos. Diagnósticos: `UML_CONNECTOR_KIND_DERIVATION` e
  `MDE_CONNECTOR_KIND_UNVERIFIED`. Estado do recorte: **P**; é corpo de operação,
  não nova invariante a acrescentar às 127 linhas.

- **E24**: `semantics/associations.rs`, `communication_path_association_ends`,
  `validate_association` e `validate`; `communication_path.rs`, sete testes.
  Cada tipo resolvido das Properties canônicas em memberEnd deve conformar a
  DeploymentTarget. Usa identidades do kernel, não endpoints visuais nem cache
  endType. Tipo opcional ausente não integra endType; a regra herdada de pontas
  tipadas continua rejeitando a associação sem tipo. Todos os membros são
  inspecionados para preservar erro local diante de unknown; coleção incompleta
  não é apresentada como completa. O gate de comandos compartilha o predicado
  e a validação global evita duplicar o diagnóstico da regra específica.
  Diagnósticos: `UML_COMMUNICATION_PATH_ASSOCIATION_ENDS`,
  `MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_INVALID` e
  `MDE_COMMUNICATION_PATH_ASSOCIATION_ENDS_UNVERIFIED`.

- **E25**: `semantics/deployments.rs`, `property_deployment_target` e
  `validate`; `property_deployment_target.rs`, doze testes. Deployment canônico
  não vazio exige que a Property seja ownedAttribute composto de um Node.
  Ownership único permite usar o Node proprietário como testemunha da operação
  part; atributo apenas herdado não é parte possuída. Subtipos Node, como Device
  e ExecutionEnvironment, são considerados. Ausência comprovada de deployment
  satisfaz a implicação; não se infere deployment da Property pelo deployment
  do Node ou por filhos de outro papel. Membro canônico conhecido pode provar
  não vazio sem apresentar coleção parcial como completa; inversas conflitantes
  e coleções malformadas continuam erros. Antecedente desconhecido pode ser
  decidido por consequente comprovado; caso contrário permanece não verificado.
  Diagnósticos: `UML_PROPERTY_DEPLOYMENT_TARGET`,
  `MDE_PROPERTY_DEPLOYMENT_TARGET_INVALID` e
  `MDE_PROPERTY_DEPLOYMENT_TARGET_UNVERIFIED`.

- **E26**: implementação publicada em `semantics/profiles.rs`,
  `end_multiplicity`, `end_aggregation` e `validate`, ligada ao passe comum
  `root.rs::validate_contracts`; `semantics/value_specifications.rs`,
  `default_lower_bound`, `lower_bound` e `multiplicity`;
  `extension_end_constraints.rs`, oito testes. ExtensionEnd exige lower 0 ou 1
  e upper 1, com default efetivo 0..1 quando os bounds estão ausentes, sem
  alterar o default 1..1 de Property. Bounds por identidade são avaliados;
  externos, opacos ou não atribuídos não viram defaults inventados. Lower e
  upper independentes preservam violação conhecida ou valor inválido diante
  de unknown no outro lado. Aggregation deve ser composite; ausência não é
  convertida silenciosamente em composite. Diagnósticos próprios usam
  `UML_EXTENSION_END_MULTIPLICITY` e `UML_EXTENSION_END_AGGREGATION`, com
  variantes `MDE_EXTENSION_END_*_UNVERIFIED`.

### Evidência de Property::deployment_target e ExtensionEnd

Property::deployment_target: commit inspecionado
`4e88d1b9f71f62f2d216130f99b5c978b4c1474a`. O log local
`/tmp/property-deployment-gate-final.log` registra **31/31 aprovados**:
property_deployment_target 12, deployment_deployed_elements 9 e
deployment_specification 10. O recorte inclui ownership/papéis canônicos,
subtipos Node, herança que não constitui part, unknowns, inversas contraditórias,
rejeição atômica de reparent e reparo de draft. A linha fica **P/E25**;
Node::internal_structure continua uma decisão normativa independente e pendente.

ExtensionEnd: commit publicado
`6188ddb14d09d2641fa188c58087f5056de5d18a`, sobre
`4e88d1b9f71f62f2d216130f99b5c978b4c1474a`. A revisão confirmou árvore idêntica
ao snapshot testado antes da publicação; isso não é rerun sobre o SHA publicado.
O log local `/tmp/extension-end-gate-final.log` registra **46/46 aprovados**:
connector_end_constraints 10, connector_end_multiplicity 10, connector_kind 8,
extension_end_constraints 8, property_collection_validation 6 e
reception_signal_contracts 4. Os testes incluem defaults, null, valores por
identidade, tipos incorretos, opacos, referências externas, precedência de
malformados, roundtrip JSON/Protobuf e comandos atômicos em container suportado.

As duas linhas ExtensionEnd ficam **P/E26**. O teste com container Extension
completo preserva `MDE_ASSOCIATION_UNSUPPORTED`, inclusive para edição que seria
válida isoladamente. Não declarar autoria completa de Extension, contratos de
Profile ou suporte integral a profiles. Esses gates são focados, não suíte
integral; não houve nova execução pela matriz nem rerun no SHA publicado.
A publicação não amplia a cobertura desses gates nem comprova UI ou geração.

### Evidência publicada de Connector::kind e CommunicationPath

Commits inspecionados: `bb795fa764adaf5d9724a5a6c4a041557e58d694` (kind) e
`f152917ce28279467e1339e9b683ac846bd16cd7` (CommunicationPath). Logs locais lidos:

- `/tmp/connector-kind-gate-final.log`: **42/42 aprovados**,
  connector_end_constraints 10, connector_end_multiplicity 10, connector_kind 8,
  connector_roles 9 e connector_types 5.
- `/tmp/communication-path-gate.log`: **15/15 aprovados**,
  association_class_constraints 4, association_kernel 2,
  association_specialization_flow 2 e communication_path 7.

São gates focados anteriores à publicação, não reruns sobre os SHAs remotos nem
suíte integral. Não houve execução nova pela matriz. Kind cobre default,
branches externos em ambas as ordens, input malformado, cache não autoritativo
e rederivação sem mutação. CommunicationPath cobre API com/sem raiz, tipos
DeploymentTarget, cardinalidade n-ária, tipos duplicados, ausência de tipo,
papéis não materializados, unknowns, malformados, rollback, reparo e
roundtrip JSON/Protobuf.

CommunicationPath passa de U para **P/E24**: o fallback especializado é removido
para esse contrato canônico, mas o warning legado `MDE_UML_METACLASS_UNSUPPORTED`
continua. Isso não significa suporte completo à metaclasse no adapter de
autoria ou aos fluxos visuais. Kind permanece consulta semântica derivada:
não implementa adapter binário ou autoria visual de Connector. Nenhum desses
gates comprova renderização, geração ou interchange XMI.

### Evidência publicada de ConnectorEnd::multiplicity/definingEnd

Commit inspecionado: `211a333d2e4c0b9f468b884a3e77623d868364e2`.
A revisão confirmou árvore `59d1b6c1db1fc2272909c52a75e16493a76b356c`
idêntica à do commit local anterior à publicação, `74aeec0`.
O log local `/tmp/connector-end-multiplicity-gate-final.log` registra
**44/44 aprovados**: connector_end_constraints 10, connector_end_multiplicity 10,
connector_roles 9, connector_types 5, property_collection_validation 6 e
reception_signal_contracts 4. É gate focado da árvore de implementação anterior
à publicação, não rerun no SHA remoto nem suíte integral. Nenhum teste novo foi
executado pela matriz. O log `/tmp/connector-value-specifications-gate.log`
executou **zero testes**, com 159 filtrados, e não é evidência de cobertura
adicional de value_specifications.

Os testes discriminam posições versus ordem de ownership/cache, coleções
malformadas ou ausentes, índice fora da coleção, referências externas,
intervalos inferiores/superiores, ilimitado, bounds por identidade e opacos,
precedência de erro em ambas as ordens, rollback e reparo de draft. Uma regressão
adicional de ConnectorEnd exige que membership canônico inválido não seja
ocultado por role externo ou partWithPort vazio.

A linha permanece **P/E22**. Connector sem type retorna definingEnd ausente e
não inventa comparação; isso registra o comportamento implementado, sem alegar
fechamento da interpretação de compatibleWith(null). Bounds não computáveis,
referências externas e ordem não materializada continuam limites explícitos.
A operação auxiliar definingEnd não acrescenta uma invariante ao inventário.
Não declarar suporte integral a Connectors, autoria visual ou conformidade UML.

### Evidência de ConnectorEnd e deployed_elements

Commit publicado: `c09553f7b0fb11a3580f91bc6e95c553d1582e2b`.
A implementação foi inspecionada no commit local
`0c448caf056d7518a82d2d079488ad35bfba5992`; a revisão confirmou a mesma árvore
`d885d518eb280c42832b902a637250151085c6d0` nos dois commits. O log local
`/tmp/connector-end-deployment-gate-final.log` registra **42/42 aprovados**:
connector_end_constraints 9, connector_roles 9, connector_types 5,
deployment_deployed_elements 9 e deployment_specification 10. É um gate focado
executado na árvore de código correspondente antes da publicação, não um
rerun no SHA remoto, suíte integral nem execução nova pela matriz. O log não
registra fingerprint completo da árvore testada; a igualdade das árvores dos
commits não substitui esse registro histórico.

Os casos incluem namespace/visibilidade de Port herdado, partes proibidas,
inversas contraditórias, desconhecidos versus malformados, implicações com
antecedente falso, rejeição atômica de mudanças de owner e roundtrip
JSON/Protobuf. Deployment cobre API com/sem raiz, localização compartilhada,
papéis ausentes, cache derivado não autoritativo, ramos externos, projeções
vazias e reparo após rejeição de reparent.

As quatro linhas ficam **P/E20–E21**, sem certificar todas as representações ou
fluxos de autoria. O incremento posterior de ConnectorEnd::multiplicity e
definingEnd está em E22; Node::internal_structure continua pendente de decisão
normativa. Esses gates não comprovam UI, geração, interchange
XMI nem fechamento integral de Connectors ou Deployments.

### Evidência transversal de envelopes externos no working tree

Ajustes inspecionados no working tree sobre
`d1ef526` (base documental).
O delta foi posteriormente registrado no commit local
`556b16456ed51d6de8352b2ef192b046bdcde913`; os resultados abaixo são dos gates
anteriores ao commit, não uma execução nova sobre esse SHA. Nenhuma linha do
inventário é promovida por esses números.

`semantics/references.rs::external_reference_href` exige objeto com exatamente
uma chave href e string não vazia. href null, booleano, número, vazio, mistura
com $ref ou campos adicionais são malformados, não referências apenas
não resolvidas. A validação é compartilhada por contratos de propriedades,
membership composto, subsetting, consultas diretas de tipo/assinatura e bounds.
URI relativa é permitida pelo envelope; sintaxe e resolução de URI não são
implementadas aqui. Envelope válido continua não verificado quando exige
resolver documento externo, e não se torna prova de tipo ou identidade UML.

`property_collection_validation.rs` contém seis testes, incluindo precedência
de violação local sobre desconhecido em ambas as ordens, coleções mistas,
queries diretas e roundtrip/API pública. O teste unitário
`composite_membership_rejects_malformed_external_envelopes_beside_local_proof`
verifica que nem membership local comprovado oculta envelope malformado.
Na revisão, expectativas de warnings de associações legadas foram atualizadas
nos testes de structural_rules/structural_diagrams sem remover Unsupported;
a responsabilidade de Graph por endpoint inexistente também permanece testada.

Logs locais lidos, sem nova execução pela matriz:

- `/tmp/external-reference-final-regressions.log`: **19/19 integrações**,
  distribuídas em graph_rules 2, kernel_api_validation 4,
  ownership_namespace_flow 4, property_collection_validation 6 e
  property_contract_flow 3.
- `/tmp/external-reference-final-unit.log`: **1/1 unitário focado**,
  com 158 filtrados.
- `/tmp/structural-warning-regressions.log`: **36/36 unitários selecionados**,
  com 123 filtrados, após revisão das expectativas estruturais.
- `/tmp/structural-warning-full-lib.log`: **159/159 testes lib**, sem filtrados.

Os gates unitários se sobrepõem; não somá-los como testes distintos. A suíte lib
completa não equivale à suíte integral de integrações, conformidade UML 2.5.1,
validação de UI, geração ou suporte a documentos externos. Os logs não fixam
fingerprint completo do working tree; futuras alterações exigem novos gates.

### Evidência publicada de Connector::roles e deployment_target

Commit de implementação inspecionado:
`17e237ada11fe92e4689b77bdae9f10b7787cd4d`. Logs locais finais lidos:

- `/tmp/connector-roles-fix-tests-final.log`: **9/9 connector_roles** e
  **5/5 connector_types**, após as correções adversariais.
- `/tmp/deployment-evidence-fix-tests.log`: **10/10 deployment_specification**,
  incluindo regressões com inversas externas e evidência canônica local.

São gates focados anteriores à publicação, não execução nova sobre o SHA
publicado nem suíte integral. Os logs não registram fingerprint completo da
árvore testada; o código e os testes foram conferidos no commit indicado.
O log anterior `/tmp/deployment-specification-gate.log` registra duas falhas;
o gate final acima contém os testes corrigidos e regressões adicionais.
`/tmp/connector-roles-final.log` termina sem resultado completo de
inherited_capabilities, portanto não comprova aprovação desse alvo.

Connector cobre roles diretos/herdados, visibilidade, redefinição, diamante,
Port/partWithPort, inversas contraditórias, referências externas, coleções
malformadas, ciclos, precedência em ambas as ordens, rejeição atômica e
reabertura JSON/Protobuf. DeploymentSpecification cobre API com/sem raiz,
ExecutionEnvironment versus outros targets, ausência de deployment,
ownership/inversas inválidas ou desconhecidas, rejeição atômica de reparent,
reparo de draft e roundtrip JSON/Protobuf. Isso não certifica todos os fluxos
de autoria nem todas as representações do catálogo.

As duas linhas permanecem **P**. Três invariantes próprias de ConnectorEnd e
DeploymentSpecification::deployed_elements têm incremento posterior em E20/E21.
ConnectorEnd::multiplicity tem recorte posterior em E22;
Node::internal_structure continua pendente. Kind tem recorte posterior em E23;
adapter binário, autoria visual de Connector e semântica integral de deployment
não são demonstrados; não houve build de UI nem geração.

### Evidência publicada de Connector::types

Commit: `8da7a1d891791d2e9ea0462e95cdc5eac36f8afa`. O log local
`/tmp/connector-types-symmetric-review.log` foi lido e registra **11/11 aprovados**:
Connector 5, Association kernel 2 e API 4. É um gate focado do incremento,
não suíte integral. Não houve nova execução pela matriz.

Os fluxos cobrem ordem, cardinalidade, identidade/herança, type opcional,
ownership, tipos incorretos, unknowns, rollback e JSON/Protobuf. Casos simétricos
verificam null, escalar, membro malformado, duplicação e href na coleção oposta
quando uma coleção está ausente. A linha permanece P/E17.
Esse gate isolado não comprovava `Connector-roles` ou
`StructuredClassifier::allRoles`; o incremento posterior está delimitado em E18.
As invariantes próprias de ConnectorEnd não são provadas por esse gate;
três delas têm recorte posterior em E20 e multiplicity em E22. Kind tem recorte
posterior em E23; adapter binário e autoria visual de Connector não são
certificados por esses incrementos.
Não alegar suporte completo a Connectors ou Composite Structure.

### Evidência publicada de BehavioredClassifier

Commit: `d1acb16d7499a2437e25f5f17828acfbc8c174a2`. Logs locais lidos:
`/tmp/class-behavior-gate.log` registra **6/6 class_behavior** e
**5/5 property_invariants_flow**; `/tmp/class-behavior-kernel-gate.log`
registra **1/1 teste unitário focado**, com 157 testes filtrados.
O teste unitário verifica rejeição pelo gate semântico e aceite após reparo;
não substitui um fluxo de autoria pelo editor genérico, ainda limitado.

Os testes incluem API com/sem raiz, specification ausente/nula/preenchida,
Behavior não selecionado, herança de metaclasses, referências externas e
malformadas, roundtrip e preservação do modelo após comando não suportado.
A linha fica P/E16, com limites de referências e autoria explícitos. Esses gates
não constituem execução integral nem conformidade de comportamento em runtime.
A matriz não iniciou novos testes, UI ou geração para registrar a evidência.

### Evidência publicada de AssociationClass

Commit: `867c6afa2a99906936ec23a398a54278ebf5266e`. O log local
`/tmp/association-class-review.log` foi lido e registra **33/33 aprovados**:
AssociationClass 4, Association kernel 2, Association specialization 2, API 4 e
templates 21. Este é o gate focado de revisão do incremento, não a suíte integral
do Kernel. A matriz não executou esses testes novamente.

O recorte comprova referência própria, ancestral direto/transitivo, ramos
externos em ambas ordens, malformados, ciclos, contagem do diagnóstico,
reabertura e rejeição atômica. Comandos válidos ainda bloqueados pelo fallback
especializado não são contabilizados como autoria aprovada. A linha fica P/E15;
as outras obrigações e a autoria completa de AssociationClass continuam fora
desta alegação. Não houve mudança de UI nem geração.

### Evidência de Enumeration no working tree

Implementação inspecionada no working tree sobre
`02b4b90af53d2c4acca34d8093edd26d974b4fa1`, sem atribuir SHA final ao delta.
O log local `/tmp/enumeration-immutability-role-final.log` foi lido e registra
**8/8 testes aprovados** no alvo `enumeration_immutability`. Não houve nova
execução para atualizar a matriz. O alvo cobre API com/sem raiz, defaults,
malformados, referências externas, papéis não relacionados, herança, rollback,
reparo e reabertura. Não representa gate integral nem publicação confirmada.
A linha fica P/E14 até manter evidência do recorte e da publicação efetiva;
publicar o commit, por si só, também não demonstrará conformidade integral.

### Evidência executada do incremento publicado

Commit de implementação: `62465b699f9ff9e8bd6da5a6aa2cdc1d73c13d53`.
Foram lidos os logs locais do ambiente de trabalho, sem executar novamente os
comandos. São referências de investigação, não artefatos versionados duráveis:

- `/tmp/structural-owner-component-gate.log`: **33/33 aprovados**, distribuídos
  em Port ownership 4, aggregation 3, defaultValue 6, API 4,
  ownership/namespace 4, inherited capabilities 5 e Component constraints 7.
- `/tmp/component-constraints-final.log`: **9/9 Component aprovados** depois
  das ampliações posteriores de testes e decisão trivalente.

O primeiro gate antecede as ampliações finais de Component; não é um gate de
33 testes repetido sobre o SHA final. O segundo comprova o alvo Component final,
sem converter os gates em suíte integral ou conformidade UML. As três linhas
ficam P: contratos executados no recorte canônico suportado, com fronteiras
não verificadas preservadas. O estado publicado não elimina a necessidade de
fechar as demais obrigações e seus testes individuais.

## Inventário por xmi:id

Cada corpo abaixo veio diretamente da fonte principal. Linhas A são mantidas
mesmo quando há infraestrutura relacionada, para não inferir cobertura.

### CommonStructure: 27 invariantes diretas

#### `Constraint-boolean_value`

- Metaclass: `Constraint`. Estado: **P**. Evidência: E08.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: The ValueSpecification for a Constraint must evaluate to a Boolean value.

#### `Constraint-no_side_effects`

- Metaclass: `Constraint`. Estado: **P**. Evidência: E08.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: Evaluating the ValueSpecification for a Constraint must not have side effects.

#### `Constraint-not_apply_to_self`

- Metaclass: `Constraint`. Estado: **P**. Evidência: E08.
- OCL publicado: `not constrainedElement->includes(self)`.

#### `Element-has_owner`

- Metaclass: `Element`. Estado: **P**. Evidência: E01.
- OCL publicado: `mustBeOwned() implies owner->notEmpty()`.

#### `Element-not_own_self`

- Metaclass: `Element`. Estado: **P**. Evidência: E01.
- OCL publicado: `not allOwnedElements()->includes(self)`.

#### `ElementImport-imported_element_is_public`

- Metaclass: `ElementImport`. Estado: **P**. Evidência: E02.
- OCL publicado: `importedElement.visibility <> null implies importedElement.visibility = VisibilityKind::public`.

#### `ElementImport-visibility_public_or_private`

- Metaclass: `ElementImport`. Estado: **P**. Evidência: E02.
- OCL publicado: `visibility = VisibilityKind::public or visibility = VisibilityKind::private`.

#### `MultiplicityElement-upper_ge_lower`

- Metaclass: `MultiplicityElement`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `upperBound() >= lowerBound()`.

#### `MultiplicityElement-lower_ge_0`

- Metaclass: `MultiplicityElement`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `lowerBound() >= 0`.

#### `MultiplicityElement-value_specification_no_side_effects`

- Metaclass: `MultiplicityElement`. Estado: **A**. Evidência: revisão individual pendente.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: If a non-literal ValueSpecification is used for lowerValue or upperValue, then evaluating that specification must not have side effects.

#### `MultiplicityElement-value_specification_constant`

- Metaclass: `MultiplicityElement`. Estado: **A**. Evidência: revisão individual pendente.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: If a non-literal ValueSpecification is used for lowerValue or upperValue, then that specification must be a constant expression.

#### `MultiplicityElement-lower_is_integer`

- Metaclass: `MultiplicityElement`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `lowerValue <> null implies lowerValue.integerValue() <> null`.

#### `MultiplicityElement-upper_is_unlimitedNatural`

- Metaclass: `MultiplicityElement`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `upperValue <> null implies upperValue.unlimitedValue() <> null`.

#### `NamedElement-visibility_needs_ownership`

- Metaclass: `NamedElement`. Estado: **P**. Evidência: E01.
- OCL publicado: `(namespace = null and owner <> null) implies visibility = null`.

#### `NamedElement-has_qualified_name`

- Metaclass: `NamedElement`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(name <> null and allNamespaces()->select(ns | ns.name = null)->isEmpty()) implies qualifiedName = allNamespaces()->iterate( ns : Namespace; agg: String = name | ns.name.concat(self.separator()).concat(agg))`.

#### `NamedElement-has_no_qualified_name`

- Metaclass: `NamedElement`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `name=null or allNamespaces()->select( ns | ns.name=null )->notEmpty() implies qualifiedName = null`.

#### `Namespace-members_distinguishable`

- Metaclass: `Namespace`. Estado: **P**. Evidência: E02.
- OCL publicado: `membersAreDistinguishable()`.

#### `Namespace-cannot_import_self`

- Metaclass: `Namespace`. Estado: **P**. Evidência: E02.
- OCL publicado: `packageImport.importedPackage.oclAsType(Namespace)->excludes(self)`.

#### `Namespace-cannot_import_ownedMembers`

- Metaclass: `Namespace`. Estado: **P**. Evidência: E02.
- OCL publicado: `elementImport.importedElement.oclAsType(Element)->excludesAll(ownedMember)`.

#### `PackageableElement-namespace_needs_visibility`

- Metaclass: `PackageableElement`. Estado: **P**. Evidência: E01.
- OCL publicado: `visibility = null implies namespace = null`.

#### `PackageImport-public_or_private`

- Metaclass: `PackageImport`. Estado: **P**. Evidência: E02.
- OCL publicado: `visibility = VisibilityKind::public or visibility = VisibilityKind::private`.

#### `TemplateBinding-parameter_substitution_formal`

- Metaclass: `TemplateBinding`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `parameterSubstitution->forAll(b | signature.parameter->includes(b.formal))`.

#### `TemplateBinding-one_parameter_substitution`

- Metaclass: `TemplateBinding`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `signature.parameter->forAll(p | parameterSubstitution->select(b | b.formal = p)->size() <= 1)`.

#### `TemplateParameter-must_be_compatible`

- Metaclass: `TemplateParameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `default <> null implies default.isCompatibleWith(parameteredElement)`.

#### `TemplateParameterSubstitution-must_be_compatible`

- Metaclass: `TemplateParameterSubstitution`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `actual->forAll(a | a.isCompatibleWith(formal.parameteredElement))`.

#### `TemplateSignature-own_elements`

- Metaclass: `TemplateSignature`. Estado: **P**. Evidência: E10.
- OCL publicado: `template.ownedElement->includesAll(parameter.parameteredElement->asSet() - parameter.ownedParameteredElement->asSet())`.

#### `TemplateSignature-unique_parameters`

- Metaclass: `TemplateSignature`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `parameter->forAll( p1, p2 | (p1 <> p2 and p1.parameteredElement.oclIsKindOf(NamedElement) and p2.parameteredElement.oclIsKindOf(NamedElement) ) implies p1.parameteredElement.oclAsType(NamedElement).name <> p2.parameteredElement.oclAsType(NamedElement).name)`.

### Classification: 44 invariantes diretas

#### `BehavioralFeature-abstract_no_method`

- Metaclass: `BehavioralFeature`. Estado: **R**. Evidência: E04.
- OCL publicado: `isAbstract implies method->isEmpty()`.

#### `Classifier-specialize_type`

- Metaclass: `Classifier`. Estado: **R**. Evidência: E03.
- OCL publicado: `parents()->forAll(c | self.maySpecializeType(c))`.

#### `Classifier-maps_to_generalization_set`

- Metaclass: `Classifier`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `powertypeExtent->forAll( gs | gs.generalization->forAll( gen | not (gen.general = self) and not gen.general.allParents()->includes(self) and not (gen.specific = self) and not self.allParents()->includes(gen.specific) ))`.

#### `Classifier-non_final_parents`

- Metaclass: `Classifier`. Estado: **R**. Evidência: E03.
- OCL publicado: `parents()->forAll(not isFinalSpecialization)`.

#### `Classifier-no_cycles_in_generalization`

- Metaclass: `Classifier`. Estado: **R**. Evidência: E03.
- OCL publicado: `not allParents()->includes(self)`.

#### `ClassifierTemplateParameter-has_constraining_classifier`

- Metaclass: `ClassifierTemplateParameter`. Estado: **P**. Evidência: E10.
- OCL publicado: `allowSubstitutable implies constrainingClassifier->notEmpty()`.

#### `ClassifierTemplateParameter-parametered_element_no_features`

- Metaclass: `ClassifierTemplateParameter`. Estado: **P**. Evidência: E10.
- OCL publicado: `parameteredElement.feature->isEmpty() and (constrainingClassifier->isEmpty() implies parameteredElement.allParents()->isEmpty())`.

#### `ClassifierTemplateParameter-matching_abstract`

- Metaclass: `ClassifierTemplateParameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(not parameteredElement.isAbstract) implies templateParameterSubstitution.actual->forAll(a | not a.oclAsType(Classifier).isAbstract)`.

#### `ClassifierTemplateParameter-actual_is_classifier`

- Metaclass: `ClassifierTemplateParameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `templateParameterSubstitution.actual->forAll(a | a.oclIsKindOf(Classifier))`.

#### `ClassifierTemplateParameter-constraining_classifiers_constrain_args`

- Metaclass: `ClassifierTemplateParameter`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `templateParameterSubstitution.actual->forAll( a | let arg : Classifier = a.oclAsType(Classifier) in constrainingClassifier->forAll( cc | arg = cc or arg.conformsTo(cc) or (allowSubstitutable and arg.isSubstitutableFor(cc)) ) )`.

#### `ClassifierTemplateParameter-constraining_classifiers_constrain_parametered_element`

- Metaclass: `ClassifierTemplateParameter`. Estado: **P**. Evidência: E10.
- OCL publicado: `constrainingClassifier->forAll( cc | parameteredElement = cc or parameteredElement.conformsTo(cc) or (allowSubstitutable and parameteredElement.isSubstitutableFor(cc)) )`.

#### `GeneralizationSet-generalization_same_classifier`

- Metaclass: `GeneralizationSet`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `generalization->collect(general)->asSet()->size() <= 1`.

#### `GeneralizationSet-maps_to_generalization_set`

- Metaclass: `GeneralizationSet`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `powertype <> null implies generalization->forAll( gen | not (gen.general = powertype) and not gen.general.allParents()->includes(powertype) and not (gen.specific = powertype) and not powertype.allParents()->includes(gen.specific) )`.

#### `InstanceSpecification-deployment_artifact`

- Metaclass: `InstanceSpecification`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `deploymentForArtifact->notEmpty() implies classifier->exists(oclIsKindOf(Artifact))`.

#### `InstanceSpecification-structural_feature`

- Metaclass: `InstanceSpecification`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `classifier->forAll(c | (c.allSlottableFeatures()->forAll(f | slot->select(s | s.definingFeature = f)->size() <= 1)))`.

#### `InstanceSpecification-defining_feature`

- Metaclass: `InstanceSpecification`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `slot->forAll(s | classifier->exists (c | c.allSlottableFeatures()->includes (s.definingFeature)))`.

#### `InstanceSpecification-deployment_target`

- Metaclass: `InstanceSpecification`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `deployment->notEmpty() implies classifier->exists(node | node.oclIsKindOf(Node) and Node.allInstances()->exists(n | n.part->exists(p | p.type = node)))`.

#### `Operation-at_most_one_return`

- Metaclass: `Operation`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `self.ownedParameter->select(direction = ParameterDirectionKind::return)->size() <= 1`.

#### `Operation-only_body_for_query`

- Metaclass: `Operation`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `bodyCondition <> null implies isQuery`.

#### `OperationTemplateParameter-match_default_signature`

- Metaclass: `OperationTemplateParameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `default->notEmpty() implies (default.oclIsKindOf(Operation) and (let defaultOp : Operation = default.oclAsType(Operation) in defaultOp.ownedParameter->size() = parameteredElement.ownedParameter->size() and Sequence{1.. defaultOp.ownedParameter->size()}->forAll( ix | let p1: Parameter = defaultOp.ownedParameter->at(ix), p2 : Parameter = parameteredElement.ownedParameter->at(ix) in p1.type = p2.type and p1.upper = p2.upper and p1.lower = p2.lower and p1.direction = p2.direction and p1.isOrdered = p2.isOrdered and p1.isUnique = p2.isUnique)))`.

#### `Parameter-in_and_out`

- Metaclass: `Parameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(effect = ParameterEffectKind::delete implies (direction = ParameterDirectionKind::_'in' or direction = ParameterDirectionKind::inout)) and (effect = ParameterEffectKind::create implies (direction = ParameterDirectionKind::out or direction = ParameterDirectionKind::inout or direction = ParameterDirectionKind::return))`.

#### `Parameter-not_exception`

- Metaclass: `Parameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `isException implies (direction <> ParameterDirectionKind::_'in' and direction <> ParameterDirectionKind::inout)`.

#### `Parameter-connector_end`

- Metaclass: `Parameter`. Estado: **P**. Evidência: E09.
- OCL publicado: `end->notEmpty() implies collaboration->notEmpty()`.

#### `Parameter-reentrant_behaviors`

- Metaclass: `Parameter`. Estado: **P**. Evidência: E09.
- OCL publicado: `(isStream and behavior <> null) implies not behavior.isReentrant`.

#### `Parameter-stream_and_exception`

- Metaclass: `Parameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `not (isException and isStream)`.

#### `Parameter-object_effect`

- Metaclass: `Parameter`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(type.oclIsKindOf(DataType)) implies (effect = null)`.

#### `ParameterSet-same_parameterized_entity`

- Metaclass: `ParameterSet`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `parameter->forAll(p1, p2 | self.owner = p1.owner and self.owner = p2.owner and p1.direction = p2.direction)`.

#### `ParameterSet-input`

- Metaclass: `ParameterSet`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `((parameter->exists(direction = ParameterDirectionKind::_'in')) implies behavioralFeature.ownedParameter->select(p | p.direction = ParameterDirectionKind::_'in' and p.parameterSet->isEmpty())->forAll(isStream)) and ((parameter->exists(direction = ParameterDirectionKind::out)) implies behavioralFeature.ownedParameter->select(p | p.direction = ParameterDirectionKind::out and p.parameterSet->isEmpty())->forAll(isStream))`.

#### `ParameterSet-two_parameter_sets`

- Metaclass: `ParameterSet`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `parameter->forAll(parameterSet->forAll(s1, s2 | s1->size() = s2->size() implies s1.parameter->exists(p | not s2.parameter->includes(p))))`.

#### `Property-subsetting_context_conforms`

- Metaclass: `Property`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `subsettedProperty->notEmpty() implies (subsettingContext()->notEmpty() and subsettingContext()->forAll (sc | subsettedProperty->forAll(sp | sp.subsettingContext()->exists(c | sc.conformsTo(c)))))`.

#### `Property-derived_union_is_read_only`

- Metaclass: `Property`. Estado: **R**. Evidência: E06.
- OCL publicado: `isDerivedUnion implies isReadOnly`.

#### `Property-multiplicity_of_composite`

- Metaclass: `Property`. Estado: **P**. Evidência: E05.
- OCL publicado: `isComposite and association <> null implies opposite.upperBound() <= 1`.

#### `Property-redefined_property_inherited`

- Metaclass: `Property`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(redefinedProperty->notEmpty()) implies (redefinitionContext->notEmpty() and redefinedProperty->forAll(rp| ((redefinitionContext->collect(fc| fc.allParents()))->asSet())->collect(c| c.allFeatures())->asSet()->includes(rp)))`.

#### `Property-subsetting_rules`

- Metaclass: `Property`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `subsettedProperty->forAll(sp | self.type.conformsTo(sp.type) and ((self.upperBound()->notEmpty() and sp.upperBound()->notEmpty()) implies self.upperBound() <= sp.upperBound() ))`.

#### `Property-binding_to_attribute`

- Metaclass: `Property`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(self.isAttribute() and (templateParameterSubstitution->notEmpty()) implies (templateParameterSubstitution->forAll(ts | ts.formal.oclIsKindOf(Property) and ts.formal.oclAsType(Property).isAttribute())))`.

#### `Property-derived_union_is_derived`

- Metaclass: `Property`. Estado: **R**. Evidência: E06.
- OCL publicado: `isDerivedUnion implies isDerived`.

#### `Property-deployment_target`

- Metaclass: `Property`. Estado: **P**. Evidência: E25; commit `4e88d1b9f71f62f2d216130f99b5c978b4c1474a`.
- OCL publicado: `deployment->notEmpty() implies owner.oclIsKindOf(Node) and Node.allInstances()->exists(n | n.part->exists(p | p = self))`.

#### `Property-subsetted_property_names`

- Metaclass: `Property`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `subsettedProperty->forAll(sp | sp.name <> name)`.

#### `Property-type_of_opposite_end`

- Metaclass: `Property`. Estado: **P**. Evidência: E05.
- OCL publicado: `(opposite->notEmpty() and owningAssociation->isEmpty()) implies classifier = opposite.type`.

#### `Property-qualified_is_association_end`

- Metaclass: `Property`. Estado: **R**. Evidência: E06.
- OCL publicado: `qualifier->notEmpty() implies association->notEmpty()`.

#### `RedefinableElement-redefinition_consistent`

- Metaclass: `RedefinableElement`. Estado: **P**. Evidência: E11.
- OCL publicado: `redefinedElement->forAll(re | re.isConsistentWith(self))`.

#### `RedefinableElement-non_leaf_redefinition`

- Metaclass: `RedefinableElement`. Estado: **P**. Evidência: E11.
- OCL publicado: `redefinedElement->forAll(re | not re.isLeaf)`.

#### `RedefinableElement-redefinition_context_valid`

- Metaclass: `RedefinableElement`. Estado: **P**. Evidência: E11.
- OCL publicado: `redefinedElement->forAll(re | self.isRedefinitionContextValid(re))`.

#### `RedefinableTemplateSignature-redefines_parents`

- Metaclass: `RedefinableTemplateSignature`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `classifier.allParents()->forAll(c | c.ownedTemplateSignature->notEmpty() implies self->closure(extendedSignature)->includes(c.ownedTemplateSignature))`.

### SimpleClassifiers: 5 invariantes diretas

#### `BehavioredClassifier-class_behavior`

- Metaclass: `BehavioredClassifier`. Estado: **P**. Evidência: E16; commit `d1acb16d7499a2437e25f5f17828acfbc8c174a2`.
- OCL publicado: `classifierBehavior->notEmpty() implies classifierBehavior.specification->isEmpty()`.

#### `Enumeration-immutable`

- Metaclass: `Enumeration`. Estado: **P**. Evidência: E14; implementação no working tree, publicação ainda não atribuída a um commit.
- OCL publicado: `ownedAttribute->forAll(isReadOnly)`.

#### `Interface-visibility`

- Metaclass: `Interface`. Estado: **R**. Evidência: E04.
- OCL publicado: `feature->forAll(visibility = VisibilityKind::public)`.

#### `Reception-same_name_as_signal`

- Metaclass: `Reception`. Estado: **P**. Evidência: E09.
- OCL publicado: `name = signal.name`.

#### `Reception-same_structure_as_signal`

- Metaclass: `Reception`. Estado: **P**. Evidência: E09.
- OCL publicado: `signal.ownedAttribute->size() = ownedParameter->size() and Sequence{1..signal.ownedAttribute->size()}->forAll( i | ownedParameter->at(i).direction = ParameterDirectionKind::_'in' and ownedParameter->at(i).name = signal.ownedAttribute->at(i).name and ownedParameter->at(i).type = signal.ownedAttribute->at(i).type and ownedParameter->at(i).lowerBound() = signal.ownedAttribute->at(i).lowerBound() and ownedParameter->at(i).upperBound() = signal.ownedAttribute->at(i).upperBound() )`.

### StructuredClassifiers: 22 invariantes diretas

#### `Association-specialized_end_number`

- Metaclass: `Association`. Estado: **P**. Evidência: E05.
- OCL publicado: `parents()->select(oclIsKindOf(Association)).oclAsType(Association)->forAll(p | p.memberEnd->size() = self.memberEnd->size())`.

#### `Association-specialized_end_types`

- Metaclass: `Association`. Estado: **P**. Evidência: E05.
- OCL publicado: `Sequence{1..memberEnd->size()}-> forAll(i | general->select(oclIsKindOf(Association)).oclAsType(Association)-> forAll(ga | self.memberEnd->at(i).type.conformsTo(ga.memberEnd->at(i).type)))`.

#### `Association-binary_associations`

- Metaclass: `Association`. Estado: **P**. Evidência: E05.
- OCL publicado: `memberEnd->exists(aggregation <> AggregationKind::none) implies (memberEnd->size() = 2 and memberEnd->exists(aggregation = AggregationKind::none))`.

#### `Association-association_ends`

- Metaclass: `Association`. Estado: **P**. Evidência: E05.
- OCL publicado: `memberEnd->size() > 2 implies ownedEnd->includesAll(memberEnd)`.

#### `Association-ends_must_be_typed`

- Metaclass: `Association`. Estado: **P**. Evidência: E05.
- OCL publicado: `memberEnd->forAll(type->notEmpty())`.

#### `AssociationClass-cannot_be_defined`

- Metaclass: `AssociationClass`. Estado: **P**. Evidência: E15; commit `867c6afa2a99906936ec23a398a54278ebf5266e`.
- OCL publicado: `self.endType()->excludes(self) and self.endType()->collect(et|et.oclAsType(Classifier).allParents())->flatten()->excludes(self)`.

#### `AssociationClass-disjoint_attributes_ends`

- Metaclass: `AssociationClass`. Estado: **P**. Evidência: E05.
- OCL publicado: `ownedAttribute->intersection(ownedEnd)->isEmpty()`.
- Recorte comprovado por leitura: `associations.rs::owned_ends` rejeita
  memberEnd possuído com papel composto conflitante ou também ownedAttribute.
  `association_specialization_flow.rs` contém o contraexemplo `conflicting_role`
  e exige o diagnóstico `disjoint_attributes_ends`. O Unsupported final dos
  demais contratos especializados não apaga esse predicado já implementado.
  Isso não certifica toda AssociationClass nem todas as representações da regra.

#### `Class-passive_class`

- Metaclass: `Class`. Estado: **R**. Evidência: E04.
- OCL publicado: `not isActive implies (ownedReception->isEmpty() and classifierBehavior = null)`.

#### `CollaborationUse-client_elements`

- Metaclass: `CollaborationUse`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `roleBinding->collect(client)->forAll(ne1, ne2 | ne1.oclIsKindOf(ConnectableElement) and ne2.oclIsKindOf(ConnectableElement) and let ce1 : ConnectableElement = ne1.oclAsType(ConnectableElement), ce2 : ConnectableElement = ne2.oclAsType(ConnectableElement) in ce1.structuredClassifier = ce2.structuredClassifier) and roleBinding->collect(supplier)->forAll(ne1, ne2 | ne1.oclIsKindOf(ConnectableElement) and ne2.oclIsKindOf(ConnectableElement) and let ce1 : ConnectableElement = ne1.oclAsType(ConnectableElement), ce2 : ConnectableElement = ne2.oclAsType(ConnectableElement) in ce1.collaboration = ce2.collaboration)`.

#### `CollaborationUse-every_role`

- Metaclass: `CollaborationUse`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `type.collaborationRole->forAll(role | roleBinding->exists(rb | rb.supplier->includes(role)))`.

#### `CollaborationUse-connectors`

- Metaclass: `CollaborationUse`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `type.ownedConnector->forAll(connector | let rolesConnectedInCollab : Set(ConnectableElement) = connector.end.role->asSet(), relevantBindings : Set(Dependency) = roleBinding->select(rb | rb.supplier->intersection(rolesConnectedInCollab)->notEmpty()), boundRoles : Set(ConnectableElement) = relevantBindings->collect(client.oclAsType(ConnectableElement))->asSet(), contextClassifier : StructuredClassifier = boundRoles->any(true).structuredClassifier->any(true) in contextClassifier.ownedConnector->exists( correspondingConnector | correspondingConnector.end.role->forAll( role | boundRoles->includes(role) ) and (connector.type->notEmpty() and correspondingConnector.type->notEmpty()) implies connector.type->forAll(conformsTo(correspondingConnector.type)) ) )`.

#### `Component-no_nested_classifiers`

- Metaclass: `Component`. Estado: **P**. Evidência: E13; commit `62465b699f9ff9e8bd6da5a6aa2cdc1d73c13d53`.
- OCL publicado: `nestedClassifier->isEmpty()`.

#### `Component-no_packaged_elements`

- Metaclass: `Component`. Estado: **P**. Evidência: E13; commit `62465b699f9ff9e8bd6da5a6aa2cdc1d73c13d53`.
- OCL publicado: `nestingClass <> null implies packagedElement->isEmpty()`.

#### `Connector-types`

- Metaclass: `Connector`. Estado: **P**. Evidência: E17; commit `8da7a1d891791d2e9ea0462e95cdc5eac36f8afa`.
- OCL publicado: `type<>null implies let noOfEnds : Integer = end->size() in (type.memberEnd->size() = noOfEnds) and Sequence{1..noOfEnds}->forAll(i | end->at(i).role.type.conformsTo(type.memberEnd->at(i).type))`.

#### `Connector-roles`

- Metaclass: `Connector`. Estado: **P**. Evidência: E18; commit `17e237ada11fe92e4689b77bdae9f10b7787cd4d`.
- OCL publicado: `structuredClassifier <> null and end->forAll( e | structuredClassifier.allRoles()->includes(e.role) or e.role.oclIsKindOf(Port) and structuredClassifier.allRoles()->includes(e.partWithPort))`.

#### `ConnectorEnd-role_and_part_with_port`

- Metaclass: `ConnectorEnd`. Estado: **P**. Evidência: E20; limites e gate na seção ConnectorEnd/deployed_elements.
- OCL publicado: `partWithPort->notEmpty() implies (role.oclIsKindOf(Port) and partWithPort.type.oclAsType(Namespace).member->includes(role))`.

#### `ConnectorEnd-part_with_port_empty`

- Metaclass: `ConnectorEnd`. Estado: **P**. Evidência: E20; limites e gate na seção ConnectorEnd/deployed_elements.
- OCL publicado: `(role.oclIsKindOf(Port) and role.owner = connector.owner) implies partWithPort->isEmpty()`.

#### `ConnectorEnd-multiplicity`

- Metaclass: `ConnectorEnd`. Estado: **P**. Evidência: E22; commit `211a333d2e4c0b9f468b884a3e77623d868364e2`.
- OCL publicado: `self.compatibleWith(definingEnd)`.

#### `ConnectorEnd-self_part_with_port`

- Metaclass: `ConnectorEnd`. Estado: **P**. Evidência: E20; limites e gate na seção ConnectorEnd/deployed_elements.
- OCL publicado: `partWithPort->notEmpty() implies not partWithPort.oclIsKindOf(Port)`.

#### `Port-port_aggregation`

- Metaclass: `Port`. Estado: **P**. Evidência: E07.
- OCL publicado: `aggregation = AggregationKind::composite`.

#### `Port-default_value`

- Metaclass: `Port`. Estado: **P**. Evidência: E07.
- OCL publicado: `type.oclIsKindOf(Interface) implies defaultValue->isEmpty()`.

#### `Port-encapsulated_owner`

- Metaclass: `Port`. Estado: **P**. Evidência: E12; commit `62465b699f9ff9e8bd6da5a6aa2cdc1d73c13d53`.
- OCL publicado: `owner = encapsulatedClassifier`.

### Packages: 14 invariantes diretas

#### `Extension-non_owned_end`

- Metaclass: `Extension`. Estado: **U**. Evidência: U01.
- OCL publicado: `metaclassEnd()->notEmpty() and metaclassEnd().type.oclIsKindOf(Class)`.

#### `Extension-is_binary`

- Metaclass: `Extension`. Estado: **U**. Evidência: U01.
- OCL publicado: `memberEnd->size() = 2`.

#### `ExtensionEnd-multiplicity`

- Metaclass: `ExtensionEnd`. Estado: **P**. Evidência: E26; commit `6188ddb14d09d2641fa188c58087f5056de5d18a`.
- OCL publicado: `(lowerBound() = 0 or lowerBound() = 1) and upperBound() = 1`.

#### `ExtensionEnd-aggregation`

- Metaclass: `ExtensionEnd`. Estado: **P**. Evidência: E26; commit `6188ddb14d09d2641fa188c58087f5056de5d18a`.
- OCL publicado: `self.aggregation = AggregationKind::composite`.

#### `Package-elements_public_or_private`

- Metaclass: `Package`. Estado: **P**. Evidência: E01.
- OCL publicado: `packagedElement->forAll(e | e.visibility<> null implies e.visibility = VisibilityKind::public or e.visibility = VisibilityKind::private)`.

#### `Profile-metaclass_reference_not_specialized`

- Metaclass: `Profile`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `metaclassReference.importedElement-> select(c | c.oclIsKindOf(Classifier) and (c.oclAsType(Classifier).allParents()->collect(namespace)->includes(self)))->isEmpty() and packagedElement-> select(oclIsKindOf(Classifier))->collect(oclAsType(Classifier).allParents())-> intersection(metaclassReference.importedElement->select(oclIsKindOf(Classifier))->collect(oclAsType(Classifier)))->isEmpty()`.

#### `Profile-references_same_metamodel`

- Metaclass: `Profile`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `metamodelReference.importedPackage.elementImport.importedElement.allOwningPackages()-> union(metaclassReference.importedElement.allOwningPackages() )->notEmpty()`.

#### `Stereotype-binaryAssociationsOnly`

- Metaclass: `Stereotype`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `ownedAttribute.association->forAll(memberEnd->size()=2)`.

#### `Stereotype-generalize`

- Metaclass: `Stereotype`. Estado: **R**. Evidência: E03.
- OCL publicado: `allParents()->forAll(oclIsKindOf(Stereotype)) and Classifier.allInstances()->forAll(c | c.allParents()->exists(oclIsKindOf(Stereotype)) implies c.oclIsKindOf(Stereotype))`.

#### `Stereotype-name_not_clash`

- Metaclass: `Stereotype`. Estado: **A**. Evidência: revisão individual pendente.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: Stereotype names should not clash with keyword names for the extended model element.

#### `Stereotype-associationEndOwnership`

- Metaclass: `Stereotype`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `ownedAttribute ->select(association->notEmpty() and not association.oclIsKindOf(Extension) and not type.oclIsKindOf(Stereotype)) ->forAll(opposite.owner = association)`.

#### `Stereotype-base_property_upper_bound`

- Metaclass: `Stereotype`. Estado: **A**. Evidência: revisão individual pendente.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: The upper bound of base-properties is exactly 1.

#### `Stereotype-base_property_multiplicity_single_extension`

- Metaclass: `Stereotype`. Estado: **A**. Evidência: revisão individual pendente.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: If a Stereotype extends only one metaclass, the multiplicity of the corresponding base-property shall be 1..1.

#### `Stereotype-base_property_multiplicity_multiple_extension`

- Metaclass: `Stereotype`. Estado: **A**. Evidência: revisão individual pendente.
- Corpo OCL: ausente nesta constraint do XMI; não inventar fórmula.
- Descrição normativa do XMI: If a Stereotype extends more than one metaclass, the multiplicity of the corresponding base-properties shall be [0..1]. At any point in time, only one of these base-properties can contain a metaclass instance during runtime.

### Deployments: 4 invariantes diretas

#### `CommunicationPath-association_ends`

- Metaclass: `CommunicationPath`. Estado: **P**. Evidência: E24; commit `f152917ce28279467e1339e9b683ac846bd16cd7`.
- OCL publicado: `endType->forAll (oclIsKindOf(DeploymentTarget))`.

#### `DeploymentSpecification-deployment_target`

- Metaclass: `DeploymentSpecification`. Estado: **P**. Evidência: E19; commit `17e237ada11fe92e4689b77bdae9f10b7787cd4d`.
- OCL publicado: `deployment->forAll (location.oclIsKindOf(ExecutionEnvironment))`.

#### `DeploymentSpecification-deployed_elements`

- Metaclass: `DeploymentSpecification`. Estado: **P**. Evidência: E21; limites e gate na seção ConnectorEnd/deployed_elements.
- OCL publicado: `deployment->forAll (location.deployedElement->forAll (oclIsKindOf(Component)))`.

#### `Node-internal_structure`

- Metaclass: `Node`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `part->forAll(oclIsKindOf(Node))`.
- Decisão normativa pendente: a prosa exige partes de tipo Node, mas o OCL
  publicado testa se cada parte é Node. StructuredClassifier::part é uma
  coleção de Property, tornando as duas leituras distintas. Não substituir
  silenciosamente o corpo por `part->forAll(type.oclIsKindOf(Node))`, nem usar
  nestedNode como se fosse part. A linha permanece N até explicitar a decisão
  e sua base normativa; não foi implementada por este incremento.

### Values: 11 invariantes diretas

#### `Duration-no_expr_requires_observation`

- Metaclass: `Duration`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `expr = null implies (observation->size() = 1 and observation->forAll(oclIsKindOf(DurationObservation)))`.

#### `DurationConstraint-first_event_multiplicity`

- Metaclass: `DurationConstraint`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `if (constrainedElement->size() = 2) then (firstEvent->size() = 2) else (firstEvent->size() = 0) endif`.

#### `DurationConstraint-has_one_or_two_constrainedElements`

- Metaclass: `DurationConstraint`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `constrainedElement->size() = 1 or constrainedElement->size()=2`.

#### `DurationObservation-first_event_multiplicity`

- Metaclass: `DurationObservation`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `if (event->size() = 2) then (firstEvent->size() = 2) else (firstEvent->size() = 0) endif`.

#### `OpaqueExpression-language_body_size`

- Metaclass: `OpaqueExpression`. Estado: **P**. Evidência: E08.
- OCL publicado: `language->notEmpty() implies (_'body'->size() = language->size())`.

#### `OpaqueExpression-one_return_result_parameter`

- Metaclass: `OpaqueExpression`. Estado: **P**. Evidência: E08.
- OCL publicado: `behavior <> null implies behavior.ownedParameter->select(direction=ParameterDirectionKind::return)->size() = 1`.

#### `OpaqueExpression-only_in_or_return_parameters`

- Metaclass: `OpaqueExpression`. Estado: **P**. Evidência: E08.
- OCL publicado: `behavior <> null implies behavior.ownedParameter->forAll(not isStream and (direction=ParameterDirectionKind::in or direction=ParameterDirectionKind::return))`.

#### `StringExpression-operands`

- Metaclass: `StringExpression`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `operand->forAll (oclIsKindOf (LiteralString))`.

#### `StringExpression-subexpressions`

- Metaclass: `StringExpression`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `if subExpression->notEmpty() then operand->isEmpty() else operand->notEmpty() endif`.

#### `TimeConstraint-has_one_constrainedElement`

- Metaclass: `TimeConstraint`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `constrainedElement->size() = 1`.

#### `TimeExpression-no_expr_requires_observation`

- Metaclass: `TimeExpression`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `expr = null implies (observation->size() = 1 and observation->forAll(oclIsKindOf(TimeObservation)))`.

## Plano executável e dependências

### Lote 0: fechar a rastreabilidade

Completar as linhas A e associar cada obrigação a função, diagnóstico e testes
positivos/negativos/não verificados. Criar registros separados para corpos de
operações necessários à prova, sem somá-los como invariantes extras. Verificar
multiplicidades, subsets, redefines e uniões derivadas separadamente. Não usar
contagem de testes nem de descritores como cobertura normativa.

### Lote 1: ownership e predicados locais, fechamento parcial publicado

Port e as duas regras de Component já têm implementação e gates descritos em
E12/E13. Enumeration (item 4) tem implementação e gate no working tree em E14,
ainda aguardando referência ao commit publicado. BehavioredClassifier (item 5)
tem recorte publicado e gates em E16. Os cinco itens têm predicados no recorte,
mas persistem limites de representação, autoria e rastreabilidade; não converter
essa contagem em conformidade integral.

1. `Port-encapsulated_owner`: derivar encapsulatedClassifier a partir do owner
   e dos papéis canônicos; não confiar numa cópia armazenada da inversa.
2. `Component-no_nested_classifiers`: provar coleção vazia, não apenas ausência
   de um campo JSON. Usar classificação de papéis de ownership.
3. `Component-no_packaged_elements`: aplicar a implicação apenas quando existe
   nestingClass; um Component não aninhado pode ter packagedElements.
4. `Enumeration-immutable`: todo ownedAttribute deve ser read-only, incluindo
   default efetivo; distinguir atributo de qualquer filho possuído.
5. `BehavioredClassifier-class_behavior`: quando há classifierBehavior, sua
   specification deve estar vazia. Não confundir com Class::passive_class.

Usar as estruturas existentes, sem catálogo paralelo nem inferências baseadas
em nomes. Os corpos exatos estão no inventário. Resultados desconhecidos não
viram vazio/false; erro estrutural não desaparece diante de outro unknown.

### Lote 2: conjuntos e correspondência posicional

AssociationClass::cannot_be_defined já tem recorte publicado em E15 e
AssociationClass::disjoint_attributes_ends tem recorte em E05. Fechar os limites
e contratos restantes sem remover Unsupported prematuramente. Continuar com
O fechamento dos limites de CommunicationPath (E24) e ConnectorEnd; três invariantes
de ConnectorEnd têm recorte em E20, multiplicity/definingEnd em E22. Connector::roles tem recorte publicado em
E18, incluindo allRoles e membership conservador; fechar os limites restantes.
Connector::types já tem recorte publicado em E17; preservar seus limites e testar as dependências
adicionadas sem transformar a implicação de type em validação de todos os roles.
Dependências:
allRoles, membership, conformance, definingEnd e compatibleWith. Testar aridade,
ordem das pontas, partWithPort, type opcional, herança e referências externas.
Implementar os contratos especializados antes de remover Unsupported.

### Lote 3: bindings e templates

TemplateBinding, TemplateParameterSubstitution, argumentos de
ClassifierTemplateParameter, OperationTemplateParameter e
RedefinableTemplateSignature. Dependências: formal/actual canônicos,
isCompatibleWith, assinaturas e isSubstitutableFor. Não extrapolar a implementação
atual do parameteredElement para actual arguments.

### Lote 4: deployment e instâncias

DeploymentSpecification::deployment_target tem recorte publicado em E19.
DeploymentSpecification::deployed_elements tem recorte em E21 e
Property::deployment_target em E25.
Decidir a contradição OCL/prosa de Node::internal_structure antes de implementar.
Continuar InstanceSpecification e Property relacionados a deployment; completar
specification/values e slots. O caminho
`instances.rs::validate_instance` declara explicitamente que valores de
InstanceSpecification::specification ainda não são avaliados.

### Lote 5: profiles e extensões

ExtensionEnd tem predicados locais publicados (E26), mas Extension mantém
Unsupported. Continuar Extension, limites de ExtensionEnd, referências de
Profile e demais constraints de Stereotype
e aplicações. Stereotype::generalize existente é apenas uma obrigação.
Semântica do modelo é distinta de renderização, edição visual e intercâmbio XMI.

## Provas e gates exigidos

Para cada obrigação promovida, identificar ao menos um modelo positivo e um
contraexemplo discriminante contra uma implementação superficial. Conforme a
regra, incluir default omitido, null permitido/proibido, referência inexistente,
tipo errado, referência externa, coleções mistas, duplicatas e ciclos.
Verificar precedência de malformados/violações diante de desconhecidos em ambas
as ordens, validação com/sem raiz quando aplicável, comandos atômicos, correção
de drafts e roundtrip JSON/Protobuf. Para estado compartilhado, repetir A-B-A e
chamadas concorrentes. Não exigir propriedade falsa de equivalência entre
validação completa de ownership e drafts incompletos.

`Model::validate_contracts` reúne os passes semânticos comuns em `root.rs`;
`validate_structural_model` também chama regras estruturais e templates.
Adicionar uma regra apenas num comando não valida importados nem a API pública.

Comandos propostos, ainda não executados por esta auditoria:

```sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
python3 script/bellatrix_mde/bellatrix_uml_catalog.py --check
cargo check --offline --locked -p bellatrix_mde
cargo test --offline --locked -p bellatrix_mde \
  --test port_aggregation --test port_default_value \
  --test property_invariants_flow --test ownership_namespace_flow \
  --test kernel_api_validation --test template_signatures \
  --test association_kernel --test association_specialization_flow \
  --test inherited_capabilities --test kb09_redefinitions
# Acrescentar os novos alvos do lote, depois de confirmar seus nomes finais.
git diff --check
```

Registrar toolchain, comandos, resultado, commit/base e fingerprint da árvore
realmente testada. Reexecutar os alvos afetados depois de correções. Não atribuir
antecipadamente o SHA de publicação a um working tree mutável. Não executar UI
ou geração para comprovar este recorte do validador.

## Pendências explícitas desta matriz

- As linhas A ainda exigem cruzamento individual; não são lista de ausências.
- R/P não são alegações de equivalência integral nem evidência de execução nova.
- Os 127 IDs diretos não fecham dependências herdadas ou de outros pacotes.
- Contratos de propriedades e corpos de operações precisam de matriz própria
  vinculada, sem confundir sua contagem com invariantes.
- O relatório público não enumera automaticamente todas as obrigações nunca
  executadas. `implemented_rules_only` deve permanecer até haver justificativa
  verificável para uma afirmação mais forte.
- Aceite de UI, sintaxe concreta e interchange XMI/UMLDI são eixos separados.
