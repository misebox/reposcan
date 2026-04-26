mod cli;
mod collect;
mod discover;
mod docker;
mod git;
mod github;
mod loc;
mod output;
mod tags;
mod types;
mod util;

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::Args;
use crate::discover::DiscoverOptions;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let root = args
        .root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let root = root.canonicalize().with_context(|| format!("invalid root: {}", root.display()))?;

    let opts = DiscoverOptions {
        include_nested: args.include_nested,
        extra_excludes: args.exclude.iter().cloned().collect::<HashSet<_>>(),
    };

    tracing::info!(root = %root.display(), "discovering repositories");
    let repos = discover::discover(&root, &opts)?;
    tracing::info!(count = repos.len(), "found repositories");

    let args_arc = Arc::new(args.clone());
    let mut entries = collect::collect_all(&root, repos, args_arc).await;

    if let Some(json_path) = &args.output {
        if args.merge {
            output::merge_user_fields(json_path, &mut entries);
        }
        output::write_json(json_path, &entries)
            .with_context(|| format!("write json: {}", json_path.display()))?;
        tracing::info!(path = %json_path.display(), n = entries.len(), "wrote JSON");
    } else if args.merge {
        tracing::warn!("--merge has no effect without --output");
    }

    if let Some(csv_path) = &args.csv {
        output::write_csv(csv_path, &entries)
            .with_context(|| format!("write csv: {}", csv_path.display()))?;
        tracing::info!(path = %csv_path.display(), "wrote CSV");
    }

    let wrote_file = args.output.is_some() || args.csv.is_some();
    if args.table || !wrote_file {
        output::print_table(&entries);
    }

    Ok(())
}
