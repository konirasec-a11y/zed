# Tera 2.4.0 no pipeline MDE — geração orientada por grafos

Esta decisão diz respeito **à Engine-MDE** dentro da plataforma Bellatrix.
O [metamodelo e as transformações](MDE_GRAPH_TRANSFORMATION_ARCHITECTURE.md)
continuam sendo o fundamento: **Tera não interpreta UML, grafos, tipagem
ou restrições**; é o renderizador textual M2T, com templates por alvo.

## Implementação integrada (primeiro adaptador: Rust)

| Etapa | Código atual | Papel |
| --- | --- | --- |
| Fonte ASG | `bellatrix_mde::{Project, Element, Relation}` | Identidades, atributos, escopo e relações persistidas |
| Análise de grafos | `bellatrix_mde::{Graph, GraphIndex, analyze_generation_graph}` | Validação, escopo, dependências orientadas e ordem determinística |
| IR intermediário de geração | `GenerationModel { units: Vec<GenerationUnit> }` | Dados estáveis e dependências sem tipo GPUI/Cordis nem filesystem |
| Adaptador Rust | `bellatrix_mde_rust_generator::plan_with_templates` | Resolve declarações, nomes, assinaturas e manifestos |
| M2T Tera 2.4.0 | `bellatrix_mde_templates::TemplatePack` + `templates/*.tera` | Renderiza exclusivamente texto de contexto serializável |
| Artefatos e I/O | `bellatrix_mde::{Plan, publish_new, publish_replace}` | Protege caminhos/collisions, diff/ownership e publicação em staging |

`bellatrix_mde_templates` é propositalmente independente de Rust:
aceita pacotes embutidos, renderiza dados `serde::Serialize` e permite
trocar templates. **Não** importa Zed, GPUI, Cordis, arquivos de projeto,
tipos UML ou `petgraph`. Apenas o adaptador da linguagem conhece
sua sintaxe e seus caminhos. O Tera é fixado em
`[workspace.dependencies] tera = "2.4.0"`.

O primeiro alvo integrado é Rust. Templating de classes/structs,
traits, assinaturas, corpos fornecidos pelo usuário e manifestos
`Cargo.toml` está em
`crates/bellatrix_mde_rust_generator/src/KERNEL/compiler/rust/templates/`.
O manifesto ainda é **validado/construído com TOML tipado** antes de
ser renderizado pelo Tera; não inventamos estruturas Cargo no template.

Arquivos binários e textos de entrada `fs.file` passam pelo `Plan`
inalterados. Eles não são reinterpretados como Tera. O publish
continua com regras de propriedade dos arquivos e staging seguro;
**não chamar** `fs::create_dir_all` e `fs::write` de cada template,
porque isso desviaria das proteções da plataforma.

## Dependências e ordem

No grafo semântico, `depends_on` e `uml.dependency` são
**cliente → fornecedor**. Para geração, o grafo de scheduling
inverte a aresta: **fornecedor → cliente**. O algoritmo
`petgraph::algo::toposort` detecta ciclos e um conjunto ordenado
estabiliza a ordem de vértices independentes. Uma associação bidirecional
**não** significa ciclo de compilação; `owns`, `declares` e
`physical_parent` pertencem a outras consultas do grafo.

Cada `GenerationUnit` preserva ID/kind/name e `requires` com
ID, tipo, nome e relação da dependência. Um adaptador TypeScript/C#/
F#/JavaScript poderá resolver imports, namespaces, módulos e
configurações a partir desses dados, mas **não existe ainda adaptador
funcional dessas linguagens**. Nem toda linguagem exige ordenação
linear de arquivos; F# é exemplo em que a ordem de compilação é
especialmente relevante. Não deduzir import automaticamente de toda
Association UML; políticas de geração são específicas por alvo.

## Como personalizar os templates

Na API Rust, sem tocar nas regras do grafo:

```rust
let mut templates = bellatrix_mde_rust_generator::default_templates()?;
templates.override_template(
    "rust/struct.rs.tera",
    "{{ annotations }}pub struct {{ name }} {\n{% for field in fields %}    {{ field }}\n{% endfor %}}",
)?;
let plan = bellatrix_mde_rust_generator::plan_with_templates(
    &project, &scope,
    bellatrix_mde_rust_generator::GenerationOptions { skeleton: false },
    &templates,
)?;
```

Um pack de outra linguagem pode ser criado diretamente em
`bellatrix_mde_templates::TemplatePack`. A API é **preparação e
renderização puras**; conectar um pack externo ao CLI exigirá
registrar um target adapter próprio e contratos de versão/permissão.
Os arquivos de template podem ser empacotados em uma engine/plugin
futura, mas o carregamento de plugins de terceiros ainda não existe.

Para código e arquivos de configuração, o pack chama
`tera.autoescape_on(Vec::<&str>::new())`. Isso é **uma escolha de
saída de código**, não recomendação para HTML: o exportador HTML
precisa usar escaping adequado a HTML. Tera recebe valores já
resolvidos e **não** realiza validação semântica nem compila o código
gerado. "1:1" aqui significa controlar a forma textual com templates,
não garantir por si só equivalência de execução.

## Evolução necessária (não implementada neste commit)

1. Metamodelos tipados por alvo, por exemplo `TargetCodeModel`,
   `FilesystemModel`, `DependencyModel`, `ConfigurationModel`.
2. Regras M2M versionadas: `SourceGraph → TargetGraph + TraceGraph`,
   com validação própria e correlações estáveis entre modelos.
3. Adaptadores de linguagens TypeScript, C#, F#, JavaScript, Rust,
   templates e mapas de tipos reais, separados do motor Tera.
4. Validação sintática/compilação do alvo e testes comparativos de
   artefatos, erros de import, ordenação, ciclos e persistência.
5. Plugin SDK para registrar metamodelos, regras e packs em runtime,
   composto via Cordis. O fork do Zed continua host, não dono do MDE.

## Teste local (sem PR nem CI)

```powershell
git pull --ff-only origin feat/bellatrix-native-engines
cargo +1.98.1 check -p bellatrix_mde_templates -p bellatrix_mde -p bellatrix_mde_rust_generator
cargo +1.98.1 test -p bellatrix_mde_templates -p bellatrix_mde -p bellatrix_mde_rust_generator
cargo +1.98.1 run -p bellatrix_mde_cli -- plan crates/bellatrix_mde/fixtures/todo.json
```

O último comando é um exemplo de uso da CLI; a resolução do workspace e
as variantes de argumento de `plan` devem seguir o `--help` da CLI.
A dependência Tera atualiza o `Cargo.lock` na primeira resolução local;
commitar um lockfile só após essa atualização real, nunca inventar hashes.
