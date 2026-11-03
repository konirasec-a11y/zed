# Plano mestre de implementação MDE — integração e reaproveitamento

**Consolidado em 2026-11-03.** Este é o plano canônico de execução, não
uma substituição do objetivo LOW CODE por um editor UML. Preserva os onze
passos do plano revisado original, incorpora os requisitos detalhados do
primeiro plano e aplica as correções posteriores do usuário.

**Resultado esperado:** criar um projeto pela interface, modelar, configurar
alvo e dependências, escrever código específico, gerar, compilar/testar e
evoluir uma aplicação sem perder autoria nem sair do ambiente. A CLI serve
para testes e automação, não como etapa manual obrigatória desse percurso.

## Arquitetura, marcos e precedência

- **Bellatrix:** plataforma geral; MDE é uma engine, não toda a plataforma.
- **Zed:** host de desenvolvimento e edição, preservado com suas capacidades.
- **MDE:** autoridade única de modelagem, validação e transformação.
- **UI Bellatrix:** contribuições nativas sobre contratos existentes, sem
  nova IDE, Project manager, Dock, EngineManager ou EventBus paralelo.
- **Cordis:** composição/lifecycle e eventos de domínio; **ProjectService:**
  identidades e revisões semânticas; **Tera:** renderização de dados resolvidos.

**M1 — ferramenta de diagramas estruturais utilizável:** primeiro marco
pedido pelo usuário. Autoria real desde diagramas vazios, regras e notações
oficiais, edição, UI/UX e persistência; fixtures não bastam. Cobrir Classes,
Objetos, Pacotes, Componentes, Estrutura Composta, Implantação e Perfis,
registrando limitações por requisito. Priorizar o suporte necessário à MDE
não reduz a especificação ao conjunto de exemplos ou símbolos existentes.

**M2 — aplicação LOW CODE completa em Rust:** projeto, autoria específica,
configuração, M2M/M2T, regeneração determinística e execução pela UI, comprovados pelo
percurso Customer do passo 11. M1 não substitui M2.

**M3 — extensibilidade e outros alvos:** Rust primeiro; depois Zig, C#,
Kotlin e Flutter, conforme decisão do usuário. Prever contratos agora,
sem anunciar implementações inexistentes. Provar extensibilidade com pelo
menos duas contribuições independentes e preservar a cobertura UML além
do primeiro marco estrutural. TGG/sincronização bidirecional genérica fica
condicionada a necessidade demonstrada, não bloqueia geração unidirecional.

Os contratos técnicos de [ASG/M2M/M2T e revisão UI/UX](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md),
[reaproveitamento Zed](protocol-and-persistence.md),
[relações](GRAPH_RELATIONSHIPS.md), [geração Tera](TERA_GENERATION_PIPELINE.md)
e [marco estrutural](STRUCTURAL_UML_MILESTONE.md) detalham este plano;
não são planos gerais concorrentes. A matriz UX-01–UX-09 já existente
continua vinculante no passo 9, sem duplicar seu acompanhamento.

**Correções que prevalecem:** reaproveitamento substitui construir uma
infraestrutura de desenvolvimento dentro da MDE; M1 precede o caso completo
Rust; revisão UI/UX é aditiva, não redefine domínio ou objetivo. A política
beta canônica **sem aliases/migração legada** substitui a promessa original
de carregar formatos antigos Bellatrix. Rejeitar formato não suportado com
diagnóstico não autoriza excluir ou sobrescrever arquivos fora da saída gerada autorizada.
Formatos/APIs nativos Zed não são alterados por essa decisão. Obrigações
oficiais de intercâmbio XMI/DI são independentes dessa política beta.

**Frente visual paralela (2026-11-03):** completar o repertório de elementos,
notações e ferramentas de criação dos 14 diagramas, reaproveitando os
componentes existentes e respeitando o tema da IDE. O Kernel continua
prioritário e os sete estruturais continuam sendo a primeira entrega.
Distinguir explicitamente notação renderizada, criação/edição funcional e
semântica validada; símbolos de galeria não comprovam suporte no canvas.
Elementos dependentes de ownership ou regras ainda ausentes devem ter
limitações identificadas, sem produzir modelos inválidos silenciosamente.

