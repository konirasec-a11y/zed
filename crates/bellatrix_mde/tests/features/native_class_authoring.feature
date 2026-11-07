Feature: Native class authoring uses the canonical schema 2 UML contracts
  Native authoring persists ruml contracts independently of diagram occurrences.
  JSON and BLX preserve the complete model, authoring knowledge and presentation.

  Scenario: Author classifiers and ordered members from the native blank
    Given a native blank class diagram
    When I author classes, interfaces, nested classifiers and ordered members
    Then ownership, names, member labels and Inspector fields reflect the UML contracts
    And JSON and BLX preserve the authored section

  Scenario: Quick actions create associations with concrete Property ends
    Given a native blank class diagram
    When I quick-create associated, aggregated and composited classes
    Then each association owns two typed ends with the requested aggregation
    And JSON and BLX preserve the authored section

  Scenario: Retarget relationships without losing their presentation
    Given an authored class model
    When I retarget associations, generalizations and interface realizations
    Then ownership and endpoints follow the contracts without changing styles or labels
    And missing or ambiguous occurrences remain unattached
    And invalid endpoint types and manual routes reject changes atomically
    And JSON and BLX preserve the authored section

  Scenario: Edit association-end navigation knowledge
    Given an authored class model
    When I cycle an association end through navigable, notNavigable and unverified
    Then the Property Inspector and derived navigation agree with the canonical association
    And JSON and BLX preserve the authored section

  Scenario: Distinguish semantic ownership from diagram occurrences
    Given an authored class model
    When I place repeated occurrences and a same-named classifier in another diagram
    Then both Explorer orders have unique keys and preserve semantic ownership
    And JSON and BLX preserve the authored section

  Scenario: Add a note to the selected occurrence
    Given an authored class model
    When I add a note beside one of two occurrences
    Then ambiguous selections fail and the Comment link binds the selected occurrence
    And removing the link View preserves the semantic link and its hierarchy guard
    And JSON and BLX preserve the authored section

  Scenario: Delete owned content while preserving independent visual containers
    Given an authored class model
    When I remove nested occurrences and delete the root of an independent diagram
    Then semantic leaves disappear and unrelated visual siblings survive in order
    And surviving style and model references prevent deletion atomically
    And JSON and BLX preserve the authored section
