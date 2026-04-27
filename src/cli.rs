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
    name = "reposnap",
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

    /// Comma-separated fields to include. Use 'all' for default. Group aliases:
    /// @minimal, @activity, @github. Example: --fields path,name,@activity
    #[arg(long, value_delimiter = ',')]
    pub fields: Vec<String>,

    /// Sort by field(s); prefix with '-' for descending. Comma separated.
    /// Example: --sort=-last_commit_date,path
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub sort: Vec<String>,

    /// Show only the first N repositories (after sort/filter)
    #[arg(long)]
    pub limit: Option<usize>,

    /// Show only repositories with uncommitted changes
    #[arg(long)]
    pub only_dirty: bool,

    /// Show only repositories with unpushed commits
    #[arg(long)]
    pub only_unpushed: bool,

    /// Show only repositories whose tech_tags include this tag (repeatable, AND)
    #[arg(long)]
    pub only_tag: Vec<String>,

    /// Filter expressions like 'field<op>value'. Operators: =, !=, >, <, >=, <=, ~.
    /// Repeatable or comma-separated; all conditions are ANDed.
    /// Examples: --filter scale=large --filter ahead>0 --filter tech_tags~rust
    #[arg(long, value_delimiter = ',', allow_hyphen_values = true)]
    pub filter: Vec<String>,

    /// Suppress tracing log output (errors only)
    #[arg(long, short = 'q')]
    pub quiet: bool,

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
