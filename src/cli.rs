use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum Format {
    Ascii,
    Json,
    Csv,
    Tsv,
    Markdown,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "reposcan",
    about = "Recursively scan git repositories and emit metadata",
    version
)]
pub struct Args {
    /// Root directory to scan (default: current directory)
    pub root: Option<PathBuf>,

    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Ascii)]
    pub format: Format,

    /// Write to this path instead of stdout
    #[arg(long)]
    pub output: Option<PathBuf>,

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

    /// Print external tool status and install hints, then exit
    #[arg(long)]
    pub diagnose: bool,
}
