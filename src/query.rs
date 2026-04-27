use std::cmp::Ordering;

use anyhow::{anyhow, Result};

use crate::cli::Args;
use crate::types::{RepoEntry, Scale};

pub fn apply(args: &Args, mut entries: Vec<RepoEntry>) -> Result<Vec<RepoEntry>> {
    let filters: Vec<Filter> = args.filter.iter().map(|s| Filter::parse(s)).collect::<Result<_>>()?;
    for f in &filters {
        f.validate()?;
    }
    entries.retain(|e| keep(args, e) && filters.iter().all(|f| f.matches(e)));
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

// --- filter ---

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
}

#[derive(Debug, Clone)]
pub struct Filter {
    field: String,
    op: Op,
    value: String,
}

impl Filter {
    fn parse(spec: &str) -> Result<Self> {
        let s = spec.trim();
        // Order matters: longer operators first.
        for (token, op) in &[
            (">=", Op::Ge),
            ("<=", Op::Le),
            ("!=", Op::Ne),
            ("~", Op::Contains),
            ("=", Op::Eq),
            (">", Op::Gt),
            ("<", Op::Lt),
        ] {
            if let Some(idx) = s.find(token) {
                let field = s[..idx].trim();
                let value = s[idx + token.len()..].trim();
                if field.is_empty() {
                    return Err(anyhow!("filter missing field name: '{}'", spec));
                }
                return Ok(Filter {
                    field: field.to_string(),
                    op: *op,
                    value: value.to_string(),
                });
            }
        }
        Err(anyhow!(
            "filter missing operator (=, !=, >, <, >=, <=, ~): '{}'",
            spec
        ))
    }

    fn validate(&self) -> Result<()> {
        if entry_field(&self.field, &RepoEntry::placeholder()).is_some() {
            Ok(())
        } else {
            Err(anyhow!("cannot filter by unknown field '{}'", self.field))
        }
    }

    fn matches(&self, e: &RepoEntry) -> bool {
        match entry_field(&self.field, e) {
            Some(val) => evaluate(&val, self.op, &self.value),
            None => false,
        }
    }
}

enum FieldVal {
    Str(String),
    Bool(bool),
    Num(i64),
    Strings(Vec<String>),
    Numbers(Vec<i64>),
    Missing,
}

fn entry_field(field: &str, e: &RepoEntry) -> Option<FieldVal> {
    let v = match field {
        "path" => FieldVal::Str(e.path.clone()),
        "name" => FieldVal::Str(e.name.clone()),
        "current_branch" => opt_str(&e.current_branch),
        "github_repo" => opt_str(&e.github_repo),
        "github_description" => opt_str(&e.github_description),
        "is_private" => e.is_private.map(FieldVal::Bool).unwrap_or(FieldVal::Missing),
        "last_commit_hash" => e
            .last_commit
            .as_ref()
            .map(|c| FieldVal::Str(c.hash.clone()))
            .unwrap_or(FieldVal::Missing),
        "last_commit_date" => e
            .last_commit
            .as_ref()
            .map(|c| FieldVal::Str(c.date.clone()))
            .unwrap_or(FieldVal::Missing),
        "last_commit_message" => e
            .last_commit
            .as_ref()
            .map(|c| FieldVal::Str(c.message.clone()))
            .unwrap_or(FieldVal::Missing),
        "has_uncommitted" => FieldVal::Bool(e.has_uncommitted),
        "uncommitted_files" => e
            .uncommitted
            .as_ref()
            .map(|u| FieldVal::Num(u.files as i64))
            .unwrap_or(FieldVal::Missing),
        "uncommitted_insertions" => e
            .uncommitted
            .as_ref()
            .map(|u| FieldVal::Num(u.insertions as i64))
            .unwrap_or(FieldVal::Missing),
        "uncommitted_deletions" => e
            .uncommitted
            .as_ref()
            .map(|u| FieldVal::Num(u.deletions as i64))
            .unwrap_or(FieldVal::Missing),
        "unpushed_commits" => opt_num_u32(e.unpushed_commits),
        "unmerged_branches" => opt_num_u32(e.unmerged_branches),
        "open_issues" => opt_num_u32(e.open_issues),
        "open_prs" => opt_num_u32(e.open_prs),
        "loc" => e
            .loc
            .map(|n| FieldVal::Num(n as i64))
            .unwrap_or(FieldVal::Missing),
        "scale" => e
            .scale
            .map(|s| FieldVal::Str(scale_name(s).to_string()))
            .unwrap_or(FieldVal::Missing),
        "dir_size_bytes" => e
            .dir_size_bytes
            .map(|n| FieldVal::Num(n as i64))
            .unwrap_or(FieldVal::Missing),
        "has_readme" => FieldVal::Bool(e.has_readme),
        "has_dockerfile" => FieldVal::Bool(e.has_dockerfile),
        "compose_file" => opt_str(&e.compose_file),
        "compose_ports" => FieldVal::Numbers(e.compose_ports.iter().map(|&p| p as i64).collect()),
        "compose_running" => FieldVal::Bool(e.compose_running),
        "tech_tags" => FieldVal::Strings(e.tech_tags.clone()),
        _ => return None,
    };
    Some(v)
}

