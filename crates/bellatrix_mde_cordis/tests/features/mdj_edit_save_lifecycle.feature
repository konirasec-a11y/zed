# language: en
Feature: Native editing after MDJ conversion and separate experimental export
  The Rust harness uses the shared native session, Cordis project services and real files.
  Preparing or exporting MDJ bytes never means that the native document was saved.

  Scenario: Edit the native document and keep experimental MDJ export separate from saving
    Given an MDJ converted into an unsaved native project in the shared platform service
    When I apply a native style that has no MDJ inverse through the common UI session
    Then the native edit commits and experimental export reports the unsupported value
    And a no-op retains the same dirty snapshot without change notifications
    When I apply a representable style and request experimental MDJ bytes
    Then the output reimports without loss
    And preparing and cancelling an export emits no saved notification
    When I publish the reserved bytes to a new file and read it again
    Then the native project remains dirty and the original MDJ source stays unchanged
    And publishing over an existing destination is rejected without changing its bytes
    When the native destination changes after its baseline was captured
    Then native save rejects the stale baseline without changing the file or clearing dirty state
    And native save with the current baseline overwrites the same file successfully
    Then only that snapshot becomes clean and a native saved notification is emitted

  Scenario: Preserve newer edits and reject receipts from a closed document session
    Given an MDJ export receipt and a reserved experimental export
    When another native edit is committed before the file is published
    Then publication rejects the old revision and the newer document remains dirty
    When I close the project and reopen the same identity at its original revision
    Then an old export receipt cannot reserve a write in the new session
    And an old snapshot replacement cannot alter the reopened document
    And queued edit and export requests from the old session are rejected too
    And a fresh unchanged export still returns the original MDJ bytes

  Scenario: Preserve complete native authoring when experimental MDJ export is unavailable
    Given an imported MDJ project with an additional native engine section
    When I request experimental MDJ export
    Then the unsupported platform section is explicitly rejected without mutation
    And native authoring still accepts edits and preserves that section in a complete native file

  Scenario: Create identical classes and relationships through events and the UI session
    Given the same blank native Class diagram in separate project sessions
    When I create Classes, an Interface, an Association, a Generalization and a Dependency
    And I rename a Class, add an Operation and Parameters, reorder them and create a quick Note
    Then direct Cordis events and UI commands produce identical canonical snapshots
    And a tracked Inspector no-op reports the current revision without replacing the snapshot
    And selected View endpoints and semantic ownership survive JSON and Protobuf reopening
    And replaying the first request at its old revision leaves the authored snapshot unchanged
