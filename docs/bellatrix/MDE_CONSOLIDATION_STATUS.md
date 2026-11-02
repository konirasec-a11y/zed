# Bellatrix MDE consolidation checkpoint

> Historical checkpoint: namespace compatibility statements below are superseded by the [current beta format policy](mde.md#beta-format-and-composition). Only `bellatrix.mde` is supported now.

Base: `9607a1aadca8b128198c4b88192464a7bb2f6638`.

The workspace now registers `bellatrix_mde` as the single persisted/modeling
package. Consumers and integration-test fixture paths use that package.
The replaced `bellatrix_mde_model` and `bellatrix_mde_modeling` packages are
removed, rather than retained as parallel domain implementations.

Before removal, all 37 non-manifest files were mapped to consolidated paths;
42 existing test names were retained and fixture/schema contents compared
byte-for-byte. Persisted namespaces and schema versions were not renamed.

## Recorded verification

- `python script/bellatrix_mde/static_check.py`: passed (18 crates, 77 includes).
- Bellatrix manifest membership/dependency/include-path inspection: passed.
- `git diff --check`: passed.
- Rust compilation and test execution: NOT RUN at this checkpoint.
- Cargo.lock refresh by Cargo: pending, not manually synthesized.
- Native Zed build and interaction: NOT RUN.

The static check detects source-layout regressions; it does not prove Rust
compilation, UML conformance, event lifecycle correctness or GUI behavior.
The next verification is a focused headless domain build/test, not a full
Zed application build. Structural UML rules and diagram interaction are
separate follow-up changes with independent tests.

## Subsequent verification

The first checkpoint above remains a historical static-only record. See
[Structural UML milestone](STRUCTURAL_UML_MILESTONE.md) for the subsequent
current scoped test and Clippy evidence, including subsequent real-root lockfile
resolution. Native GUI acceptance remains separate and has not run here.
