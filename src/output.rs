use std::io::Write;

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::Format;
use crate::types::{RepoEntry, Scale};

pub fn render<W: Write>(format: Format, entries: &[RepoEntry], w: &mut W) -> Result<()> {
    match format {
        Format::Json => write_json(entries, w),
        Format::Csv => write_delimited(entries, b',', w),
        Format::Tsv => write_delimited(entries, b'\t', w),
        Format::Markdown => write_markdown(entries, w),
        Format::Ascii => write_ascii(entries, w),
    }
}

fn write_json<W: Write>(entries: &[RepoEntry], w: &mut W) -> Result<()> {
    serde_json::to_writer_pretty(&mut *w, entries)?;
    writeln!(w)?;
    Ok(())
}

const HEADERS: &[&str] = &[
    "path",
    "name",
    "github_repo",
    "github_description",
    "is_private",
    "last_commit_hash",
    "last_commit_date",
    "last_commit_message",
    "has_uncommitted",
    "uncommitted_files",
    "uncommitted_insertions",
    "uncommitted_deletions",
    "unpushed_commits",
    "unmerged_branches",
    "dir_size_bytes",
    "tech_tags",
    "has_readme",
    "scale",
    "loc",
    "has_dockerfile",
    "compose_file",
    "compose_ports",
    "compose_running",
    "open_issues",
    "open_prs",
];

fn full_row(e: &RepoEntry) -> Vec<String> {
    vec![
        e.path.clone(),
        e.name.clone(),
        opt_str(&e.github_repo),
        opt_str(&e.github_description),
        opt_bool(e.is_private),
        e.last_commit
            .as_ref()
            .map(|c| c.hash.clone())
            .unwrap_or_default(),
        e.last_commit
            .as_ref()
            .map(|c| c.date.clone())
            .unwrap_or_default(),
        e.last_commit
            .as_ref()
            .map(|c| c.message.clone())
            .unwrap_or_default(),
        e.has_uncommitted.to_string(),
        opt_num(e.uncommitted.as_ref().map(|u| u.files as u64)),
        opt_num(e.uncommitted.as_ref().map(|u| u.insertions as u64)),
        opt_num(e.uncommitted.as_ref().map(|u| u.deletions as u64)),
        opt_num(e.unpushed_commits.map(|n| n as u64)),
        opt_num(e.unmerged_branches.map(|n| n as u64)),
        opt_num(e.dir_size_bytes),
        e.tech_tags.join("|"),
        e.has_readme.to_string(),
        e.scale.map(scale_str).unwrap_or("").to_string(),
        opt_num(e.loc),
        e.has_dockerfile.to_string(),
        opt_str(&e.compose_file),
        e.compose_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("|"),
        e.compose_running.to_string(),
        opt_num(e.open_issues.map(|n| n as u64)),
        opt_num(e.open_prs.map(|n| n as u64)),
    ]
}

fn write_delimited<W: Write>(entries: &[RepoEntry], delim: u8, w: &mut W) -> Result<()> {
    let mut wtr = csv::WriterBuilder::new().delimiter(delim).from_writer(w);
    wtr.write_record(HEADERS)?;
    for e in entries {
        wtr.write_record(full_row(e))?;
    }
    wtr.flush()?;
    Ok(())
}

fn write_markdown<W: Write>(entries: &[RepoEntry], w: &mut W) -> Result<()> {
    writeln!(w, "| {} |", HEADERS.join(" | "))?;
    writeln!(
        w,
        "| {} |",
        HEADERS.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
    )?;
    for e in entries {
        let cells: Vec<String> = full_row(e).into_iter().map(escape_md_cell).collect();
        writeln!(w, "| {} |", cells.join(" | "))?;
    }
    Ok(())
}

fn escape_md_cell(s: String) -> String {
    if s.is_empty() {
        return String::new();
    }
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', " ")
}

const ASCII_HEADERS: &[&str] = &[
    "PATH",
    "DATE",
    "HASH",
    "MSG",
    "DIRTY",
    "AHEAD",
    "UNMERGED",
    "GH",
    "PRIV",
    "DESC",
    "ISSUES",
    "PRS",
    "LOC",
    "SCALE",
    "SIZE",
    "README",
    "DOCKERFILE",
    "COMPOSE",
    "PORTS",
    "RUNNING",
    "TAGS",
];

