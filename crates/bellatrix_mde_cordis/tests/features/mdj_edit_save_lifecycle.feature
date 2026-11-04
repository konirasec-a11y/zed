# language: en
Feature: Transactional MDJ editing and verified external save receipts
  The Rust harness uses Cordis project services and real file save/reopen steps.
  Preparing bytes or reserving a write never means that a file was saved.

  Scenario: Commit only edits with an MDJ inverse and acknowledge only actual saved bytes
    Given a saved imported MDJ project in the shared platform service
    When I request an unsupported style edit through the MDJ edit event
    Then the snapshot, revision, clean state and change notifications stay unchanged
    And a supported no-op retains the same clean or dirty snapshot without notifications
    When I apply a supported style edit and request its MDJ bytes
    Then the shared document becomes dirty and the output reimports without loss
    And preparing and cancelling an external save emits no saved notification
    When I write the reserved bytes, close the file and read it again
    And acknowledge the exact file bytes for that snapshot
    Then only that revision becomes clean and the original MDJ source stays unchanged
    And a mismatching byte receipt leaves the document dirty

  Scenario: Preserve newer edits and reject receipts from a closed document session
    Given an MDJ export receipt and a reserved external save
    When another supported edit is committed before the write is acknowledged
    Then the acknowledgement cannot mark the newer document saved
    When I close the project and reopen the same identity at its original revision
    Then an old export receipt cannot reserve a write in the new session
    And an old snapshot replacement cannot alter the reopened document
    And queued edit and export requests carrying the old snapshot are rejected too
    And a fresh unchanged export still returns the original MDJ bytes

  Scenario: Keep native authoring distinct from lossless MDJ editing
    Given an imported MDJ project with an additional native engine section
    When I request MDJ editing or export
    Then the unsupported platform section is explicitly rejected without mutation
    And native authoring can still preserve that section in a complete native file
