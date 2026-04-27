use anyhow::{anyhow, Result};

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

const GROUPS: &[(&str, &[&str])] = &[
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
        return Ok(ALL_FIELDS.to_vec());
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
        let canonical = ALL_FIELDS.iter().find(|f| **f == token).ok_or_else(|| {
            anyhow!(
                "unknown field '{}'. Run with --fields all to see canonical names.",
                token
            )
        })?;
        if seen.insert(*canonical) {
            out.push(*canonical);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_all() {
        let r = resolve(&[]).unwrap();
        assert_eq!(r, ALL_FIELDS);
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
}