fn ascii_row(e: &RepoEntry) -> Vec<String> {
    let date = e
        .last_commit
        .as_ref()
        .map(|c| c.date.split('T').next().unwrap_or("").to_string())
        .unwrap_or_else(|| "-".into());
    let hash = e
        .last_commit
        .as_ref()
        .map(|c| c.hash.clone())
        .unwrap_or_else(|| "-".into());
    let msg = e
        .last_commit
        .as_ref()
        .map(|c| truncate(&c.message, 40))
        .unwrap_or_else(|| "-".into());
    let dirty = match &e.uncommitted {
        Some(u) => format!("{}f+{}-{}", u.files, u.insertions, u.deletions),
        None => "-".into(),
    };
    let ahead = num_or_dash(e.unpushed_commits.map(|n| n as u64));
    let unmerged = num_or_dash(e.unmerged_branches.map(|n| n as u64));
    let gh = e.github_repo.clone().unwrap_or_else(|| "-".into());
    let priv_ = match e.is_private {
        Some(true) => "Y",
        Some(false) => "N",
        None => "-",
    };
    let desc = e
        .github_description
        .as_deref()
        .map(|s| truncate(s, 40))
        .unwrap_or_else(|| "-".into());
    let issues = num_or_dash(e.open_issues.map(|n| n as u64));
    let prs = num_or_dash(e.open_prs.map(|n| n as u64));
    let loc = e.loc.map(humanize_count).unwrap_or_else(|| "-".into());
    let scale = e.scale.map(scale_str).unwrap_or("-").to_string();
    let size = e
        .dir_size_bytes
        .map(humanize_bytes)
        .unwrap_or_else(|| "-".into());
    let readme = if e.has_readme { "Y" } else { "N" };
    let dockerfile = if e.has_dockerfile { "Y" } else { "N" };
    let compose = e.compose_file.clone().unwrap_or_else(|| "-".into());
    let ports = if e.compose_ports.is_empty() {
        "-".into()
    } else {
        e.compose_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let running = if e.compose_running { "Y" } else { "N" };
    let tags = if e.tech_tags.is_empty() {
        "-".into()
    } else {
        e.tech_tags.join(",")
    };

    vec![
        e.path.clone(),
        date,
        hash,
        msg,
        dirty,
        ahead,
        unmerged,
        gh,
        priv_.into(),
        desc,
        issues,
        prs,
        loc,
        scale,
        size,
        readme.into(),
        dockerfile.into(),
        compose,
        ports,
        running.into(),
        tags,
    ]
}

fn write_ascii<W: Write>(entries: &[RepoEntry], w: &mut W) -> Result<()> {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(ASCII_HEADERS.to_vec());
    for e in entries {
        table.add_row(ascii_row(e));
    }
    writeln!(w, "{table}")?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.lines().next().unwrap_or("");
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}

fn num_or_dash(v: Option<u64>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "-".into())
}

fn humanize_count(n: u64) -> String {
    if n < 1_000 {
        format!("{n}")
    } else if n < 1_000_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

fn humanize_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if n < KB {
        format!("{n}B")
    } else if n < MB {
        format!("{:.1}K", n as f64 / KB as f64)
    } else if n < GB {
        format!("{:.1}M", n as f64 / MB as f64)
    } else {
        format!("{:.2}G", n as f64 / GB as f64)
    }
}

fn opt_str(v: &Option<String>) -> String {
    v.clone().unwrap_or_default()
}

fn opt_bool(v: Option<bool>) -> String {
    v.map(|b| b.to_string()).unwrap_or_default()
}

fn opt_num(v: Option<u64>) -> String {
    v.map(|n| n.to_string()).unwrap_or_default()
}

fn scale_str(s: Scale) -> &'static str {
    match s {
        Scale::Small => "small",
        Scale::Medium => "medium",
        Scale::Large => "large",
        Scale::Huge => "huge",
    }
}
