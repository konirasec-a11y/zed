# language: en
Feature: Complete authored UML project lifecycle through the public kernel boundary
  Valid means accepted by the implemented contracts, not full UML certification.
  Unsupported semantic warnings remain visible and never prove valid or invalid.
  The Rust integration harness executes these examples; this file is a specification and inventory, not an interpreted Gherkin runner.
  Expected error diagnostics match exactly; additional unsupported warnings are retained.
  Verdict mismatches fail the scenario after both persistence round-trips are exercised.
  These flows do not exercise the graphical interface or generated application runtime.

  Scenario Outline: Import, validate, persist and reopen an isolated diagram
    Given the authored project identified by "<case_id>" in its corpus manifest
    When I import the project through the public pure document API
    And I validate the selected diagram through the public pure validation API
    Then its verdict and exact diagnostic code, subject and severity match the specification
    And unsupported semantic warnings remain visible without becoming a conformance verdict
    When I persist the project to JSON and BLX, close it and reopen both files
    Then the entire model and validation diagnostics remain identical

    Examples:
      | case_id |
      | class-valid |
      | class-invalid |
      | object-valid |
      | object-invalid |
      | package-valid |
      | package-invalid |
      | component-valid |
      | component-invalid |
      | composite_structure-valid |
      | composite_structure-invalid |
      | deployment-valid |
      | deployment-invalid |
      | profile-valid |
      | profile-invalid |
      | use_case-valid |
      | use_case-invalid |
      | activity-valid |
      | activity-invalid |
      | state_machine-valid |
      | state_machine-invalid |
      | sequence-valid |
      | sequence-invalid |
      | communication-valid |
      | communication-invalid |
      | interaction_overview-valid |
      | interaction_overview-invalid |
      | timing-valid |
      | timing-invalid |

  Scenario Outline: Validate and reopen the complete TodoList modeling application
    Given the authored TodoList project identified by "<case_id>" in its corpus manifest
    And the valid and invalid projects differ in exactly one semantic field
    When I import the project through the public pure document API
    And I validate the complete model and every diagram through the public pure validation API
    Then its verdict and exact diagnostic code, subject and severity match the specification
    And unsupported semantic warnings remain visible without becoming a conformance verdict
    When I persist the project to JSON and BLX, close it and reopen both files
    Then the entire model and validation diagnostics remain identical

    Examples:
      | case_id |
      | todolist-valid |
      | todolist-invalid |
