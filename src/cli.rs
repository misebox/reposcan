use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "reposcan", about = "Recursively scan git repositories and emit metadata", version)]
pub struct Args {
    /// Root directory to scan (default: current directory)
    pub root: Option<PathBuf>,

    /// JSON output path
    #[arg(long, default_value = "./reposcan.json")]
    pub output: PathBuf,

    /// Also write CSV to this path
    #[arg(long)]
    pub csv: Option<PathBuf>,

    /// Skip GitHub (gh) lookups
    #[arg(long)]
    pub no_github: bool,

    /// Skip Docker runtime checks (compose ps); compose file parsing still runs
    #[arg(long)]
    pub no_docker: bool,

    /// Skip line-of-code counting
    #[arg(long)]
    pub no_loc: bool,

    /// Descend into nested git repositories
    #[arg(long)]
    pub include_nested: bool,

    /// Additional directory names to exclude (repeatable)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Number of repositories to scan in parallel
    #[arg(long, default_value_t = 4)]
    pub concurrency: usize,

    /// Merge with existing JSON, preserving user-edited fields and manual tags
    #[arg(long)]
    pub merge: bool,

    /// Print a summary table to stdout
    #[arg(long)]
    pub table: bool,
}
