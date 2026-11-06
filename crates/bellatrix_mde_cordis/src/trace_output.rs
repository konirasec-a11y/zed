//! Host adapter: bounded, nonblocking transfer from the pure engine to observers.
use crate::ExecutionEvent;
use bellatrix_mde::MdeExecutionTrace as ExecutionTrace;
use cordis::{Context, Routing};
use std::{
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::Duration,
};

pub(crate) fn start(
    context: Context,
    trace: &mut ExecutionTrace,
    project_id: bellatrix_project::ProjectId,
) -> tokio::task::JoinHandle<()> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<ExecutionEvent>(1024);
    let dropped = Arc::new(AtomicU64::new(0));
    let sink_dropped = Arc::clone(&dropped);
    trace.set_sink(move |event| {
        if sender.try_send(event.clone()).is_err() {
            sink_dropped.fetch_add(1, Ordering::Relaxed);
        }
    });
    let run_id = trace.run_id().to_owned();
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            let delivery = context.emit_parallel::<crate::diagrams::ModelingExecutionTraceEvent>(
                Routing::Unscoped,
                crate::diagrams::ModelingExecutionTrace {
                    project_id: project_id.clone(),
                    run_id: run_id.clone(),
                    events: vec![event],
                    dropped: 0,
                },
            );
            match tokio::time::timeout(Duration::from_millis(100), delivery).await {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    dropped.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Bellatrix trace observer failure: {error}");
                }
                Err(_) => {
                    dropped.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Bellatrix trace observer timed out; delivery is uncertain");
                }
            }
        }
        let count = dropped.load(Ordering::Relaxed);
        if count > 0 {
            let notice = context.emit_parallel::<crate::diagrams::ModelingExecutionTraceEvent>(
                Routing::Unscoped,
                crate::diagrams::ModelingExecutionTrace {
                    project_id,
                    run_id,
                    events: Vec::new(),
                    dropped: count,
                },
            );
            match tokio::time::timeout(Duration::from_millis(100), notice).await {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => eprintln!("Bellatrix trace loss notice failed: {error}"),
                Err(_) => eprintln!("Bellatrix trace loss notice timed out"),
            }
        }
    })
}

static OUTPUT: OnceLock<Option<SyncSender<crate::ModelingExecutionTrace>>> = OnceLock::new();
static OUTPUT_DROPPED: AtomicU64 = AtomicU64::new(0);

/// Terminal I/O must not hold up GPUI or retention when stderr is a slow pipe.
pub(crate) fn write(batch: &crate::ModelingExecutionTrace) {
    if std::env::var_os("BELLATRIX_MDE_TRACE").is_some_and(|value| value == "off") {
        return;
    }
    let sender = OUTPUT.get_or_init(|| {
        let (sender, receiver) = sync_channel::<crate::ModelingExecutionTrace>(1024);
        match std::thread::Builder::new()
            .name("mde-trace-output".into())
            .spawn(move || {
                while let Ok(batch) = receiver.recv() {
                    write_batch(&batch);
                    let dropped = OUTPUT_DROPPED.swap(0, Ordering::Relaxed);
                    if dropped > 0 {
                        eprintln!(
                            "MDE_TRACE {}",
                            serde_json::json!({
                                "record_type": "terminal_output_dropped", "count": dropped
                            })
                        );
                    }
                }
            }) {
            Ok(_) => Some(sender),
            Err(error) => {
                eprintln!("Bellatrix terminal trace worker unavailable: {error}");
                None
            }
        }
    });
    if sender
        .as_ref()
        .is_none_or(|sender| sender.try_send(batch.clone()).is_err())
    {
        OUTPUT_DROPPED.fetch_add(
            batch.events.len() as u64 + u64::from(batch.dropped > 0),
            Ordering::Relaxed,
        );
    }
}

fn write_batch(batch: &crate::ModelingExecutionTrace) {
    for event in &batch.events {
        match serde_json::to_value(event) {
            Ok(mut value) => {
                if let Some(fields) = value.as_object_mut() {
                    fields.insert(
                        "project_id".into(),
                        serde_json::json!(batch.project_id.as_str()),
                    );
                }
                eprintln!("MDE_TRACE {value}");
            }
            Err(error) => eprintln!("Bellatrix trace serialization error: {error}"),
        }
    }
    if batch.dropped > 0 {
        eprintln!(
            "MDE_TRACE {}",
            serde_json::json!({
                "record_type": "dropped", "project_id": batch.project_id.as_str(),
                "run_id": batch.run_id, "count": batch.dropped
            })
        );
    }
}