**Contrato comum de apresentação:** todos os elementos visuais compartilham
as mesmas características de estilo por View, independentes do Kernel e da
validação semântica. Cor, fonte, borda, opacidade, alinhamento horizontal/vertical
de texto e ordem de sobreposição pertencem à visualização, não ao elemento UML.
Oferecer trazer à frente/enviar para trás e avançar/recuar uma camada. Usar
ícones reconhecíveis, descrição ao passar o mouse e nomes acessíveis, com
aplicação em tempo real e herança do tema da IDE. Geometria/notação específica
não autoriza duplicar contratos de estilo por tipo.

**Organização visual e validação configurável:** a árvore do Model Explorer
é projetada a partir do projeto e pode registrar organização visual explícita
para elementos e diagramas sem alterar propriedade UML nem confundir
organização com contexto UMLDI. A edição visual não deve ser proibida por
regras UML ainda ausentes: guardar representações provisórias sem fingir que
foram validadas. O usuário escolhe validar a cada mudança, ao salvar ou por
comando manual. O front envia o snapshot ao motor UML, apresenta diagnósticos
e associa resultados à revisão; regras não implementadas permanecem como
não verificadas. A validação e suas regras pertencem ao motor.

**Fronteira do motor e composição:** a engine recebe em memória o grafo, a
configuração e os dados de templates já resolvidos; não conhece caminhos
físicos e não acessa arquivos. Mapear metaclasses, nós e relações para funções
de transformação reutilizáveis, com dependências explícitas, ordenação
determinística e diagnósticos para ciclos de execução ou mapeamentos ausentes.
Ciclos legítimos do modelo UML não são automaticamente ciclos de execução.
A saída é um grafo/plano em memória; adaptadores externos carregam templates
de diretório ou SQLite. A engine compõe as funções necessárias e gera um único
script de construção, em Rust, Python, Julia, F# ou outra linguagem escolhida.
A execução posterior desse script é que materializa todo o projeto físico.
A engine não acessa o sistema de arquivos nem materializa o projeto diretamente.
O próprio grafo é a representação intermediária: grafos UML e de saída podem
compartilhar infraestrutura, preservando contratos distintos. Cada operação de
transformação declara entradas e saídas tipadas; a composição verifica as
ligações, entradas ausentes e ciclos reais de dependência, e ordena as funções
com desempate estável. A linguagem do script pode diferir da linguagem do
projeto gerado. Fable é referência de separação de passes e backends, não uma
exigência de AST adicional nem de dependência do compilador Fable.

## 1. Fixar a base e consolidar as especificações

Auditar o resultado real da consolidação existente antes de acrescentar
código: registrar SHA, diferenças, testes e lacunas. Não iniciar outra
consolidação concorrente nem fixar um HEAD histórico como requisito futuro.

Fechar quatro contratos persistentes:

- **UML:** elementos, propriedades, relações, perfis, restrições e notação.
- **Projeto:** identidade/revisão, modelos, linguagens-alvo, autoria,
  dependências, configuração e artefatos.
- **Transformação:** entradas/saídas, regras versionadas, correspondências,
  pré/pós-condições e diagnósticos.
- **Autoria:** propriedade do usuário/gerador, preservação e conflitos.

Reaproveitamento antes de código novo; protos primeiro, investigação em
cascata, depois Actions/eventos/APIs/crates. Mudanças concentradas em
`bellatrix*`; UI Zed somente onde indispensável, núcleo como último recurso.
Nenhuma perda do fluxo normal de edição é aceitável.

**Entrega/aceite P01:** matriz `requisito → código existente → integração →
lacuna comprovada → teste/evidência`, com responsáveis e dependências.
Estados distintos: especificado, implementado, testado, aceito; bloqueado
ou não executado nunca equivale a aprovado. Os quatro contratos precisam
cobrir M1, M2 e a extensão futura, não apenas o Canvas.

## 2. Mapear o reaproveitamento, capacidade por capacidade

Para abrir documentos, editar código, salvar, buscar, diagnosticar, gerar,
compilar e executar:

1. Localizar mensagens/contratos nos protos Zed.
2. Seguir registro, handlers e chamadas até a implementação efetiva.
3. Verificar contexto, resultados, erros, cancelamento e limitações.
4. Sem contrato útil, investigar Actions, eventos locais e APIs nativas.
5. Integrar na menor superfície possível das crates Bellatrix.

RPC remoto, evento GPUI local, Action e evento Cordis não são mecanismos
intercambiáveis. `BufferVersion` não é `ProjectRevision`; protocolo de
colaboração não é barramento genérico nem persistência completa da aplicação.

