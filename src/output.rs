use std::io::Write;

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::{Args, Format};
use crate::types::{RepoEntry, Scale};

#[derive(Copy, Clone, PartialEq, Eq)]
enum Group {
    Always,
    Github,
    Docker,
    Loc,
}

#[derive(Copy, Clone)]
struct Mask {
    no_github: bool,
    no_docker: bool,
    no_loc: bool,
}

impl Mask {
    fn from(args: &Args) -> Self {
        Self {
            no_github: args.no_github,
            no_docker: args.no_docker,
            no_loc: args.no_loc,
        }
    }
    fn keep(&self, g: Group) -> bool {
        match g {
            Group::Always => true,
            Group::Github => !self.no_github,
            Group::Docker => !self.no_docker,
            Group::Loc => !self.no_loc,
        }
    }
}

pub fn render<W: Write>(
    format: Format,
    entries: &[RepoEntry],
    args: &Args,
    w: &mut W,
) -> Result<()> {
    let mask = Mask::from(args);
    match format {
        Format::Json => write_json(entries, mask, w),
        Format::Csv => write_delimited(entries, mask, b',', w),
        Format::Tsv => write_delimited(entries, mask, b'\t', w),
        Format::Markdown => write_markdown(entries, mask, w),
        Format::Ascii => write_ascii(entries, mask, w),
    }
}

fn write_json<W: Write>(entries: &[RepoEntry], mask: Mask, w: &mut W) -> Result<()> {
    let mut arr = Vec::with_capacity(entries.len());
    for e in entries {
        let mut v = serde_json::to_value(e)?;
        if let serde_json::Value::Object(map) = &mut v {
            if mask.no_github {
                for k in ["github_repo", "github_description", "is_private", "open_issues", "open_prs"] {
                    map.remove(k);
                }
            }
            if mask.no_docker {
                for k in ["has_dockerfile", "compose_file", "compose_ports", "compose_running"] {
                    map.remove(k);
                }
            }
            if mask.no_loc {
                for k in ["loc", "scale"] {
                    map.remove(k);
                }
            }
        }
        arr.push(v);
    }
    serde_json::to_writer_pretty(&mut *w, &arr)?;
    writeln!(w)?;
    Ok(())
}

const FLAT_COLUMNS: &[(&str, Group)] = &[
    ("path", Group::Always),
    ("name", Group::Always),
    ("github_repo", Group::Github),
    ("github_description", Group::Github),
    ("is_private", Group::Github),
    ("last_commit_hash", Group::Always),
    ("last_commit_date", Group::Always),
    ("last_commit_message", Group::Always),
    ("has_uncommitted", Group::Always),
    ("uncommitted_files", Group::Always),
    ("uncommitted_insertions", Group::Always),
    ("uncommitted_deletions", Group::Always),
    ("unpushed_commits", Group::Always),
    ("unmerged_branches", Group::Always),
    ("dir_size_bytes", Group::Always),
    ("tech_tags", Group::Always),
    ("has_readme", Group::Always),
    ("scale", Group::Loc),
    ("loc", Group::Loc),
    ("has_dockerfile", Group::Docker),
    ("compose_file", Group::Docker),
    ("compose_ports", Group::Docker),
    ("compose_running", Group::Docker),
    ("open_issues", Group::Github),
    ("open_prs", Group::Github),
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

fn flat_active(mask: Mask) -> Vec<usize> {
    FLAT_COLUMNS
        .iter()
        .enumerate()
        .filter(|(_, (_, g))| mask.keep(*g))
        .map(|(i, _)| i)
        .collect()
}

fn pick<T: Clone>(src: &[T], idx: &[usize]) -> Vec<T> {
    idx.iter().map(|i| src[*i].clone()).collect()
}

fn write_delimited<W: Write>(
    entries: &[RepoEntry],
    mask: Mask,
    delim: u8,
    w: &mut W,
) -> Result<()> {
    let active = flat_active(mask);
    let headers: Vec<&str> = active.iter().map(|i| FLAT_COLUMNS[*i].0).collect();
    let mut wtr = csv::WriterBuilder::new().delimiter(delim).from_writer(w);
    wtr.write_record(&headers)?;
    for e in entries {
        let row = full_row(e);
        wtr.write_record(pick(&row, &active))?;
    }
    wtr.flush()?;
    Ok(())
}

fn write_markdown<W: Write>(entries: &[RepoEntry], mask: Mask, w: &mut W) -> Result<()> {
    let active = flat_active(mask);
    let headers: Vec<&str> = active.iter().map(|i| FLAT_COLUMNS[*i].0).collect();
    writeln!(w, "| {} |", headers.join(" | "))?;
    writeln!(
        w,
        "| {} |",
        headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
    )?;
    for e in entries {
        let row = full_row(e);
        let cells: Vec<String> = pick(&row, &active).into_iter().map(escape_md_cell).collect();
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

const ASCII_COLUMNS: &[(&str, Group)] = &[
    ("PATH", Group::Always),
    ("LAST COMMIT\nDATE", Group::Always),
    ("DIRTY", Group::Always),
    ("AHEAD", Group::Always),
    ("UNMERGED", Group::Always),
    ("GH", Group::Github),
    ("PRIV", Group::Github),
    ("DESC", Group::Github),
    ("ISSUES", Group::Github),
    ("PRS", Group::Github),
    ("LOC", Group::Loc),
    ("SCALE", Group::Loc),
    ("SIZE", Group::Always),
    ("README", Group::Always),
    ("DOCKERFILE", Group::Docker),
    ("COMPOSE", Group::Docker),
    ("PORTS", Group::Docker),
    ("RUNNING", Group::Docker),
    ("TAGS", Group::Always),
];

fn ascii_row(e: &RepoEntry) -> Vec<String> {
    let date = e
        .last_commit
        .as_ref()
        .map(|c| c.date.split('T').next().unwrap_or("").to_string())
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

fn write_ascii<W: Write>(entries: &[RepoEntry], mask: Mask, w: &mut W) -> Result<()> {
    let active: Vec<usize> = ASCII_COLUMNS
        .iter()
        .enumerate()
        .filter(|(_, (_, g))| mask.keep(*g))
        .map(|(i, _)| i)
        .collect();
    let headers: Vec<&str> = active.iter().map(|i| ASCII_COLUMNS[*i].0).collect();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers);
    for e in entries {
        let row = ascii_row(e);
        table.add_row(pick(&row, &active));
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
