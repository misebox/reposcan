use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};

use crate::types::{RepoEntry, Scale};

pub fn write_json(path: &Path, entries: &[RepoEntry]) -> Result<()> {
    let f = std::fs::File::create(path)?;
    serde_json::to_writer_pretty(&f, entries)?;
    use std::io::Write;
    let mut f = f;
    writeln!(f)?;
    Ok(())
}

pub fn write_csv(path: &Path, entries: &[RepoEntry]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record([
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
    ])?;
    for e in entries {
        wtr.write_record([
            e.path.as_str(),
            e.name.as_str(),
            opt(&e.github_repo),
            opt(&e.github_description),
            &opt_bool(e.is_private),
            e.last_commit.as_ref().map(|c| c.hash.as_str()).unwrap_or(""),
            e.last_commit.as_ref().map(|c| c.date.as_str()).unwrap_or(""),
            e.last_commit.as_ref().map(|c| c.message.as_str()).unwrap_or(""),
            &e.has_uncommitted.to_string(),
            &opt_num(e.uncommitted.as_ref().map(|u| u.files as u64)),
            &opt_num(e.uncommitted.as_ref().map(|u| u.insertions as u64)),
            &opt_num(e.uncommitted.as_ref().map(|u| u.deletions as u64)),
            &opt_num(e.unpushed_commits.map(|n| n as u64)),
            &opt_num(e.unmerged_branches.map(|n| n as u64)),
            &opt_num(e.dir_size_bytes),
            &e.tech_tags.join("|"),
            &e.has_readme.to_string(),
            &e.scale.map(scale_str).unwrap_or("").to_string(),
            &opt_num(e.loc),
            &e.has_dockerfile.to_string(),
            opt(&e.compose_file),
            &e
                .compose_ports
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("|"),
            &e.compose_running.to_string(),
            &opt_num(e.open_issues.map(|n| n as u64)),
            &opt_num(e.open_prs.map(|n| n as u64)),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn print_table(entries: &[RepoEntry]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["PATH", "LAST COMMIT", "DIRTY", "DOCKER", "TAGS", "SCALE"]);

    for e in entries {
        let last = e
            .last_commit
            .as_ref()
            .map(|c| c.date.split('T').next().unwrap_or("").to_string())
            .unwrap_or_default();
        let dirty = match &e.uncommitted {
            Some(u) => format!("{}files", u.files),
            None => "-".into(),
        };
        let docker = if e.compose_running {
            "running"
        } else if e.has_dockerfile || e.compose_file.is_some() {
            "yes"
        } else {
            "-"
        };
        let tags = e.tech_tags.join(",");
        let scale = e.scale.map(scale_str).unwrap_or("-");
        table.add_row(vec![
            e.path.as_str(),
            &last,
            &dirty,
            docker,
            &tags,
            scale,
        ]);
    }
    println!("{table}");
}

pub fn merge_user_fields(path: &Path, fresh: &mut [RepoEntry]) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let Ok(prev): Result<Vec<RepoEntry>, _> = serde_json::from_str(&text) else { return };
    let mut prev_by_path: HashMap<String, RepoEntry> =
        prev.into_iter().map(|e| (e.path.clone(), e)).collect();

    for entry in fresh.iter_mut() {
        if let Some(p) = prev_by_path.remove(&entry.path) {
            let seen: std::collections::HashSet<String> =
                entry.tech_tags.iter().cloned().collect();
            for t in p.tech_tags {
                if !seen.contains(&t) {
                    entry.tech_tags.push(t);
                }
            }
            // Preserve user-edited description if newer fetch returned None.
            if entry.github_description.is_none() && p.github_description.is_some() {
                entry.github_description = p.github_description;
            }
        }
    }
}

fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("")
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
