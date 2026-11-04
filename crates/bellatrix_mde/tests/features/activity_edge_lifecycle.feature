# language: en
Feature: ControlFlow object-node endpoint semantics across the authored Activity lifecycle
  Normative source: docs/bellatrix/standards/uml/20161101/UML.xmi, ControlFlow-object_nodes.
  Both source and target that conform to ObjectNode must have isControlType=true.
  ObjectNode-isControlType defaults to false; subclasses including Pins are covered.
  The Rust integration harness executes these full flows through the public pure API.
  No UI or unit-level validator is exercised; implemented checks are not full UML certification.

  Scenario: Repair a ControlFlow between typed Pins through persistence
    Given a valid imported Activity with an ObjectFlow between OutputPin and InputPin
    When I change the edge to ControlFlow with omitted isControlType properties
    Then the rule reports exactly the proved endpoint violation without materializing defaults
    When I set the source and target independently to true and false
    Then both ends are checked and only two allowed endpoints satisfy the conjunction
    When an authored Boolean is malformed or a stored endpoint contradicts the canonical relation
    Then malformed data retains its property-contract error without guessing a Boolean
    And contradictory endpoints leave the rule unverified rather than falsely violated
    But an independently known violating opposite endpoint still proves the conjunction false
    When I save, close and reopen each authored state through JSON and BLX
    Then data and exact findings survive unchanged
    When I repair the Boolean and endpoint and repeat the original ObjectFlow request
    Then earlier invalid calls cannot change the accepted findings

  Scenario: Reopen object-record ControlFlows with unresolved endpoints
    Given an imported Activity whose edge is stored as a ControlFlow object with source and target references
    Then the same inherited Pin and default-Boolean semantics apply
    When its source is an unresolved external href and its target is allowed
    Then the rule remains explicitly unverified rather than invalid or silently satisfied
    When its known target violates the rule
    Then the unknown source cannot hide the proved violation
    When a local endpoint has an unsupported UML metaclass
    Then the rule remains unverified without guessing its type
    When I save, close and reopen every state through JSON and BLX and repair the endpoint
    Then authored data and findings remain deterministic
