# MDE execution events

Observation is implemented outside `KERNEL`; kernel calculations do not accept traces, read observation clocks or invoke trace sinks. Create an `ExecutionTrace` per run with a caller-supplied unique run ID, optional model revision and event capacity, and pass it through the traced entry points. Read `events()` and `dropped()` and use `execution_trace_jsonl` to export JSON Lines; the host owns the destination. A nonzero dropped count means the trace is incomplete. IDs can be application-specific and should be handled as potentially sensitive; command payloads are never serialized. Allowlisted creation/ownership context records element/relation/member kind, owner/parent and diagram IDs. Returned diagnostic messages are recorded (up to 2,048 characters, control characters replaced by spaces); these messages may contain model data or filesystem paths. Review copied logs before sharing; this is bounded formatting, not automatic secret redaction. Runs are independent and may be correlated by their IDs. The per-run sequence is the observed order; `elapsed_ns` is monotonic elapsed time since the run began, not a wall-clock timestamp.

Current external observation covers structural model/diagram validation through wrappers in `observability/`, Cordis command application boundaries, and Cordis generation/export task boundaries. Diagnostics record returned issues, not every rule evaluated. Graph integrity validation, generation analysis, typed operation execution, Python script composition and the Rust backend have no internal tracing. The removed `*_traced` kernel variants are not alternative entry points.

Cordis publishes external events through `ModelingExecutionTraceEvent`. A bounded nonblocking queue forwards them to observers and stderr; overflow or an observer delivery failure/timeout produces a loss-count event. A timeout is counted conservatively because delivery may have occurred before the timeout. Forwarding is detached from the operation result. The opt-in `ModelingValidateDiagramTraced` request carries a caller run ID and capacity and returns events from the external validation wrapper. The MDE host bridge maintains a bounded `ExecutionJournal` with retained history, eviction and producer-loss counts, and live change subscriptions. Absence of internal events does not mean a kernel calculation did not run.

The Cordis host streams each default event to standard error prefixed `MDE_TRACE ` followed by a JSON object. Capture the application's standard error, filter lines with that prefix, and remove it to obtain JSON Lines for plotting. Set `BELLATRIX_MDE_TRACE=off` before starting the host to disable this output. The Cordis event batch remains available to observers. A JSON record with `record_type` set to `dropped` reports the affected run ID and count and means that run is incomplete. Treat logs as potentially sensitive because model IDs and returned errors may reveal project structure, names or paths.

Each Cordis JSON event also contains `project_id`. Each event contains `run_id`, optional `revision`, zero-based `sequence`, monotonic `elapsed_ns`, `component`, `action`, `phase`, optional `identity`, `dependencies` (source IDs), optional `outcome`, optional `message`, and optional `context` (structural metadata). Older JSON events without the new fields remain readable. Sort by run ID and sequence; match start/finish pairs by action and identity. The dependencies field is retained in the external event format, but kernel calculations no longer emit dependency events. Durations describe externally observed call boundaries. Rust generation and HTML export record only their Cordis task boundaries, not individual compiler stages or operations. Validation and generation use separate run IDs unless an external caller explicitly shares one.

The Cordis `diagram.validate.request` outcome reports whether the request completed; the external `diagram.validate` wrapper reports model validity. These are separate results, and the legacy `kernel` component label does not indicate instrumentation inside the kernel.

## Functional API requirement and observation effects

The [master plan functional contract](MDE_IMPLEMENTATION_PLAN.md#contrato-funcional-público-do-kernel-e-mini-compilador)
is a target requirement, not a claim that the current mutable `ExecutionTrace`
API satisfies the intended public boundary. Public consumers must use functions
with explicit inputs and results without depending on internal trace storage,
operation registries or compiler representations. This requirement does not
select the future input/output format, serialization, transport or ABI. The
JSON Lines behavior above describes the existing observation adapter only.

The explicitly named `kernel-api` directory is the sole public function-based
boundary. Consumers must not bypass it through internal modules or reexports.
Actual internal resource I/O belongs in `KERNEL/I_O`, including resource reads,
file publication and process execution performed by this component. Observation
clocks, event construction and trace export/delivery stay outside `KERNEL`,
including outside `KERNEL/I_O`. Optional measurement may wrap a public API call
before and after its calculation, never run inside it. `KERNEL/I_O` is internal implementation,
not a second public API. Functions exposed through `kernel-api` must declare
these effects; pure generation remains separate from effectful adapters even
when the latter are housed inside `KERNEL/I_O`. In-memory trace construction is also excluded from kernel calculations.
Verify dependencies and actual effectful calls, not
just directory names, without prescribing the future data representation.

For each public function, document its inputs, results, errors, preconditions,
postconditions and allowed effects. Distinguish pure computation from optional
observation and from external adapters that export events, read resources,
publish files or execute processes. Any observer callback has an explicit
effect contract and trust boundary; accepting a callback does not enforce
purity or sandbox its implementation.

A call must not retain execution state for later calls. Caller-supplied run
correlation and per-call temporary storage do not authorize hidden sessions or
shared mutable registries. Clocks used for observation, event delivery and
observer failures must not influence semantic results. Observed elapsed times
and delivery metadata are not part of deterministic semantic output or model
transformation correspondence.

Kernel tests must verify deterministic semantic results without instrumentation.
Tests of external observation adapters must compare results with observation
enabled, disabled, at capacity, and with failing observers. They must also check repeated A→B→A
calls, independent/concurrent runs, unchanged input, per-run isolation and
visible loss reporting. Generating a construction script must neither execute
it nor publish its artifacts. Record which functions are actually covered;
this requirement does not imply that all existing entry points are traced or
that these acceptance tests have already passed.

## One access point, existing producers

Use **Logs & diagnostics** in the MDE workbench. Copy on a row includes its
message and structural context; the header Copy includes validation, latest
feedback and all retained events, ignoring filters. Search includes messages
and context. The 2,000-event journal and producer loss counts remain bounded;
copy is a snapshot, not a durable log archive.

- `observability/execution_trace.rs` is the shared event construction and
  diagnostic formatting point. The public MDE observation handle delegates to it.
- `bellatrix_mde_cordis/src/mutation.rs` records every command boundary and
  returned failure, including snapshot/publication failures. Creation/ownership
  commands add allowlisted structural context; other commands retain their names.
- `diagrams.rs` and the MDE validation wrapper record request failures and
  returned presentation diagnostics, including their messages.
- `task_events.rs` records generation/export task failures, not internal stages.
- `trace_output.rs` remains the sole Cordis forwarding/stderr adapter;
  `execution_journal.rs` remains the host's retained history, including UI interactions.

Host `ExecutionJournal::record_result` uses the same trace formatter for
session edit/validate/save results, native save/MDJ-export outcomes (including
worker join failure), successful native document opening, and UI enqueue
rejections. These host-local events enter the document journal directly, like
UI interactions; they do not stream to stderr. Session events summarize the
result, while command events preserve the detailed command/run context.

This does not intercept arbitrary `eprintln!`/`log` output, opening/import failures
before a document bridge exists, or every UI precondition. Observer transport failures
remain stderr diagnostics plus loss counters, since forwarding them through the
failing transport would recurse. Rendering and kernel calculations are not
newly instrumented. Latest UI feedback is still copied separately and can be
replaced by the next operation; only the explicitly observed result boundaries
above retain their errors in history. Import diagnostics returned before bridge
creation and successful export cleanup warnings are not newly retained. No parallel
logger, global collector or instrumentation inside `KERNEL` was introduced.
