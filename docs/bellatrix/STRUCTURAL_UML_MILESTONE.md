# Bellatrix structural UML milestone

## Scope and authority

This increment extends the single `bellatrix_mde` domain. It does not claim
complete UML 2.5.1 support or seven complete specialized editor palettes.
The normative inputs remain `standards/uml/20161101/UML.xmi` and `UMLDI.xmi`.
RUML and StarUML remain implementation/interaction references, not authorities.

Reuse order: inspect Zed protocol contracts and follow their handlers; where
no local diagram contract exists, use native Item/Panel/Action APIs and the
existing typed Cordis bridge. No Zed core edits are required by this increment.

## Implemented contracts (runtime verification recorded separately)

| Contract | Normative basis / implementation | Focused evidence |
| --- | --- | --- |
| Metaclass ancestry | Generated from all 242 UML metaclasses; `uml::metamodel` | Generator `--check`, unknown-name and inheritance tests |
| Model-wide generalization | `Classifier-no_cycles_in_generalization` | Hidden/unrendered edges and cycles tested |
| Compatible specialization | `Classifier-specialize_type`, `Classifier-maySpecializeType-spec` | Metaclass conformance uses the generated catalog |
| Final parent | `Classifier-non_final_parents` | `isFinalSpecialization`, not a substitute `isLeaf` flag |
| Stereotype specialization | `Stereotype-generalize` | Both directions reject non-stereotype peers |
| Diagram metadata | Optional `uml_di_type`, default-empty `model_elements` | JSON/protobuf legacy and future-name round trips |
| Context of six diagram categories | UMLDI `*-no_modelElement` | Context is not the list of depicted subjects |
| Composite Structure context | `UMLCompositeStructureDiagram-modelElement_mult/modelElement_type` | Exactly one StructuredClassifier context; instance context is explicitly unsupported |
| Safe domain mutation | `commands::apply`, Project revision integration | New invalid generalizations roll back; unrelated drafts remain editable |
| Diagram lifecycle | Create/rename/delete reuse domain commands and Cordis | Delete removes only diagram/views, never semantic elements |
| UI integration | Native Zed controls in Bellatrix Workbench | New-diagram menu, stored type display, coordinate/selection tests |

## Seven-category coverage boundary

Class, Object, Package, Component, Composite Structure, Deployment and Profile
have explicit type identities and shared model/projection checks. This is not
proof of complete per-element constraint coverage, rendering or editing for
each category. The toolbox remains labelled as Class editing tools.

Composite Structure context may also be an InstanceSpecification with a
StructuredClassifier classifier under UMLDI. The compatibility model does not
yet define a validated instance-classifier encoding: this case emits an explicit
unsupported diagnostic, not a false conformance success. Complete Component,
Deployment, Object and Profile editing still needs dedicated model/rule/view
acceptance. Absence of a rule must never be reported as verified conformance.

## Persistence and feedback

Existing protobuf fields retain their tags; Diagram adds optional tag 6 and
repeated tag 7. Old JSON fields default to absent type and empty context. Unknown
future diagram type strings round-trip without reinterpretation; explicit
structural validation refuses a mismatched type. Existing namespaces stay intact.

Validation is read-only. Domain mutations publish through the existing
revision-checked ProjectService path. Code, model and native GUI verification
are distinct: static checks or passing headless tests do not establish native
interaction success.

## Fast feedback commands

```text
python script/bellatrix_mde/static_check.py
python script/bellatrix_mde/bellatrix_uml_catalog.py --check
cargo +1.98.1 test -p bellatrix_mde
cargo +1.98.1 test -p bellatrix_mde_cordis
```

Use focused headless checks before a full Zed build. Preserve warmed Cargo caches.
The maintainer validates native Windows behavior on the working branch; no PR
or manual CI run is required to obtain that feedback.

## Verification record — 2026-11-02

- Exact copied sources of seven headless packages: **129 tests passed, zero failures**.
- Repository `script/clippy` on the isolated domain/Cordis packages: **passed**, release/all-targets/all-features with warnings denied.
- Rust toolchain: 1.98.1. Shared Cargo/target caches retained between runs.
- Source fingerprint recorded by `bellatrix_headless.py`; copied sources were
  unchanged during the run. Root workspace patches/profiles/GUI dependencies
  are deliberately outside this supplemental check.
- Root `cargo metadata --offline --no-deps`: passed, 278 workspace packages.
- Catalog deterministic check and source-contract check: passed.
- Two exact extracted Canvas coordinate/selection helper tests: passed with
  rustc; this is not a GPUI compilation or interaction test.
- Native Zed/GPUI/Windows behavior: not executed here.
- Root Cargo.lock resolution: pending; no dependency hashes invented.

The legacy class-diagram acceptance fixture attempted Class-to-Interface
Generalization. It now uses Class-to-Class for Generalization, keeps
Class-to-Interface for InterfaceRealization, and explicitly checks rejection
and rollback for the invalid Generalization. Aggregation/composition checks
use the existing canonical Association/end1Aggregation representation.

Focused reproducible check, without the graphical dependency graph:

```text
python script/bellatrix_mde/bellatrix_headless.py --cache-dir <outside-repository-cache> -- test --workspace --no-fail-fast
```

This command tests only the seven isolated headless packages and preserves its
build cache. It does not replace native workspace acceptance.
