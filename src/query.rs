use std::cmp::Ordering;

use anyhow::{anyhow, Result};

use crate::cli::Args;
use crate::types::{RepoEntry, Scale};

pub fn apply(args: &Args, mut entries: Vec<RepoEntry>) -> Result<Vec<RepoEntry>> {
    entries.retain(|e| keep(args, e));
    sort(&args.sort, &mut entries)?;
    if let Some(n) = args.limit {
        entries.truncate(n);
    }
    Ok(entries)
}

fn keep(args: &Args, e: &RepoEntry) -> bool {
    if args.only_dirty && !e.has_uncommitted {
        return false;
    }
    if args.only_unpushed && e.unpushed_commits.unwrap_or(0) == 0 {
        return false;
    }
    for tag in &args.only_tag {
        if !e.tech_tags.iter().any(|t| t == tag) {
            return false;
        }
    }
    true
}

fn sort(spec: &[String], entries: &mut [RepoEntry]) -> Result<()> {
    if spec.is_empty() {
        return Ok(());
    }
    let keys: Vec<(String, bool)> = spec
        .iter()
        .map(|raw| {
            let s = raw.trim();
            if let Some(rest) = s.strip_prefix('-') {
                (rest.to_string(), false) // desc
            } else {
                (s.to_string(), true) // asc
            }
        })
        .collect();
    for (field, _) in &keys {
        if cmp_for(field, &RepoEntry::placeholder(), &RepoEntry::placeholder()).is_none() {
            return Err(anyhow!("cannot sort by unknown field '{}'", field));
        }
    }
    entries.sort_by(|a, b| {
        for (field, asc) in &keys {
            let ord = cmp_for(field, a, b).unwrap_or(Ordering::Equal);
            if ord != Ordering::Equal {
                return if *asc { ord } else { ord.reverse() };
            }
        }
        Ordering::Equal
    });
    Ok(())
}

/// Returns None if the field is not sortable / unknown.
fn cmp_for(field: &str, a: &RepoEntry, b: &RepoEntry) -> Option<Ordering> {
    match field {
        "path" => Some(a.path.cmp(&b.path)),
        "name" => Some(a.name.cmp(&b.name)),
        "current_branch" => Some(opt_cmp(&a.current_branch, &b.current_branch)),
        "github_repo" => Some(opt_cmp(&a.github_repo, &b.github_repo)),
        "is_private" => Some(a.is_private.cmp(&b.is_private)),
        "last_commit_date" => Some(opt_cmp(
            &a.last_commit.as_ref().map(|c| c.date.clone()),
            &b.last_commit.as_ref().map(|c| c.date.clone()),
        )),
        "last_commit_hash" => Some(opt_cmp(
            &a.last_commit.as_ref().map(|c| c.hash.clone()),
            &b.last_commit.as_ref().map(|c| c.hash.clone()),
        )),
        "has_uncommitted" => Some(a.has_uncommitted.cmp(&b.has_uncommitted)),
        "uncommitted_files" => Some(opt_cmp(
            &a.uncommitted.as_ref().map(|u| u.files),
            &b.uncommitted.as_ref().map(|u| u.files),
        )),
        "uncommitted_insertions" => Some(opt_cmp(
            &a.uncommitted.as_ref().map(|u| u.insertions),
            &b.uncommitted.as_ref().map(|u| u.insertions),
        )),
        "uncommitted_deletions" => Some(opt_cmp(
            &a.uncommitted.as_ref().map(|u| u.deletions),
            &b.uncommitted.as_ref().map(|u| u.deletions),
        )),
        "unpushed_commits" => Some(opt_cmp(&a.unpushed_commits, &b.unpushed_commits)),
        "unmerged_branches" => Some(opt_cmp(&a.unmerged_branches, &b.unmerged_branches)),
        "open_issues" => Some(opt_cmp(&a.open_issues, &b.open_issues)),
        "open_prs" => Some(opt_cmp(&a.open_prs, &b.open_prs)),
        "loc" => Some(opt_cmp(&a.loc, &b.loc)),
        "scale" => Some(opt_cmp(&a.scale.map(scale_rank), &b.scale.map(scale_rank))),
        "dir_size_bytes" => Some(opt_cmp(&a.dir_size_bytes, &b.dir_size_bytes)),
        "has_readme" => Some(a.has_readme.cmp(&b.has_readme)),
        "has_dockerfile" => Some(a.has_dockerfile.cmp(&b.has_dockerfile)),
        "compose_running" => Some(a.compose_running.cmp(&b.compose_running)),
        "tech_tags" => Some(a.tech_tags.len().cmp(&b.tech_tags.len())),
        _ => None,
    }
}

fn opt_cmp<T: Ord>(a: &Option<T>, b: &Option<T>) -> Ordering {
    // None sorts first (treat as smaller).
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, _) => Ordering::Less,
        (_, None) => Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(y),
    }
}

fn scale_rank(s: Scale) -> u8 {
    match s {
        Scale::Small => 0,
        Scale::Medium => 1,
        Scale::Large => 2,
        Scale::Huge => 3,
    }
}

impl RepoEntry {
    fn placeholder() -> Self {
        RepoEntry::new(String::new(), String::new())
    }
}
