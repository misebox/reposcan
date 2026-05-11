use anyhow::{anyhow, Result};

/// Short aliases (mostly mirroring the ASCII column headers) that map to the
/// canonical field names. Used by --filter, --sort, --fields uniformly.
const ALIASES: &[(&str, &str)] = &[
    ("branch", "current_branch"),
    ("dirty", "has_uncommitted"),
    ("ahead", "unpushed_commits"),
    ("unmerged", "unmerged_branches"),
    ("gh", "github_repo"),
    ("priv", "is_private"),
    ("desc", "github_description"),
    ("tags", "tech_tags"),
    ("readme", "has_readme"),
    ("size", "dir_size_bytes"),
    ("dockerfile", "has_dockerfile"),
    ("compose", "compose_file"),
    ("ports", "compose_ports"),
    ("running", "compose_running"),
];

/// Resolve an alias to its canonical name, or return the input unchanged.
pub fn canonical(name: &str) -> &str {
    for (alias, canon) in ALIASES {
        if *alias == name {
            return canon;
        }
    }
    name
}

/// Canonical field names in their default order. Same set as the flat headers
/// used for csv / tsv / markdown output.
pub const ALL_FIELDS: &[&str] = &[
    "path",
    "name",
    "current_branch",
    "github_repo",
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
    "open_issues",
    "open_prs",
    "loc",
    "scale",
    "dir_size_bytes",
    "has_readme",
    "has_dockerfile",
    "compose_file",
    "compose_ports",
    "compose_running",
    "github_description",
    "tech_tags",
];

pub const DEFAULT_FIELDS: &[&str] = &[
    "name",
    "current_branch",
    "last_commit_date",
    "has_uncommitted",
    "unpushed_commits",
    "scale",
    "tech_tags",
];

const GROUPS: &[(&str, &[&str])] = &[
    ("default", DEFAULT_FIELDS),
    ("minimal", &["path", "name", "current_branch", "tech_tags"]),
    (
        "activity",
        &[
            "path",
            "last_commit_date",
            "has_uncommitted",
            "unpushed_commits",
            "unmerged_branches",
        ],
    ),
    (
        "github",
        &[
            "path",
            "github_repo",
            "is_private",
            "open_issues",
            "open_prs",
        ],
    ),
];

/// Expand the user spec into the canonical field list (order preserved, dedup).
/// Empty input → all fields. `all` → all fields. `@<group>` → group expansion.
pub fn resolve(spec: &[String]) -> Result<Vec<&'static str>> {
    if spec.is_empty() {
        return Ok(DEFAULT_FIELDS.to_vec());
    }
    let mut out: Vec<&'static str> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for raw in spec {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        if token == "all" {
            for f in ALL_FIELDS {
                if seen.insert(*f) {
                    out.push(*f);
                }
            }
            continue;
        }
        if let Some(group) = token.strip_prefix('@') {
            let fields = GROUPS
                .iter()
                .find(|(name, _)| *name == group)
                .map(|(_, fs)| *fs)
                .ok_or_else(|| {
                    anyhow!(
                        "unknown field group '@{}'. Available: {}",
                        group,
                        GROUPS
                            .iter()
                            .map(|(n, _)| format!("@{}", n))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            for f in fields {
                if seen.insert(*f) {
                    out.push(*f);
                }
            }
            continue;
        }
        let resolved = canonical(token);
        let matched = ALL_FIELDS.iter().find(|f| **f == resolved).ok_or_else(|| {
            anyhow!(
                "unknown field '{}'. Run with --fields all to see canonical names.",
                token
            )
        })?;
        if seen.insert(*matched) {
            out.push(*matched);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_default() {
        let r = resolve(&[]).unwrap();
        assert_eq!(r, DEFAULT_FIELDS);
    }

    #[test]
    fn explicit_fields_preserve_order_and_dedup() {
        let r = resolve(&["name".into(), "path".into(), "name".into()]).unwrap();
        assert_eq!(r, vec!["name", "path"]);
    }

    #[test]
    fn group_expansion() {
        let r = resolve(&["@minimal".into()]).unwrap();
        assert_eq!(r, vec!["path", "name", "current_branch", "tech_tags"]);
    }

    #[test]
    fn group_and_explicit_dedup() {
        let r = resolve(&["path".into(), "@minimal".into()]).unwrap();
        assert_eq!(r, vec!["path", "name", "current_branch", "tech_tags"]);
    }

    #[test]
    fn unknown_field_errors() {
        assert!(resolve(&["bogus".into()]).is_err());
    }

    #[test]
    fn unknown_group_errors() {
        assert!(resolve(&["@bogus".into()]).is_err());
    }

    #[test]
    fn alias_normalizes_to_canonical() {
        let r = resolve(&["branch".into(), "ahead".into(), "tags".into()]).unwrap();
        assert_eq!(r, vec!["current_branch", "unpushed_commits", "tech_tags"]);
    }

    #[test]
    fn alias_dedups_against_canonical() {
        let r = resolve(&["current_branch".into(), "branch".into()]).unwrap();
        assert_eq!(r, vec!["current_branch"]);
    }
}
