# language: en
Feature: Complete authored UML project lifecycle through the public kernel boundary
  Valid means accepted by the implemented contracts, not full UML certification.
  Unsupported semantic warnings remain visible and never prove valid or invalid.
  The Rust integration harness executes these examples; this file is a specification and inventory, not an interpreted Gherkin runner.
  Document and validation responses retain API version 1 and the declared implemented-rule coverage.
  Expected error diagnostics match exactly; additional unsupported warnings are retained.
  Verdict mismatches fail the scenario after both persistence round-trips are exercised.
  Each of the seven valid structural examples also authors a semantic violation through public commands,
  validates the model and diagram, persists the invalid draft to JSON and BLX, repairs both reopened
  drafts through public commands, and persists the repaired results in both formats.
  The authored Object, Component and Deployment violations use supported commands rather than
  claiming to edit the read-only classifier collection, absent nestedClassifier collection or kind field.
  The seven imported invalid fixtures retain their original exact diagnostic and persistence checks.
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
      | activity-malformed-incidence-invalid |
      | activity-external-control-valid |
      | activity-presentation-roundtrip-valid |
      | activity-invalid |
      | activity-initial-incoming-invalid |
      | activity-initial-control-invalid |
      | activity-final-outgoing-invalid |
      | activity-fork-incoming-invalid |
      | activity-join-outgoing-invalid |
      | activity-merge-outgoing-invalid |
      | activity-decision-incoming-invalid |
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
