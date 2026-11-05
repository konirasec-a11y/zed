# language: en
Feature: Lossless StarUML association defaults through the public document APIs
  These BDD integration flows import MDJ, author edits, save files, close, reopen,
  and export MDJ. They do not compile the platform UI.
  The Rust harness executes these scenarios; this file is not an interpreted runner.

  Scenario: Preserve omitted and explicit empty multiplicities through edits and reopening
    Given an MDJ association with explicit navigation and one omitted multiplicity
    And its other end has an explicit empty multiplicity string
    When I import the MDJ into the universal document contract
    Then both Property ends use the UML default multiplicity of one with absent bound specifications
    And the original omitted versus empty source notation is preserved separately
    And exporting without edits returns the original bytes exactly
    When I rename a class, save to JSON and BLX, close and reopen each file
    Then MDJ export preserves the omitted field and the explicit empty string separately
    When I author a nonempty multiplicity on the originally omitted end and reopen again
    Then the edited multiplicity and its derived association projection survive export
    When I restore the empty default on that end without changing its source metadata
    Then MDJ export restores the original field absence

  Scenario: Author navigation decisions without inventing defaults or losing source spelling
    Given association ends with absent and explicit unspecified navigation declarations
    When I import them as canonical UML with typed incomplete authoring knowledge
    Then normative Boolean and complete-set queries never reinterpret unknown as false
    When I save to JSON and BLX, close, reopen and rename an unrelated class
    Then the knowledge and absent versus explicit source spelling survive
    When I choose navigable and not navigable through atomic commands
    Then the knowledge gap disappears and the normative queries return the chosen Booleans
    When I restore both unspecified declarations in reverse order
    Then export preserves the original absence and explicit string without creating stale projections
