# language: en
Feature: Pure UML validation at the Bellatrix document boundary
  These integration flows use the public APIs and real save, close and reopen steps.
  The Rust harness executes the scenarios; this specification is not an interpreted runner.
  No graphical interface or unit-level validator is exercised.

  Scenario: Preserve UML findings across malformed presentation and repair
    Given an imported valid UML class project and a copy with one semantic violation
    When I corrupt presentation structure, View identities, geometry, style and hierarchy
    Then the full document decoder rejects the malformed presentation
    But pure UML validation reports exactly the same findings for each semantic graph
    When I save the supplied request as JSON, close it and reopen it unchanged
    Then its semantic collections and UML findings remain identical
    When I repair presentation and repeat the original request after the invalid request
    Then the original UML findings return unchanged

  Scenario: Keep mixed-domain validation explicitly blocked through reopening and repair
    Given an imported UML class project with accepted implemented-rule findings
    When I add Rust and C4 elements and repeated explicit cross-domain references
    Then validity is unverified and coverage is mixed_domain_semantics_unverified
    And a capacity error prevents error-only consumers from reporting success
    And external domains and references have explicit warning diagnostics
    When I persist the mixed-domain project to JSON and BLX, close it and reopen both files
    Then every external object and collection member survives with identical findings
    And an independent UML violation cannot turn the blocked model into a false success
    When I restore the original pure UML graph and repeat the mixed request
    Then both results remain deterministic and preserve their distinct evaluation states
