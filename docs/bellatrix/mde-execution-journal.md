# MDE execution journal

The Cordis adapter, native UI interactions and host operation results use one
bounded `ExecutionJournal` per active project. The `ExecutionJournals` service is
owned by `MdePlugin`; bridges acquire the same journal rather than registering
additional trace observers. Weak registry entries do not keep closed documents
alive. Without a UI owner, headless events still reach terminal output, but their
journal is not retained for a future UI session.

## Producers and consumers

- `trace_output::start` transfers engine/Cordis events through the existing bounded
  channel and `ModelingExecutionTraceEvent`. The plugin's single observer appends
  them to the project's journal. The forwarder no longer prints a separate copy.
- `record_interaction` appends stable UI action names and model IDs through that
  same boundary, including selection, clicks, command enqueue and drag start.
- `record_result` appends host save/export and session results, preserving returned
  errors and the existing bounded diagnostic formatter.
- `ExecutionJournal::append` retains events and publishes `MDE_TRACE` JSON lines
  with project ID through a single nonblocking terminal queue (1,024 batches).
  Slow stderr cannot block UI capture; terminal-only losses are reported as
  `terminal_output_dropped`, while the journal retains those events. Pending
  terminal output is best-effort at process exit. `BELLATRIX_MDE_TRACE=off`
  disables terminal output only.
- The former execution dashboard, now embedded in the sidebar, reads the journal
  snapshot. Its copy action includes all retained snapshot events, regardless of
  filters, plus validation and feedback. Pausing freezes that snapshot, not capture.

Only 2,000 events and 100 producer-loss notices are retained. Eviction and producer
loss counters remain visible in the panel and copied report. Terminal output can
contain older events that have since been evicted or cleared from the journal.
Observer delivery errors/timeouts still use stderr as an emergency fallback:
reporting failure through the failing observer itself would recurse. Low-level
GPUI rendering errors and errors before a document journal exists are not claimed
as captured here. The pure engine's JSONL formatter remains available for explicit
headless trace exports; generated artifact source mappings named `trace` are not
runtime logs and are unchanged.

## Pointer capture

`update_gesture` requires the left mouse button and an active dragging gesture.
It records a start only once after meaningful movement, not every move event.
Hovering does not produce interaction events. Completion still travels through the
existing command enqueue/result path; successful enqueue is not successful apply.
No typed text or document payload is added by interaction capture. Diagnostic
messages may include paths or model data and should be reviewed before sharing.