**Entrega/aceite P02:** inventário por capacidade com contrato e ponto de
integração demonstrados. Nenhuma implementação nova sem evidência de que a
existente não atende. Usar Editor/Buffer e acrescentar vínculo semântico,
não construir editor, terminal, explorador de arquivos ou banco de sessão.

## 3. Fechar o domínio único da MDE

Manter em `bellatrix_mde` uma única autoridade para modelo/identidades,
metamodelo/perfis, elementos/tipos/propriedades/relações, diagramas/Views,
comandos/transações, validação/diagnósticos, especializações de linguagem,
transformações/traces e planejamento/serialização. Módulos internos não
são engines paralelas. ASG tipado e atribuído não se confunde com layout.

Canvas, Inspector, QuickEdit, CLI e geradores consomem o mesmo domínio.
Migrar regras espalhadas, não copiá-las em controles/templates. Cordis
coordena solicitações e revisões sem duplicar regras UML; adaptadores não
reconstroem ProjectService. Remover tipos duplicados e dependências
compiláveis de código mantido somente como referência, não dados autorais.

**Entrega/aceite P03:** domínio testável sem GPUI/Zed; mesmo comando produz
mesmo resultado pela UI ou headless. Comandos atômicos validam ownership,
referências, tipos e invariantes; revisão obsoleta e rejeição preservam
estado anterior. Consumidores/testes migram ao contrato canônico beta,
com rejeição explícita de legado e nenhuma destruição de documentos.

## 4. Concluir a normalização e validação UML 2.5.1

**Prioridade adotada em 2026-11-03: construir o motor UML Rust a partir dos
contratos normativos, não apenas um catálogo ou editor de figuras.**

1. Ler UML.xmi, UMLDI.xmi, PrimitiveTypes.xmi e StandardProfile.xmi, resolvendo
   identificadores e imports normativos transitivos com versão e proveniência.
2. Gerar a estrutura repetitiva em Rust: metaclasses, heranças, propriedades,
   tipos, referências, cardinalidades, defaults e descritores de restrições.
3. Consumir esses contratos no estado canônico, nos comandos e na validação.
   Implementar as regras semânticas e propriedades derivadas; texto OCL
   armazenado e structs sem consumidores não são regras executadas.
4. Fazer a UI consumir os mesmos contratos para campos, seletores e operações,
   com apresentação e notação orientadas também pelo PDF completo.

O núcleo do motor é o **grafo UML tipado canônico**. As definições extraídas
formam seu grafo de tipos/contratos; o modelo autorado é um grafo tipado por
esse metamodelo. Structs, traits, comandos, validação e Views acessam esse
estado, sem um modelo de objetos paralelo nem cópias autoritativas por painel.

Existem duas gerações distintas: **XMI OMG → estrutura do motor UML Rust**;
posteriormente **modelo UML autorado e validado → grafos de transformação →
código da aplicação**. Um grafo de armazenamento não substitui os grafos-alvo.
Só a revisão validada correspondente pode alimentar a geração; diagnósticos
impeditivos precisam ser resolvidos. Esta prioridade serve ao marco completo
UML estrutural e não exclui os contratos de primitivos, UMLDI ou perfis.


