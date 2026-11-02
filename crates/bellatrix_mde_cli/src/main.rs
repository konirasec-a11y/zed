//! Headless entrypoint. Commands cross a typed Cordis event before execution.
mod graph_report;
mod runner;
use bellatrix_mde_cordis::{
    ModelingCommand, ModelingCompleted, ModelingCompletion, ModelingOperation, ModelingPlugin,
    ModelingRequested,
};
use bellatrix_mde_modeling::Scope;
use clap::{Parser, Subcommand};
use cordis::{Context, Plugin, PreparedPlugin, Routing};
use std::{error::Error, io, path::PathBuf};
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(
    name = "bellatrix-bellatrix-mde-modeling",
    about = "Bellatrix Modeling: graph, Rust and HTML CLI"
)]
struct Args {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    Check {
        source: PathBuf,
    },
    Inspect {
        source: PathBuf,
    },
    Analyze {
        source: PathBuf,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        relation: Option<String>,
    },
    Plan {
        source: PathBuf,
        #[arg(long = "element")]
        elements: Vec<String>,
        #[arg(long)]
        skeleton: bool,
    },
    Generate {
        source: PathBuf,
        #[arg(long, default_value = "rust")]
        target: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long = "element")]
        elements: Vec<String>,
        #[arg(long)]
        replace_generated: bool,
        #[arg(long)]
        skeleton: bool,
    },
    Export {
        source: PathBuf,
        #[arg(long, default_value = "html")]
        format: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        replace_generated: bool,
    },
    Convert {
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

fn scope(elements: Vec<String>) -> Scope {
    if elements.is_empty() {
        Scope::Project
    } else {
        Scope::Elements(elements)
    }
}

fn operation(action: Action) -> Result<ModelingOperation, io::Error> {
    Ok(match action {
        Action::Check { source } => ModelingOperation::Check { source },
        Action::Inspect { source } => ModelingOperation::Inspect { source },
        Action::Analyze {
            source,
            from,
            to,
            relation,
        } => {
            if to.is_some() && from.is_none() {
                return Err(io::Error::other("--to requires --from"));
            }
            ModelingOperation::Analyze {
                source,
                from,
                to,
                relation,
            }
        }
        Action::Plan {
            source,
            elements,
            skeleton,
        } => ModelingOperation::Plan {
            source,
            scope: scope(elements),
            skeleton,
        },
        Action::Generate {
            source,
            target,
            output,
            elements,
            replace_generated,
            skeleton,
        } => {
            if target != "rust" {
                return Err(io::Error::other("only Rust target is supported"));
            }
            ModelingOperation::Generate {
                source,
                scope: scope(elements),
                destination: output,
                replace: replace_generated,
                skeleton,
            }
        }
        Action::Export {
            source,
            format,
            output,
            replace_generated,
        } => {
            if format != "html" {
                return Err(io::Error::other("only HTML export is supported"));
            }
            ModelingOperation::ExportHtml {
                source,
                destination: output,
                replace: replace_generated,
            }
        }
        Action::Convert { source, output } => ModelingOperation::Convert {
            source,
            destination: output,
        },
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = Args::parse();
    let ctx = Context::new();
    let (tx, mut rx) = mpsc::channel(32);
    let (completed_tx, mut completed_rx) = mpsc::channel(32);
    let plugin = ModelingPlugin::new(tx, completed_tx);
    let input = plugin.prepare(())?;
    let fiber = ctx.spawn(PreparedPlugin::from_input(plugin, input)).await?;
    ctx.emit::<ModelingRequested>(
        Routing::Unscoped,
        ModelingCommand {
            correlation_id: 1,
            operation: operation(args.action)?,
        },
    )
    .await?;
    let command = tokio::time::timeout(std::time::Duration::from_secs(15), rx.recv())
        .await
        .map_err(|_| io::Error::other("engine request event timed out"))?
        .ok_or_else(|| io::Error::other("engine event channel closed"))?;
    let id = command.correlation_id;
    let result = tokio::task::spawn_blocking(move || runner::run(command.operation)).await?;
    let completion = ModelingCompletion {
        correlation_id: id,
        success: result.is_ok(),
        summary: match &result {
            Ok(message) | Err(message) => message.clone(),
        },
    };
    ctx.emit::<ModelingCompleted>(Routing::Unscoped, completion)
        .await?;
    let observed = tokio::time::timeout(std::time::Duration::from_secs(15), completed_rx.recv())
        .await
        .map_err(|_| io::Error::other("engine completion timed out"))?
        .ok_or_else(|| io::Error::other("engine completion event channel closed"))?;
    if observed.correlation_id != id {
        return Err(io::Error::other("correlation mismatch").into());
    }
    fiber.dispose().await?;
    match result {
        Ok(report) => {
            println!("{report}");
            Ok(())
        }
        Err(message) => Err(io::Error::other(message).into()),
    }
}
