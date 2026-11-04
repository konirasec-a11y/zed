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
    Then validity is unverified and coverage is implemented_uml_scalars_and_integrity_only
    And a capacity error prevents error-only consumers from reporting success
    And external domains and references have explicit warning diagnostics
    When I persist the mixed-domain project to JSON and BLX, close it and reopen both files
    Then every external object and collection member survives with identical findings
    And an independent UML violation cannot turn the blocked model into a false success
    When I restore the original pure UML graph and repeat the mixed request
    Then both results remain deterministic and preserve their distinct evaluation states

  Scenario: Validate the closed UML graph alongside unmapped domains
    Given a valid UML class project and an unrelated Rust object with a conflicting name and invalid UML-like property
    Then the entire closed UML graph is checked without interpreting the Rust object as UML
    And the whole mixed project remains explicitly unverified
    When I make two UML members indistinguishable within their namespace
    Then the same UML namespace errors appear as in the original pure UML project
    When I save, close and reopen both states through JSON and BLX
    Then semantic data, order and diagnostics are unchanged
    When I repair the namespace collision and repeat the invalid request
    Then both results are repeatable without claiming external-domain validity

  Scenario: Suspend global rules at external references while retaining proved UML errors
    Given a UML Type reference and PackageImport targeting unmapped Rust and C4 objects
    Then explicit warnings identify those cross-domain references
    And global and relational rules are suspended without false missing, type or namespace errors
    When I author an invalid Boolean on an independent UML Class
    Then its proved scalar violation is reported alongside the unverified capacity error
    When I save, close and reopen the unknown and invalid states through JSON and BLX
    Then all objects and findings remain identical
    When I repair the Boolean but retain the external boundary
    Then only the proved violation disappears and validity remains unverified
    When an external object alone references the UML namespace
    Then reverse dependencies also suspend global absence queries
    And reopening and restoring the pure UML graph retain deterministic findings

  Scenario: Preserve canonical ownership integrity across opaque domain boundaries
    Given a valid UML project and two opaque external objects
    When an already owned UML Class receives an additional external owner
    Then canonical ownership reports the proved multiple-owner violation
    When I save, close and reopen the project through JSON and BLX
    Then all domain records and diagnostics remain identical
    When I remove only the added owner
    Then the proved violation disappears without claiming external validity
    When the UML Class owns a path through both external objects back to itself
    Then canonical ownership reports the proved cross-domain cycle
    And saving, reopening and restoring the original ownership preserves the findings and repair
    When the ownership cycle instead remains entirely in a disconnected external component
    Then it is not falsely reported as a UML violation
    When a separate cross-domain Type reference forces partial integrity evaluation
    Then the disconnected external ownership cycle is still excluded from UML diagnostics
    And JSON and BLX reopening preserve that partial coverage and capacity error
    And persistence and repeated pure and mixed requests retain their distinct evaluation states
