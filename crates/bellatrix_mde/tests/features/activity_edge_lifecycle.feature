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

  Scenario: Preserve Fork and Merge edge-family semantics through repair
    Given an imported Activity with homogeneous ControlFlows around a ForkNode or MergeNode
    When I change one incident edge to ObjectFlow
    Then the known mixture violates ForkNode-edges or MergeNode-edges from the pinned UML.xmi
    When I change all incident edges to ObjectFlows
    Then the same edge-family predicate is satisfied
    When I author an isolated node with no incident edges
    Then uniformity is vacuously true but the separate cardinality rule still fails
    When one self-loop supplies both incoming and outgoing roles
    Then the union contains one edge identity
    When I save, close and reopen every state using JSON and BLX and restore the original graph
    Then data and exact findings remain unchanged

  Scenario: Preserve provable Fork and Merge findings with unresolved incidence
    Given known ControlFlows and a possibly incident ObjectFlow with an unresolved endpoint
    Then edge-family uniformity is unverified rather than falsely violated
    When its opposite endpoint is known to reach the ForkNode or MergeNode
    Then its incident ObjectFlow identity proves a mixture despite the unresolved end
    And an additional unknown edge family cannot hide that proved violation
    When the known incident edges are homogeneous and one additional known incident edge has unknown family
    Then the family predicate reports only an unverified warning rather than a proved mixture
    When a possibly incident edge is instead known to share the ControlFlow family
    Then it cannot invalidate established uniformity
    When authored and canonical endpoints contradict each other
    Then uncertain incidence alone does not prove a mixture
    But another definitely incident ObjectFlow still proves the known mixture
    When I save, close and reopen every state through JSON and BLX and repair the graph
    Then all collections and diagnostic findings are repeatable

  Scenario: Preserve the Join input-existence conditional across repairs
    Given a JoinNode whose incoming and outgoing edges are ControlFlows
    When only its outgoing edge becomes ObjectFlow
    Then JoinNode-incoming_object_flow rejects the output family without an incoming ObjectFlow
    When one incoming edge becomes ObjectFlow while another remains ControlFlow
    Then the mixed inputs select the object-output branch and satisfy it
    When the outgoing edge becomes ControlFlow again
    Then the independently selected object branch fails
    When an isolated Join has empty input and output sets
    Then the family predicate is true and outgoing cardinality fails separately
    When one ControlFlow self-loop becomes ObjectFlow
    Then the same edge witnesses and satisfies each corresponding branch
    When I save, close and reopen each state through JSON and BLX and repair the model
    Then all data and findings repeat unchanged

  Scenario: Keep unresolved Join inputs distinct from proved absence
    Given a possibly incoming ObjectFlow with unresolved incidence
    Then neither control nor object output allows silently choosing the guard branch
    When the possible input is known ControlFlow
    Then it cannot change whether an incoming ObjectFlow exists
    When a definitely incoming ObjectFlow provides a witness
    Then unrelated unresolved inputs do not prevent selecting the object branch
    When a possible outgoing edge has the wrong family
    Then output conformance is unverified
    But a possible outgoing edge of the required family does not prevent proof
    And a definitely wrong outgoing edge still proves a violation
    When the incoming edge family itself is unknown
    Then known incidence alone cannot prove or disprove ObjectFlow existence
    When the guard is unknown and the outgoing set is empty or contains both families
    Then agreement of both branches proves true or false independently of all guard completions
    And empty-output cardinality remains a separate diagnostic
    When I save, close and reopen every state through JSON and BLX and repair the graph
    Then unknown data, proofs and authored records remain unchanged

  Scenario: Preserve Decision edge identity exclusion through repair
    Given an imported Activity with homogeneous DecisionNode ControlFlows
    When an ObjectFlow input is added and selected as decisionInputFlow
    Then only that identity is excluded from the incoming and outgoing union
    And another ObjectFlow still proves a mixed-family violation
    And null or a nonincident selector does not remove an incident edge
    When the selector is external, malformed or of the wrong type
    Then mixed-family evaluation is unverified rather than a fabricated violation
    And an already homogeneous union remains homogeneous under any exclusion
    And outgoing and self-loop identities are excluded without inventing incoming membership
    Then every state is saved and reopened as JSON and BLX with identical diagnostics
    And restoring the original document restores the baseline result

  Scenario: Preserve ObjectFlow distribution flags through repair
    Given an imported ObjectFlow as a canonical relation or object record
    And absent distribution flags use their normative false defaults without mutation
    When each Boolean combination is authored
    Then only both true violates ObjectFlow-is_multicast_or_is_multireceive
    When one or both flags are malformed
    Then property-value errors remain separate from the unverified rule
    And a known false proves the conjunction false in either operand order
    Then every state is saved and reopened as JSON and BLX with identical diagnostics
    And restoring the original document restores the baseline result

  Scenario: Preserve ObjectFlow selection source semantics through repair
    Given an imported ObjectFlow and a selection Behavior
    When selection is authored with an ObjectNode subclass source
    Then the selection source constraint is satisfied
    When the source is an Action then the selection source constraint is violated
    And absent or null selection does not constrain the source
    And an external selection reference proves presence without resolving its definition
    And an unresolved source remains unverified unless selection is absent
    And malformed selection retains its separate property-contract error
    Then canonical relation and object record edits survive JSON and BLX reopening
    And restoring the original document restores the baseline result
