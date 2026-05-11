use std::collections::BTreeSet;
use std::io::Write;

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::cli::Format;
use crate::fields::ALL_FIELDS;
use crate::types::{RepoEntry, Scale};

pub fn render<W: Write>(
    format: Format,
    entries: &[RepoEntry],
    fields: &[&str],
    w: &mut W,
) -> Result<()> {
    match format {
        Format::Json => write_json(entries, fields, w),
        Format::Csv => write_delimited(entries, fields, b',', w),
        Format::Tsv => write_delimited(entries, fields, b'\t', w),
        Format::Markdown => write_markdown(entries, fields, w),
        Format::Ascii => write_ascii(entries, fields, w),
        Format::Html => write_html(entries, fields, false, w),
        Format::HtmlStyled => write_html(entries, fields, true, w),
    }
}

// --- JSON ---

fn write_json<W: Write>(entries: &[RepoEntry], fields: &[&str], w: &mut W) -> Result<()> {
    if fields.len() == ALL_FIELDS.len() {
        serde_json::to_writer_pretty(&mut *w, entries)?;
    } else {
        let mut json_keys: Vec<String> = Vec::new();
        let mut seen = BTreeSet::new();
        for f in fields {
            let k = flat_to_json_key(f).to_string();
            if seen.insert(k.clone()) {
                json_keys.push(k);
            }
        }
        let mut arr = Vec::with_capacity(entries.len());
        for e in entries {
            let v = serde_json::to_value(e)?;
            if let serde_json::Value::Object(map) = v {
                let mut filtered = serde_json::Map::new();
                for k in &json_keys {
                    if let Some(val) = map.get(k) {
                        filtered.insert(k.clone(), val.clone());
                    }
                }
                arr.push(serde_json::Value::Object(filtered));
            }
        }
        serde_json::to_writer_pretty(&mut *w, &arr)?;
    }
    writeln!(w)?;
    Ok(())
}

fn flat_to_json_key(flat: &str) -> &str {
    match flat {
        "last_commit_hash" | "last_commit_date" | "last_commit_message" => "last_commit",
        "uncommitted_files" | "uncommitted_insertions" | "uncommitted_deletions" => "uncommitted",
        other => other,
    }
}

// --- CSV / TSV / Markdown (flat columnar) ---

fn flat_value(e: &RepoEntry, field: &str) -> String {
    match field {
        "path" => e.path.clone(),
        "name" => e.name.clone(),
        "current_branch" => opt_str(&e.current_branch),
        "github_repo" => opt_str(&e.github_repo),
        "is_private" => opt_bool(e.is_private),
        "last_commit_hash" => e
            .last_commit
            .as_ref()
            .map(|c| c.hash.clone())
            .unwrap_or_default(),
        "last_commit_date" => e
            .last_commit
            .as_ref()
            .map(|c| c.date.clone())
            .unwrap_or_default(),
        "last_commit_message" => e
            .last_commit
            .as_ref()
            .map(|c| c.message.clone())
            .unwrap_or_default(),
        "has_uncommitted" => e.has_uncommitted.to_string(),
        "uncommitted_files" => opt_num(e.uncommitted.as_ref().map(|u| u.files as u64)),
        "uncommitted_insertions" => opt_num(e.uncommitted.as_ref().map(|u| u.insertions as u64)),
        "uncommitted_deletions" => opt_num(e.uncommitted.as_ref().map(|u| u.deletions as u64)),
        "unpushed_commits" => opt_num(e.unpushed_commits.map(|n| n as u64)),
        "unmerged_branches" => opt_num(e.unmerged_branches.map(|n| n as u64)),
        "open_issues" => opt_num(e.open_issues.map(|n| n as u64)),
        "open_prs" => opt_num(e.open_prs.map(|n| n as u64)),
        "loc" => opt_num(e.loc),
        "scale" => e.scale.map(scale_str).unwrap_or("").to_string(),
        "dir_size_bytes" => opt_num(e.dir_size_bytes),
        "has_readme" => e.has_readme.to_string(),
        "has_dockerfile" => e.has_dockerfile.to_string(),
        "compose_file" => opt_str(&e.compose_file),
        "compose_ports" => e
            .compose_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("|"),
        "compose_running" => e.compose_running.to_string(),
        "github_description" => opt_str(&e.github_description),
        "tech_tags" => e.tech_tags.join("|"),
        _ => String::new(),
    }
}

fn write_delimited<W: Write>(
    entries: &[RepoEntry],
    fields: &[&str],
    delim: u8,
    w: &mut W,
) -> Result<()> {
    let mut wtr = csv::WriterBuilder::new().delimiter(delim).from_writer(w);
    wtr.write_record(fields)?;
    for e in entries {
        let row: Vec<String> = fields.iter().map(|f| flat_value(e, f)).collect();
        wtr.write_record(row)?;
    }
    wtr.flush()?;
    Ok(())
}

