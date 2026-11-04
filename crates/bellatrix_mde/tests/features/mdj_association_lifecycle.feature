# language: en
Feature: Lossless StarUML association defaults through the public document APIs
  These BDD integration flows import MDJ, author edits, save files, close, reopen,
  validate through the kernel and export MDJ. They do not compile the platform UI.
  The Rust harness executes these scenarios; this file is not an interpreted runner.

  Scenario: Preserve omitted and explicit empty multiplicities through edits and reopening
    Given an MDJ association with explicit navigation and one omitted multiplicity
    And its other end has an explicit empty multiplicity string
    When I import the MDJ into the universal document contract
    Then both Property ends retain an empty multiplicity rather than asserting one
    And exporting without edits returns the original bytes exactly
    When I rename a class, save to JSON and BLX, close and reopen each file
    Then kernel validation produces the same findings before and after reopening
    And MDJ export preserves the omitted field and the explicit empty string separately
    When I author a nonempty multiplicity on the originally omitted end and reopen again
    Then the edited multiplicity and its derived association projection survive export
    When I restore the empty default on that end without changing its source metadata
    Then MDJ export restores the original field absence

  Scenario: Author navigation decisions without inventing defaults or losing source spelling
    Given association ends with absent and explicit unspecified navigation declarations
    When I import them as canonical UML with typed incomplete authoring knowledge
    Then normative Boolean and complete-set queries never reinterpret unknown as false
    And kernel validation is unverified with a capacity blocker and retains independent violations
    When I save to JSON and BLX, close, reopen and rename an unrelated class
    Then the knowledge, kernel results and absent versus explicit source spelling survive
    When I choose navigable and not navigable through atomic commands
    Then the knowledge gap disappears and the normative queries return the chosen Booleans
    When I restore both unspecified declarations in reverse order
    Then export preserves the original absence and explicit string without creating stale projections
    When knowledge references are duplicated, unresolved, non-owned or overlap known navigation
    Then only the kernel reports the malformed semantic declarations
    And a reparent command cannot silently leave stale unverified navigation references
