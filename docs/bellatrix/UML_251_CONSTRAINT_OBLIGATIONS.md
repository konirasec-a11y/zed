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
33/33 e limites de suporte descritos em E15.
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
de execução nova. E12/E13/E14/E15/E16 possuem os gates delimitados nas seções seguintes.

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
  AssociationClass (E05/E15) nem autoriza autoria completa.
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

- Metaclass: `Property`. Estado: **A**. Evidência: revisão individual pendente.
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

- Metaclass: `Connector`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `type<>null implies let noOfEnds : Integer = end->size() in (type.memberEnd->size() = noOfEnds) and Sequence{1..noOfEnds}->forAll(i | end->at(i).role.type.conformsTo(type.memberEnd->at(i).type))`.

#### `Connector-roles`

- Metaclass: `Connector`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `structuredClassifier <> null and end->forAll( e | structuredClassifier.allRoles()->includes(e.role) or e.role.oclIsKindOf(Port) and structuredClassifier.allRoles()->includes(e.partWithPort))`.

#### `ConnectorEnd-role_and_part_with_port`

- Metaclass: `ConnectorEnd`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `partWithPort->notEmpty() implies (role.oclIsKindOf(Port) and partWithPort.type.oclAsType(Namespace).member->includes(role))`.

#### `ConnectorEnd-part_with_port_empty`

- Metaclass: `ConnectorEnd`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `(role.oclIsKindOf(Port) and role.owner = connector.owner) implies partWithPort->isEmpty()`.

#### `ConnectorEnd-multiplicity`

- Metaclass: `ConnectorEnd`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `self.compatibleWith(definingEnd)`.

#### `ConnectorEnd-self_part_with_port`

- Metaclass: `ConnectorEnd`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
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

- Metaclass: `ExtensionEnd`. Estado: **A**. Evidência: revisão individual pendente.
- OCL publicado: `(lowerBound() = 0 or lowerBound() = 1) and upperBound() = 1`.

#### `ExtensionEnd-aggregation`

- Metaclass: `ExtensionEnd`. Estado: **A**. Evidência: revisão individual pendente.
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

- Metaclass: `CommunicationPath`. Estado: **U**. Evidência: U01.
- OCL publicado: `endType->forAll (oclIsKindOf(DeploymentTarget))`.

#### `DeploymentSpecification-deployment_target`

- Metaclass: `DeploymentSpecification`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `deployment->forAll (location.oclIsKindOf(ExecutionEnvironment))`.

#### `DeploymentSpecification-deployed_elements`

- Metaclass: `DeploymentSpecification`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `deployment->forAll (location.deployedElement->forAll (oclIsKindOf(Component)))`.

#### `Node-internal_structure`

- Metaclass: `Node`. Estado: **N**. Evidência: sem predicado dedicado identificado nos passes inspecionados.
- OCL publicado: `part->forAll(oclIsKindOf(Node))`.

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
CommunicationPath, Connector e ConnectorEnd. Dependências:
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

Node, DeploymentSpecification, InstanceSpecification e Property relacionados a
deployment; completar specification/values e slots. O caminho
`instances.rs::validate_instance` declara explicitamente que valores de
InstanceSpecification::specification ainda não são avaliados.

### Lote 5: profiles e extensões

Extension/ExtensionEnd, referências de Profile, demais constraints de Stereotype
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