fn opt_str(v: &Option<String>) -> FieldVal {
    v.clone().map(FieldVal::Str).unwrap_or(FieldVal::Missing)
}

fn opt_num_u32(v: Option<u32>) -> FieldVal {
    v.map(|n| FieldVal::Num(n as i64))
        .unwrap_or(FieldVal::Missing)
}

fn evaluate(val: &FieldVal, op: Op, raw: &str) -> bool {
    match val {
        FieldVal::Missing => false,
        FieldVal::Str(s) => match op {
            Op::Eq => {
                // Allow case-insensitive match for "scale" enum values handled at field level.
                s == raw
            }
            Op::Ne => s != raw,
            Op::Contains => s.contains(raw),
            Op::Gt => s.as_str() > raw,
            Op::Lt => s.as_str() < raw,
            Op::Ge => s.as_str() >= raw,
            Op::Le => s.as_str() <= raw,
        },
        FieldVal::Bool(b) => match op {
            Op::Eq => parse_bool(raw).map(|v| *b == v).unwrap_or(false),
            Op::Ne => parse_bool(raw).map(|v| *b != v).unwrap_or(false),
            _ => false,
        },
        FieldVal::Num(n) => raw
            .parse::<i64>()
            .ok()
            .map(|v| match op {
                Op::Eq => *n == v,
                Op::Ne => *n != v,
                Op::Gt => *n > v,
                Op::Lt => *n < v,
                Op::Ge => *n >= v,
                Op::Le => *n <= v,
                Op::Contains => false,
            })
            .unwrap_or(false),
        FieldVal::Strings(arr) => match op {
            Op::Contains => arr.iter().any(|s| s == raw),
            Op::Eq => arr.len() == 1 && arr[0] == raw,
            Op::Ne => !arr.iter().any(|s| s == raw),
            _ => false,
        },
        FieldVal::Numbers(arr) => match op {
            Op::Contains => raw
                .parse::<i64>()
                .ok()
                .map(|v| arr.contains(&v))
                .unwrap_or(false),
            _ => false,
        },
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "t" | "yes" | "y" | "1" => Some(true),
        "false" | "f" | "no" | "n" | "0" => Some(false),
        _ => None,
    }
}

// --- sort ---

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

fn scale_name(s: Scale) -> &'static str {
    match s {
        Scale::Small => "small",
        Scale::Medium => "medium",
        Scale::Large => "large",
        Scale::Huge => "huge",
    }
}

impl RepoEntry {
    fn placeholder() -> Self {
        RepoEntry::new(String::new(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_filters() {
        let f = Filter::parse("scale=large").unwrap();
        assert_eq!(f.field, "scale");
        assert_eq!(f.op, Op::Eq);
        assert_eq!(f.value, "large");

        let f = Filter::parse("loc>=10000").unwrap();
        assert_eq!(f.op, Op::Ge);
        assert_eq!(f.value, "10000");

        let f = Filter::parse("tech_tags~rust").unwrap();
        assert_eq!(f.op, Op::Contains);

        assert!(Filter::parse("=value").is_err());
        assert!(Filter::parse("noop").is_err());
    }

    #[test]
    fn evaluates_string_eq_and_contains() {
        let v = FieldVal::Str("rust".into());
        assert!(evaluate(&v, Op::Eq, "rust"));
        assert!(!evaluate(&v, Op::Eq, "go"));
        assert!(evaluate(&v, Op::Contains, "ust"));
    }

    #[test]
    fn evaluates_num_compare() {
        let v = FieldVal::Num(42);
        assert!(evaluate(&v, Op::Gt, "10"));
        assert!(evaluate(&v, Op::Le, "42"));
        assert!(!evaluate(&v, Op::Lt, "0"));
    }

    #[test]
    fn evaluates_array_contains() {
        let v = FieldVal::Strings(vec!["rust".into(), "wasm".into()]);
        assert!(evaluate(&v, Op::Contains, "rust"));
        assert!(!evaluate(&v, Op::Contains, "go"));
    }

    #[test]
    fn missing_value_never_matches() {
        let v = FieldVal::Missing;
        assert!(!evaluate(&v, Op::Eq, "anything"));
        assert!(!evaluate(&v, Op::Ne, "anything"));
    }
}
