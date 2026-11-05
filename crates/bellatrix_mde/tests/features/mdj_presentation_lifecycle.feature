# language: en
Feature: Lossless authored StarUML presentation interchange
  These BDD flows use the public import, command, persistence and export APIs.
  The Rust harness executes the scenarios with real save and reopen steps.
  They do not prove native UI editing or StarUML renderer equivalence.

  Scenario: Preserve and edit authored style through native persistence and MDJ
    Given an MDJ document with two Views of one Class and a nested label
    And explicit colors, complete fonts, alignments and Boolean visual flags
    When I import, save, close and reopen JSON and BLX copies
    Then supported style is authored only in the universal Style contract
    And missing fields remain missing and unedited export returns the original bytes
    When I change one occurrence's style through the public command
    And save, reopen, export MDJ and import the exported file again
    Then the edited style survives without changing other occurrences or nested labels
    And semantics, composition, unknown fields and the source file remain unchanged
    When I edit an edge color and the nested label's persisted style contract
    Then their MDJ encodings survive reopening without changing the model
    When I remove the complete font and fill from the first occurrence
    Then the corresponding fields are absent after export and reopening

  Scenario: Preserve unsupported source style without substituting defaults
    Given an MDJ document with null, malformed, partial and unknown style encodings
    When I import, persist and reopen the document
    Then those fields remain source residue with explicit diagnostics
    And unedited export returns the original bytes
    When I rename only the project and export it
    Then every unsupported style value and unrelated extension survives exactly
    When I try to overwrite a retained source field through typed style
    Then export fails instead of discarding either value
    And the existing source file remains unchanged

  Scenario: Reject unsupported style edits and recover without changing the source
    Given an imported MDJ document saved and reopened as JSON and BLX
    When I author an unsupported opacity, weight or incomplete font
    Then MDJ export fails rather than replaying stale source or inventing defaults
    When I repair the edit and export again
    Then the original bytes return unchanged
    And repeated exports remain deterministic