Usar a [especificação completa UML 2.5.1](https://www.omg.org/spec/UML/2.5.1/PDF)
e os originais versionados [UML.xmi](standards/uml/20161101/UML.xmi),
[UMLDI.xmi](standards/uml/20161101/UMLDI.xmi),
[PrimitiveTypes.xmi](standards/uml/20161101/PrimitiveTypes.xmi) e
[StandardProfile.xmi](standards/uml/20161101/StandardProfile.xmi).
Arquivos consumíveis por máquina prevalecem em conflitos, conforme cláusula
2. As cinco imagens são exemplos, não recorte normativo; tabelas/exemplos
B.6 não são exaustivos nem definem obrigatório/opcional.

Catálogo executável e rastreável: ID normativo/versão, generalizações,
propriedades e capacidades herdadas transitivamente, tipos, multiplicidade,
ordem/unicidade, ownership, opostos, subsets/redefinitions, propriedades
derivadas, consultas e restrições. Cada regra tem origem, ID estável,
implementação e exemplos válidos/inválidos. OCL traduzida requer testes de
equivalência; texto copiado não significa restrição executada.

Quatro camadas de validação, independentes de apresentação:

1. **Integridade:** IDs, referências e estrutura persistente.
2. **Conformidade UML:** metaclasses, propriedades, relações e perfis.
3. **Representação:** compatibilidade de Views, notações e diagramas.
4. **Prontidão do alvo:** dados/recursos para a transformação selecionada.

Validar modelo inteiro, inclusive relações não desenhadas. Permitir salvar
rascunhos com estados distintos: incompleto, inválido, recurso não suportado,
pronto para gerar. Bloquear transformação por suas pré-condições, não todo
salvamento. Limite do editor não vira proibição UML (por exemplo aridade de
associações). Glyph disponível não prova suporte funcional.

**Entrega/aceite P04:** matriz dos sete diagramas estruturais e de requisitos
da especificação completa, com lacunas explícitas e ampliação comportamental
sem alegação prematura. Testes positivos/negativos por regra e regressões
para herança, notação e relações ocultas. Não marcar como válido o que não
foi implementado. Os cinco eixos oficiais da cláusula 2 terão evidências
separadas no gate final, não um selo agregado obtido por testes visuais.

## 5. Integrar projetos e documentos ao Zed real

Reaproveitar Project, worktrees, ProjectPath, Buffer, Workspace Item e
SerializableItem/restauração conforme auditoria. Associar documento/revisão
MDE, arquivo/buffer real, diagramas abertos, estado visual e artefatos.
Model Explorer é semântico; Project Panel continua responsável por arquivos.
Sessão SQLite não é a única cópia canônica do projeto. Retirar shell de I/O
redundante apenas após demonstrar autoridade Zed e testar conflitos.

O contrato de projeto precisa representar, sem forçar tudo a classes UML:

- Pacotes, módulos, diretórios, arquivos e referências entre pacotes.
- Bibliotecas, executáveis e pontos de entrada.
- Dependências com versões, features e recursos exigidos pelas linguagens.
- Toolchain, edição/versão/dialeto e plataformas-alvo.
- Recursos estáticos e arquivos de configuração.
- Perfis de build, execução e teste.
- Versões de geradores, templates e regras de transformação.

Oferecer configuração pela UI, não uma classe fictícia chamada Cargo.toml.
Definir dirty/saved state, recarga/alteração externa, conflito de revisão,
salvar/cancelar e restauração de identidade/documento/viewport/seleção.
Deixar Dock persistir sua própria configuração; não duplicar estado.

A integração não pode substituir o registro nativo de Editor para Buffer,
nem gravar duas vezes o mesmo arquivo por caminhos de salvamento distintos.
Verificar o vínculo documento/Item por identidade, não somente pelo tipo
da aba. O auto-open atual de fixtures em Workspaces ainda exige correção;
a documentação histórica não comprova que essa integração foi concluída.

**Entrega/aceite P05:** abrir/editar/salvar/fechar/reabrir documento real e
reproduzir geração a partir do estado salvo, sem configurações ocultas da
máquina ou dependência exclusiva da fixture de startup. Repositório sem
MDE funciona sem painéis impostos. Headless mantém adaptador de armazenamento
sem depender de entidades GPUI. Intercâmbio XMI/DI exige roundtrip separado,
incluindo perfis e versões oficiais antes de qualquer declaração conforme.

## 6. Integrar código autoral e particularidades das linguagens

Usar Editor/Buffer do Zed para corpos de métodos, expressões, declarações
auxiliares, arquivos autorais, macros, atributos e anotações. Separar
semântica UML, especialização do alvo e conteúdo autoral; autoria específica
é requisito central LOW CODE, não correção manual externa ao produto.

Cada contribuição tem identidade, linguagem/versão/dialeto, proprietário,
contexto/posição de aplicação, conteúdo, revisão, correspondência com
arquivo/trecho e política de preservação. Descritores conhecidos informam
parâmetros, incompatibilidades, dependências/features e emissão. Distinguir
**definir macro** de **invocar macro**. Derive/atributo Rust pertence a item
específico; Data Annotation C# tem destinatário, argumentos e referências.
Recursos desconhecidos podem permanecer explícitos/opacos, sem alegar
validação completa. Usar OpaqueBehavior/OpaqueExpression/perfis onde cabem,
sem converter toda particularidade de linguagem em estereótipo.

Cada conteúdo tem **uma fonte de verdade: arquivo OU contribuição no modelo**.
Buffer edita essa fonte com contrato explícito de salvar, nunca duas cópias
editáveis divergentes. Reaproveitar LSP e diagnósticos com contexto válido
(worktree, dependências e módulo envolvente); linguagem atribuída a um
fragmento isolado não garante análise completa. Definir contexto/sourcemap
para fragmentos e declarar limites quando não houver análise confiável.

**Entrega/aceite P06:** pela UI editar corpo e macro/anotação com dependências,
salvar/reabrir/regenerar preservando conteúdo. Renome não reescreve código
opaco por adivinhação; manter diagnóstico e decisão/correção explícita.

## 7. Completar transformação e geração sem duplicar ferramentas

Pipeline obrigatório: **UML + especializações + configuração → M2M para
TargetGraph de código/infraestrutura + TraceGraph → plano de artefatos →
M2T/Tera → publicação controlada**. Metamodelos fonte/destino têm tipos e
versões próprios; coexistência de elementos `fs.*`/`rust.*` não prova M2M.

Cada regra declara identidade/versão, match, pré/pós-condições, restrições
negativas, preservações/consumos/criações, correspondências, conflitos e
diagnósticos. Formalizar semântica DPO/SPO somente onde implementada e
testada (inclusive dangling/gluing), sem alegar garantias algébricas por
renomear estruturas. TGG permanece condicional à sincronização bidirecional.

Reutilizar gerador Rust, templates Tera e publicação existentes onde
atenderem. Tera não valida UML, decide ownership ou resolve dependências.
Mapear explicitamente classes/propriedades/operações/associações/interfaces,
herança e ownership Rust; construção sem mapeamento produz diagnóstico,
não implementação inventada. Rastrear elementos/regras até artefatos e erros.

**Entrega/aceite P07:** entradas/versões iguais geram plano determinístico;
conflitos, pré-condições e recursos sem suporte aparecem antes da escrita.
Modelos-alvo e traces validáveis; testes de regras, plano, artefatos e build
Rust gerado. Ampliação futura para outros alvos usa os mesmos contratos,
não implementação fictícia inferida de um seletor de linguagem.

## 8. Gerar e regenerar de forma determinística

Nesta fase a geração é **unidirecional**: o modelo e a configuração são a
entrada; a saída gerada pode ser sobrescrita. Usar Git para versionamento e
recuperação, sem exigir reconciliação bidirecional, diff ou resolução de
conflitos como pré-requisito de geração. Não criar commits automaticamente.

Carregar os templates de cada linguagem de um diretório ou do SQLite e
renderizá-los com Tera.
O mesmo UML, a mesma configuração e a mesma versão dos templates devem gerar
exatamente o mesmo script e a mesma saída. Compor no script as funções para
criar diretórios e arquivos; sua execução externa materializa o projeto.
Restringir a escrita ao destino de geração definido e
validar caminhos; não tratar a autorização para sobrescrever a saída gerada
como autorização para apagar arquivos fora desse destino.

**Entrega/aceite P08:** a mesma entrada e configuração produzem os mesmos
arquivos e conteúdo; regenerar atualiza a saída definida, cria diretórios e
reporta falhas de escrita. O projeto pode ser versionado e recuperado via Git.
Sincronização do código editado de volta para UML não faz parte desta fase.

## Auditoria normativa e prioridade estrutural — 2026-11-03

Prioridade imediata: fechar o Kernel compartilhado do motor — Element,
NamedElement, Package, Class, Property, Operation e Association, com suas
heranças/dependências normativas — antes de ampliar outros diagramas ou a UI.
Reutilizar mecanismos gerais de grafo para tipagem, cardinalidades, conjuntos,
ciclos e transações; implementar as interpretações UML específicas com referência
ao PDF/XMI e ao código/testes revisados do uml4net. O upstream C# é referência
para portes rastreáveis, não outro runtime ou fonte de estado autoritativa.

Escopo final explicitado: regras para os 14 diagramas UML. **A primeira entrega
continua sendo exclusivamente os sete estruturais**: Classes, Componentes,
Estrutura Composta, Implantação, Objetos, Pacotes e Perfis. Atividades, Casos de
Uso, Máquinas de Estados, Sequência, Comunicação, Visão Geral de Interação e
Timing ficam para uma etapa posterior; não antecipar sua implementação nesta
primeira entrega nem contar metadados extraídos como suporte funcional.

A ordem do plano completo é preservada. A execução atual concentra P03/P04/P09
no marco de **todos os sete diagramas estruturais**, começando pelas dependências
do índice oficial: Root, Namespaces, Types/Values, Classifiers, Simple/Structured
Classifiers, Packages/Profiles e Deployments. Não expandir geração de código ou
outras aplicações enquanto esse marco permanece incompleto.

- [Auditoria dos contratos semânticos](UML_251_SEMANTIC_AUDIT.md).
- [Auditoria dos sete diagramas estruturais](UML_251_STRUCTURAL_AUDIT.md).
- APIs e contratos internos podem mudar: não há obrigação de retrocompatibilidade
  de código. Migrar os consumidores e preservar os fluxos funcionais e os dados
  autorais; não reintroduzir DTOs sem uso nem criar outra fonte de estado.
- PDF e XMI normativos definem os contratos. Catálogo de metaclasses, tipos Rust
  declarados, símbolos ou build verde não comprovam implementação das invariantes.
- Não declarar conformidade integral. Cada lacuna exige implementação, vínculo
  ao fluxo real e evidência focada; os resultados das auditorias são limitados
  às cláusulas e aos caminhos de código explicitamente examinados.

## 9. Montar a UI Bellatrix como contribuição nativa

Conectar Canvas/Toolbox, Model Explorer, Inspector/QuickEdit, Editor de
código, configuração de projeto/alvos/recursos e diagnósticos/geração ao
mesmo domínio. Navegar modelo ↔ código ↔ diagnóstico com correspondência
real; abas, Dock, foco, atalhos, buffers e edição continuam nativos Zed.

**Revisão UI/UX é parte deste passo, não substitui os demais.** Reutilizar
[painéis de design](../../crates/bellatrix_design_system/docs/design/src/components/panels/)
e [variantes visuais](../../crates/bellatrix_design_system/docs/design/preview/).
Executar a matriz UX-01–UX-09 no
[contrato técnico](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md): Working Diagrams,
catálogo/busca/disponibilidade, árvore semântica, Inspector compacto e
QuickEdit, Canvas/notação/geometria/rotas/curvas, acessibilidade/recuperação,
Dock, autoria desde vazio e gate nativo. Protótipo não autoriza mutações
silenciosas. Catálogo, descritores, capacidades herdadas e apresentação
compartilham fonte de verdade; não redefinem o domínio na UI.

Respeitar tema ativo da IDE, contraste alto e outline legível, sem paleta
fixa que invisibilize símbolos/seleção. Separar estilo/identidade/notação:
suprimir compartimento não apaga semântica; rótulos, setas, curvas manuais,
ancoragem, zoom e seleção preservam referências. Validar claro/escuro,
foco/teclado, erros, confirmar/cancelar e resize/collapse/restore do Dock.

Edição reativa é requisito de UX: alterações válidas nos campos e estilos
emitem os comandos existentes, e eventos/revisões atualizam Canvas, Explorer
e Inspector do mesmo documento sem botão Apply. Entrada intermediária
inválida permanece no controle com erro local, sem corromper o modelo.
Preservar foco, cursor e digitação mais recente durante confirmações
assíncronas; serializar/coalescer comandos sem ciclos de realimentação nem
sobrescrita silenciosa em conflito. Trocar documento não redireciona edições
pendentes. Salvar arquivo continua distinto de editar o modelo em memória.
MVC/MVVM são referências para avaliar responsabilidades, não exigência de
introduzir outro framework: reutilizar entidades observáveis GPUI, comandos
e eventos Cordis, com domínio canônico e estado transitório de apresentação.

**Entrega/aceite P09:** ciclo completo pela UI; modelagem e edição coexistem.
Sem quebra do histórico textual por alteração MDE. Undo/redo entre Buffer e
domínio tem contrato explícito, não equivalência presumida. Rejeição de
revisão obsoleta, dirty state, cancelamento e reabertura são verificáveis.
Mockup visual, implementação nativa e runtime aceito são estados separados.

## 10. Usar o Zed para validar e executar o resultado

Reaproveitar tarefas, terminal e infraestrutura de linguagem/diagnósticos.
MDE fornece alvo/contexto/rastreabilidade, não um executor genérico novo.
Validar, gerar, compilar/testar e executar são ações distintas com estados,
logs, cancelamento, ambiente definido e tratamento de código não confiável.
Gerar não instala dependências nem executa macros/scripts implicitamente;
build pode executar código e precisa ser disparado explicitamente.

**Entrega/aceite P10:** gerar, disparar build, cancelar ou ler resultado,
navegar do diagnóstico ao código/elemento quando houver trace preciso.
Sem trace, exibir localização original, não inventar origem semântica.
Validar UML não prova compilação; compilar não prova comportamento.

## 11. Validar dois percursos obrigatórios

**Percurso MDE/Rust (M2), inteiramente pela UI:**

1. Criar projeto vazio, escolher Rust e configurar pacote/alvo/toolchain.
2. Criar `Customer`, propriedades e uma operação.
3. Adicionar anotação/derive e configurar dependência/versão/features.
4. Escrever corpo da operação no Editor nativo.
5. Validar o modelo e gerar o projeto a partir do grafo de saída.
6. Disparar build e executar um teste explicitamente.
7. Renomear `Customer` e regenerar deterministicamente a saída definida.
8. Verificar que mudanças no modelo atualizam a saída gerada sem exigir
   reconciliação bidirecional do código editado.
9. Corrigir pela UI, salvar, fechar e reabrir sem perder configuração/código.
10. Regenerar e repetir build/teste a partir do estado persistido.

**Percurso Zed normal:** abrir repositório sem MDE, editar código não
gerenciado, usar busca, linguagem/LSP, diagnósticos, tarefas e Git existentes,
salvar/fechar/reabrir. Sem painéis impostos, regressão de atalhos/foco,
histórico ou substituição de capacidades nativas.

**Entrega/aceite P11:** evidências para ambos, casos negativos/repetidos,
interrupção/cancelamento e recuperação. M1 exige também autoria vazia e
persistência por categoria estrutural, não só o exemplo Customer. Outros
alvos e extensões precisam de seus próprios testes antes de serem anunciados.

## Dependências, gates e retomada

- P01/P02 auditam a base; P03 estabiliza contratos comuns. Frentes paralelas
  não podem inventar tipos, eventos ou serviços durante sua consolidação.
- UML (P04), integração (P05), autoria/geração (P06–P08) e UI (P09) podem
  avançar sobre contratos estabilizados. Revisão independente verifica
  cobertura, beta canônica, reaproveitamento e regressão Zed continuamente.
- M1 prioriza P03/P04/P05/P09/P11. M2 exige os quatro contratos e P01–P11,
  especialmente configuração/autoria/regeneração, não só diagrama bonito.
- M3 comprova contribuições instaláveis/removíveis sem alterar host nem
  apagar estado persistido; não é um novo requisito de implementar agora
  todos os alvos futuros ou TGG.
- Prioridade prática: capacidades herdadas e apresentação compartilhada;
  estilos/identidade/notação/interação; persistência/cobertura; autoria e
  projeto completo; M2M/M2T/regeneração; execução e extensibilidade.
  UI/UX e testes acompanham todas as frentes, não ficam para o final.
- Estratégia de testes: poucos cenários BDD por fluxo de feature, atravessando
  contratos públicos e módulos reais. Priorizar edição, propagação da revisão
  e persistência no mesmo percurso, com conflitos/cancelamento relevantes.
  Evitar testes por função/método ou por texto/organização do código, frágeis
  às refatorações. Unitários apenas para regras isoladas críticas; quantidade
  de testes não é medida de progresso nem substitui evidência do fluxo.
- Gate focado: catálogo/contratos, testes domínio/Cordis, persistência e
  geração. Reutilizar caches e evitar builds Zed repetitivos/desnecessários.
- Gate nativo: build completo da integração Zed/GPUI e interação no ambiente
  do mantenedor nos checkpoints pertinentes. Metadata, testes isolados ou
  mockup não o substituem. Registrar SHA, comando, toolchain, plataforma,
  resultado e evidência; falha corrigida exige repetir o gate afetado.
- Gate de conformidade: sintaxe abstrata, sintaxe concreta, intercâmbio de
  modelos, intercâmbio de diagramas e semântica, com evidências separadas.
  Intercâmbio de modelos exige XMI/perfis e versões UML 2.4.1/2.5 (anexo E);
  diagramas exigem também intercâmbio de modelos e sintaxe concreta;
  semântica implica sintaxe abstrata. Não há conformidade integral declarada.
- Gate bloqueado mantém motivo, ação de desbloqueio e próximo teste; continuar
  trabalho independente autorizado. Retomar do último requisito comprovado,
  não reiniciar consolidação ou promover etapa não executada a aceita.

## Matriz inicial de rastreabilidade e tarefas

Esta é uma fila de verificação/integração, não alegação de conclusão. Cada
linha deve receber evidência por requisito e SHA antes de aceite.

| Tarefa | Código/contrato existente a auditar | Integração e lacuna a fechar | Teste/gate |
| --- | --- | --- | --- |
| P01/P02 | [protocolos](../../crates/proto/), [auditoria](protocol-and-persistence.md) | Protos → registros/handlers → APIs/Actions; quatro contratos e inventário completo | Inventário por capacidade, sem substitutos paralelos |
| P03/P04 | [domínio](../../crates/bellatrix_mde/), [Cordis](../../crates/bellatrix_mde_cordis/) | ASG/comandos/revisões, catálogo/herança/restrições e quatro camadas | Positivos/negativos, rollback, cobertura normativa |
| P05 | [Project](../../crates/project/), [Workspace](../../crates/workspace/), [Bellatrix Project](../../crates/bellatrix_project/) | ProjectPath/Buffer/SerializableItem, configuração completa e revisão semântica | Documento real, conflito externo, salvar/restaurar |
| P06 | [Editor](../../crates/editor/), [Language](../../crates/language/) | Fonte única autoral, contexto LSP, macros/anotações/dependências | Corpo + recurso alvo, save/reopen/regenerate |
| P07/P08 | [gerador Rust](../../crates/bellatrix_mde_rust_generator/), [Tera](../../crates/bellatrix_mde_templates/) | Grafo de saída, Tera e script de materialização | Determinismo, saída gerada, Git e Rust gerado |
| P09 | [workbench](../../crates/bellatrix_mde_workbench/), [UML UI](../../crates/bellatrix_ui_uml/) | UX-01–UX-09, apresentação compartilhada, tema ativo e interação | M1 vazio, teclado/Dock, gate GUI |
| P10/P11 | [Project](../../crates/project/), [terminal](../../crates/terminal/) | Tarefas/build/run explícitos, traces e regressão host | Customer completo + Zed normal, cancelamento |

## Estado e comunicação

**Não confundir evidência histórica com aceite atual.** Há relato do usuário
de build aprovado em `a887`; é evidência daquele checkpoint, não aprovação
automática de alterações posteriores. `b2ef40e` integrou painéis;
`b19cce5` acrescentou propriedades herdadas; `dd2c0ea` trouxe três modos de
Style e multiline; `4fa12a3` acrescentou quick-create herdado e Usage de
interface requerida. Esses incrementos não demonstram M1/M2 concluídos.
Testes headless posteriores não equivalem a GUI nativa aceita; auditar
resultados/escopo e SHA no início de cada retomada. Todos os aceites P01–P11
continuam pendentes até evidência correspondente, sem descartar testes já
comprovados de requisitos individuais.

Commits incrementais na branch de trabalho, sem PR/CI voluntários;
validação local pelo mantenedor precede pedido de PR. A cada **cinco novos
commits publicados e verificados após `b19cce5`** (pedido de 2026-11-03,
01:18 UTC), reenviar **este plano completo atualizado**, com estado,
evidências, lacunas e próximos passos, não só delta. Checkpoint não pausa
trabalho independente. Contagem operacional fica fora do documento, sem
commit cosmético a cada incremento; publicação não equivale a validação.

## Proveniência e reconciliação dos planos

Histórico das decisões de 2026-11-02 incorporadas nesta consolidação.
O plano revisado de onze passos preserva o objetivo e os requisitos
detalhados do plano inicial, com as correções posteriores indicadas acima.

| Fonte da conversa | Requisito preservado / destino |
| --- | --- |
| 15:01 UTC — objetivo do produto | Objetivo LOW CODE e código específico na UI: objetivo, P06 e M2 |
| 15:08 UTC — plano inicial | Plano inicial: quatro contratos P01; domínio P03; UML/validação P04; autoria P06; projeto completo P05; M2M/M2T P07; regeneração P08; UI P09; execução P10; Customer P11; fronteiras e remoção de duplicação transversais |
| 15:43 UTC — revisão de reaproveitamento | Revisão reuse-first: onze títulos e ordem preservados; protocolo/handlers/APIs P02 e percurso Zed normal P11 |
| 15:48 UTC — ordem dos alvos | Rust primeiro; Zig, C#, Kotlin e Flutter depois, M2/M3 |
| 15:54 UTC — prioridade do primeiro marco | Diagramas estruturais como primeiro marco; feedback focado/caches antes de build completo pertinente |

A revisão posterior não elimina autoria, configuração ou geração. A síntese
intermediária de doze fases rejeitada não é fonte normativa deste plano.
Referências antigas a preservar formatos/aliases Bellatrix são superadas
pela política beta atual; preservação de trabalho autoral continua obrigatória.
Atualizar specs, formatos, comandos, testes e UI para os contratos reais;
renome de pasta, glyph ou layout não prova conclusão funcional.