fn write_markdown<W: Write>(entries: &[RepoEntry], fields: &[&str], w: &mut W) -> Result<()> {
    writeln!(w, "| {} |", fields.join(" | "))?;
    writeln!(
        w,
        "| {} |",
        fields.iter().map(|_| "---").collect::<Vec<_>>().join(" | ")
    )?;
    for e in entries {
        let cells: Vec<String> = fields
            .iter()
            .map(|f| escape_md_cell(flat_value(e, f)))
            .collect();
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

// --- HTML ---

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn write_html<W: Write>(
    entries: &[RepoEntry],
    fields: &[&str],
    styled: bool,
    w: &mut W,
) -> Result<()> {
    if styled {
        write!(w, "{}", HTML_STYLED_HEAD)?;
    }
    writeln!(w, "<table>")?;
    writeln!(w, "<thead><tr>")?;
    for f in fields {
        writeln!(w, "  <th>{}</th>", escape_html(f))?;
    }
    writeln!(w, "</tr></thead>")?;
    writeln!(w, "<tbody>")?;
    for e in entries {
        writeln!(w, "<tr>")?;
        for f in fields {
            writeln!(w, "  <td>{}</td>", escape_html(&flat_value(e, f)))?;
        }
        writeln!(w, "</tr>")?;
    }
    writeln!(w, "</tbody>")?;
    writeln!(w, "</table>")?;
    if styled {
        write!(w, "{}", HTML_STYLED_TAIL)?;
    }
    Ok(())
}

const HTML_STYLED_HEAD: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>reposnap</title>
<style>
:root { --bg: #0d1117; --fg: #e6edf3; --border: #30363d; --head-bg: #161b22; --row-hover: #1c2128; --accent: #58a6ff; }
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif; background: var(--bg); color: var(--fg); padding: 2rem; }
table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
th, td { padding: 0.5rem 0.75rem; border: 1px solid var(--border); text-align: left; white-space: nowrap; }
th { background: var(--head-bg); color: var(--accent); font-weight: 600; position: sticky; top: 0; }
tr:hover td { background: var(--row-hover); }
@media (prefers-color-scheme: light) {
  :root { --bg: #fff; --fg: #1f2328; --border: #d1d9e0; --head-bg: #f6f8fa; --row-hover: #f0f3f6; --accent: #0969da; }
}
</style>
</head>
<body>
"#;

const HTML_STYLED_TAIL: &str = "</body>\n</html>\n";

// --- ASCII (humanized, condensed) ---
//
// One ASCII column may correspond to several flat fields:
//   has_uncommitted / uncommitted_* → DIRTY
//   last_commit_hash / *_date / *_message → LAST COMMIT DATE
// Selecting any of those flat fields yields the single ASCII column once.

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum AsciiCol {
    Path,
    Name,
    Branch,
    Gh,
    Priv,
    LastCommit,
    Dirty,
    Ahead,
    Unmerged,
    Issues,
    Prs,
    Loc,
    Scale,
    Size,
    Readme,
    Dockerfile,
    Compose,
    Ports,
    Running,
    Desc,
    Tags,
}

fn flat_to_ascii_col(flat: &str) -> Option<AsciiCol> {
    match flat {
        "path" => Some(AsciiCol::Path),
        "name" => Some(AsciiCol::Name),
        "current_branch" => Some(AsciiCol::Branch),
        "github_repo" => Some(AsciiCol::Gh),
        "is_private" => Some(AsciiCol::Priv),
        "last_commit_hash" | "last_commit_date" | "last_commit_message" => {
            Some(AsciiCol::LastCommit)
        }
        "has_uncommitted"
        | "uncommitted_files"
        | "uncommitted_insertions"
        | "uncommitted_deletions" => Some(AsciiCol::Dirty),
        "unpushed_commits" => Some(AsciiCol::Ahead),
        "unmerged_branches" => Some(AsciiCol::Unmerged),
        "open_issues" => Some(AsciiCol::Issues),
        "open_prs" => Some(AsciiCol::Prs),
        "loc" => Some(AsciiCol::Loc),
        "scale" => Some(AsciiCol::Scale),
        "dir_size_bytes" => Some(AsciiCol::Size),
        "has_readme" => Some(AsciiCol::Readme),
        "has_dockerfile" => Some(AsciiCol::Dockerfile),
        "compose_file" => Some(AsciiCol::Compose),
        "compose_ports" => Some(AsciiCol::Ports),
        "compose_running" => Some(AsciiCol::Running),
        "github_description" => Some(AsciiCol::Desc),
        "tech_tags" => Some(AsciiCol::Tags),
        _ => None,
    }
}

fn ascii_header(c: AsciiCol) -> &'static str {
    match c {
        AsciiCol::Path => "PATH",
        AsciiCol::Name => "NAME",
        AsciiCol::Branch => "BRANCH",
        AsciiCol::Gh => "GH",
        AsciiCol::Priv => "PRIV",
        AsciiCol::LastCommit => "LAST COMMIT\nDATE",
        AsciiCol::Dirty => "DIRTY",
        AsciiCol::Ahead => "AHEAD",
        AsciiCol::Unmerged => "UNMERGED",
        AsciiCol::Issues => "ISSUES",
        AsciiCol::Prs => "PRS",
        AsciiCol::Loc => "LOC",
        AsciiCol::Scale => "SCALE",
        AsciiCol::Size => "SIZE",
        AsciiCol::Readme => "README",
        AsciiCol::Dockerfile => "DOCKERFILE",
        AsciiCol::Compose => "COMPOSE",
        AsciiCol::Ports => "PORTS",
        AsciiCol::Running => "RUNNING",
        AsciiCol::Desc => "DESC",
        AsciiCol::Tags => "TAGS",
    }
}

fn ascii_value(c: AsciiCol, e: &RepoEntry) -> String {
    match c {
        AsciiCol::Path => e.path.clone(),
        AsciiCol::Name => e.name.clone(),
        AsciiCol::Branch => e.current_branch.clone().unwrap_or_else(|| "-".into()),
        AsciiCol::Gh => e.github_repo.clone().unwrap_or_else(|| "-".into()),
        AsciiCol::Priv => match e.is_private {
            Some(true) => "Y".into(),
            Some(false) => "N".into(),
            None => "-".into(),
        },
        AsciiCol::LastCommit => e
            .last_commit
            .as_ref()
            .map(|c| c.date.split('T').next().unwrap_or("").to_string())
            .unwrap_or_else(|| "-".into()),
        AsciiCol::Dirty => match &e.uncommitted {
            Some(u) => format!("{}f+{}-{}", u.files, u.insertions, u.deletions),
            None => "-".into(),
        },
        AsciiCol::Ahead => num_or_dash(e.unpushed_commits.map(|n| n as u64)),
        AsciiCol::Unmerged => num_or_dash(e.unmerged_branches.map(|n| n as u64)),
        AsciiCol::Issues => num_or_dash(e.open_issues.map(|n| n as u64)),
        AsciiCol::Prs => num_or_dash(e.open_prs.map(|n| n as u64)),
        AsciiCol::Loc => e.loc.map(humanize_count).unwrap_or_else(|| "-".into()),
        AsciiCol::Scale => e.scale.map(scale_str).unwrap_or("-").into(),
        AsciiCol::Size => e
            .dir_size_bytes
            .map(humanize_bytes)
            .unwrap_or_else(|| "-".into()),
        AsciiCol::Readme => if e.has_readme { "Y" } else { "N" }.into(),
        AsciiCol::Dockerfile => if e.has_dockerfile { "Y" } else { "N" }.into(),
        AsciiCol::Compose => e.compose_file.clone().unwrap_or_else(|| "-".into()),
        AsciiCol::Ports => {
            if e.compose_ports.is_empty() {
                "-".into()
            } else {
                e.compose_ports
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            }
        }
        AsciiCol::Running => if e.compose_running { "Y" } else { "N" }.into(),
        AsciiCol::Desc => e
            .github_description
            .as_deref()
            .map(|s| truncate(s, 40))
            .unwrap_or_else(|| "-".into()),
        AsciiCol::Tags => {
            if e.tech_tags.is_empty() {
                "-".into()
            } else {
                e.tech_tags.join(",")
            }
        }
    }
}

fn write_ascii<W: Write>(entries: &[RepoEntry], fields: &[&str], w: &mut W) -> Result<()> {
    // Map flat fields to ASCII columns, dedup, preserve first-seen order.
    let mut cols: Vec<AsciiCol> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for f in fields {
        if let Some(c) = flat_to_ascii_col(f) {
            if seen.insert(c) {
                cols.push(c);
            }
        }
    }
    let headers: Vec<&str> = cols.iter().copied().map(ascii_header).collect();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers);
    for e in entries {
        let row: Vec<String> = cols.iter().copied().map(|c| ascii_value(c, e)).collect();
        table.add_row(row);
    }
    writeln!(w, "{table}")?;
    Ok(())
}

// --- helpers ---

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
