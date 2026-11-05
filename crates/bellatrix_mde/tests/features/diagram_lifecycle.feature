# language: en
Feature: Authored project persistence through public document APIs
  The Rust integration harness executes these examples.
  This file is documentation, not an interpreted Gherkin runner.
  Existing fixture names retain their historical valid/invalid labels;
  these flows no longer evaluate UML conformance or diagnostic verdicts.

  Scenario Outline: Import, persist and reopen an isolated diagram
    Given the authored project identified by "<case_id>" in its corpus manifest
    When I import the project through the public document API
    And I persist the project to JSON and BLX, close it and reopen both files
    Then the entire model remains identical

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

  Scenario Outline: Persist and reopen the complete TodoList modeling application
    Given the authored TodoList project identified by "<case_id>" in its corpus manifest
    When I import the project through the public document API
    And I persist the project to JSON and BLX, close it and reopen both files
    Then the entire model remains identical

    Examples:
      | case_id |
      | todolist-valid |
      | todolist-invalid |
