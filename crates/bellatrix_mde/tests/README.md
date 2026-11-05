# Existing workflow tests

The standalone UML kernel validation stage has been removed. Tests dedicated to
its diagnostics, aggregate verdicts and validation-only mutation gates have been
removed with it. Mixed scenarios retain their existing command, query, Inspector,
MDJ interchange and persistence assertions where those checks are independent of
that stage. No replacement UML certification or placeholder validator is used.

The remaining suites cover existing document, editing, presentation, query and
generation workflows. The feature files describe the surviving BDD scenarios;
they are documentation, not an interpreted Gherkin runner. Existing Cordis
workflows remain in `../../bellatrix_mde_cordis/tests/`.

This cleanup adds no scenarios or unit tests. It does not establish a runtime
pass: compilation and test execution were not run as part of this change.
Further migration of fixtures and legacy internal test imports to the ruml
contract is separate from removal of the deleted validation APIs.
