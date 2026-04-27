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

    // ---- Output ----
    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Ascii, help_heading = "Output")]
    pub format: Format,

    /// Write to this path instead of stdout
    #[arg(long, help_heading = "Output")]
    pub output: Option<PathBuf>,

    /// Comma-separated fields to include.
    ///
    /// Use 'all' for the default set.
    /// Group aliases: @minimal, @activity, @github
    /// Example: --fields path,name,@activity
    #[arg(
        long,
        value_delimiter = ',',
        verbatim_doc_comment,
        help_heading = "Output"
    )]
    pub fields: Vec<String>,

    /// Suppress tracing log output (errors only)
    #[arg(long, short = 'q', help_heading = "Output")]
    pub quiet: bool,

    // ---- Filter / sort ----
    /// Filter expression 'field<op>value' (repeatable, ANDed).
    ///
    /// Operators: =, !=, >, <, >=, <=, ~  (~ = substring / array contains)
    ///
    /// Fields (canonical / alias):
    ///   path  name  current_branch (branch)
    ///   github_repo (gh)  is_private (priv)  github_description (desc)
    ///   last_commit_hash  last_commit_date  last_commit_message
    ///   has_uncommitted (dirty)  uncommitted_files  uncommitted_insertions  uncommitted_deletions
    ///   unpushed_commits (ahead)  unmerged_branches (unmerged)
    ///   open_issues  open_prs
    ///   loc  scale  dir_size_bytes (size)
    ///   has_readme (readme)  has_dockerfile (dockerfile)
    ///   compose_file (compose)  compose_ports (ports)  compose_running (running)
    ///   tech_tags (tags)
    ///
    /// Type semantics:
    ///   string: =, !=, ~  (date strings are ISO so lexical = chronological)
    ///   bool:   =, !=     (true|false|yes|no|1|0, case-insensitive)
    ///   number: =, !=, >, <, >=, <=
    ///   scale:  =, !=, >, <, >=, <=  (small < medium < large < huge)
    ///   array:  ~          (contains)
    ///
    /// Null fields never match any condition.
    /// Values may contain commas (use --filter twice for AND).
    ///
    /// Examples:
    ///   --filter ahead>0
    ///   --filter scale>=large
    ///   --filter "last_commit_date<2025-01-01"
    ///   --filter tags~rust --filter tags~wasm
    #[arg(
        long,
        allow_hyphen_values = true,
        verbatim_doc_comment,
        help_heading = "Filter / sort"
    )]
    pub filter: Vec<String>,

    /// Show only repositories with uncommitted changes
    #[arg(long, help_heading = "Filter / sort")]
    pub only_dirty: bool,

    /// Show only repositories with unpushed commits
    #[arg(long, help_heading = "Filter / sort")]
    pub only_unpushed: bool,

    /// Show only repositories whose tech_tags include this tag (repeatable, AND)
    #[arg(long, help_heading = "Filter / sort")]
    pub only_tag: Vec<String>,

    /// Sort by field(s); '-' prefix = descending; comma separated.
    ///
    /// Example: --sort=-last_commit_date,path
    #[arg(
        long,
        value_delimiter = ',',
        allow_hyphen_values = true,
        verbatim_doc_comment,
        help_heading = "Filter / sort"
    )]
    pub sort: Vec<String>,

    /// Show only the first N repositories (after sort/filter)
    #[arg(long, help_heading = "Filter / sort")]
    pub limit: Option<usize>,

    // ---- Scan ----
    /// Descend into nested git repositories
    #[arg(long, help_heading = "Scan")]
    pub include_nested: bool,

    /// Additional directory names to exclude (repeatable)
    #[arg(long, help_heading = "Scan")]
    pub exclude: Vec<String>,

    /// Number of repositories to scan in parallel
    #[arg(long, default_value_t = 4, help_heading = "Scan")]
    pub concurrency: usize,

    /// Skip GitHub (gh) lookups
    #[arg(long, help_heading = "Scan")]
    pub no_github: bool,

    /// Skip Docker runtime checks (compose ps); compose file parsing still runs
    #[arg(long, help_heading = "Scan")]
    pub no_docker: bool,

    /// Skip line-of-code counting
    #[arg(long, help_heading = "Scan")]
    pub no_loc: bool,

    // ---- Misc ----
    /// Print external tool status and install hints, then exit
    #[arg(long, help_heading = "Diagnostics")]
    pub diagnose: bool,
}
