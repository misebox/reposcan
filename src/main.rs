mod cli;
mod collect;
mod discover;
mod docker;
mod git;
mod github;
mod loc;
mod output;
mod tags;
mod tools;
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
    let _ = tracing_log::LogTracer::init();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tokei=error")),
        )
        .with_writer(std::io::stderr)
        .try_init();

    let mut args = Args::parse();

    if args.diagnose {
        tools::print_diagnosis();
        return Ok(());
    }

    check_tools(&mut args)?;

    let root = args
        .root
        .clone()
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let root = root
        .canonicalize()
        .with_context(|| format!("invalid root: {}", root.display()))?;

    let opts = DiscoverOptions {
        include_nested: args.include_nested,
        extra_excludes: args.exclude.iter().cloned().collect::<HashSet<_>>(),
    };

    tracing::info!(root = %root.display(), "discovering repositories");
    let repos = discover::discover(&root, &opts)?;
    tracing::info!(count = repos.len(), "found repositories");

    let args_arc = Arc::new(args.clone());
    let entries = collect::collect_all(&root, repos, args_arc).await;

    match &args.output {
        Some(path) => {
            let mut f = std::fs::File::create(path)
                .with_context(|| format!("create {}", path.display()))?;
            output::render(args.format, &entries, &args, &mut f)?;
            tracing::info!(path = %path.display(), n = entries.len(), format = ?args.format, "wrote output");
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            output::render(args.format, &entries, &args, &mut handle)?;
        }
    }

    Ok(())
}

fn check_tools(args: &mut Args) -> Result<()> {
    if !tools::is_available("git") {
        anyhow::bail!("git not found in PATH (required). Run `reposcan --diagnose` for help.");
    }
    if !args.no_github && !tools::is_available("gh") {
        args.no_github = true;
    }
    if !args.no_docker && !tools::is_available("docker") {
        args.no_docker = true;
    }
    Ok(())
}
