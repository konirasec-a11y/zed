//! Host adapter: bounded, nonblocking transfer from the pure engine to observers.
use crate::ExecutionEvent;
use bellatrix_mde::MdeExecutionTrace as ExecutionTrace;
use cordis::{Context, Routing};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
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
        let output = std::env::var_os("BELLATRIX_MDE_TRACE").is_none_or(|value| value != "off");
        while let Some(event) = receiver.recv().await {
            if output {
                match serde_json::to_value(&event) {
                    Ok(mut value) => {
                        if let Some(fields) = value.as_object_mut() {
                            fields.insert(
                                "project_id".into(),
                                serde_json::json!(project_id.as_str()),
                            );
                        }
                        eprintln!("MDE_TRACE {value}");
                    }
                    Err(error) => eprintln!("Bellatrix trace serialization error: {error}"),
                }
            }
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
            if output {
                eprintln!(
                    "MDE_TRACE {}",
                    serde_json::json!({"record_type": "dropped", "project_id": project_id.as_str(), "run_id": run_id, "count": count})
                );
            }
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
