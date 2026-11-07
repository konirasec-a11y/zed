# BDD workflow requirements

The `.feature` files preserve the document, native authoring and StarUML
interchange requirements. Their Rust integration harnesses exercise complete
flows through public APIs, including editing, writing files, closing and
reopening. They are not unit tests or a general Gherkin interpreter.

Native documents use schema version 2 and the typed `ruml::Registry` contract.
Bellatrix schema version 1 fixtures and their old BDD implementation are not
supported by these flows. StarUML `.mdj` inputs remain external interchange
documents and are imported through the public MDJ API.

The diagram examples retain their historical `valid`/`invalid` names as
identifiers. They verify persistence, not UML certification or diagnostics from
the removed validation stage. Cordis editing and save requirements are in
`../../bellatrix_mde_cordis/tests/features/`.

Eight diagram examples remain explicitly pending because their original data
cannot be represented by the current typed contracts. Their manifests retain
the reasons, but no replacement fixtures pretend to provide equivalent
coverage. The integration harness marks them ignored and returns an explicit
error if they are forced with `--ignored`; they must not be counted as passed.

Commit and push before running the five BDD integration targets selected by
`script/bellatrix_mde/smoke.ps1`. Passing a source check or compiling the crates
does not establish that these workflows pass at runtime.
