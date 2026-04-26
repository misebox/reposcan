use std::path::Path;

use serde::Deserialize;

use crate::util::run;

#[derive(Debug, Default, Clone)]
pub struct GitHubMeta {
    pub description: Option<String>,
    pub is_private: Option<bool>,
    pub open_issues: Option<u32>,
    pub open_prs: Option<u32>,
}

#[derive(Deserialize)]
struct RepoView {
    description: Option<String>,
    #[serde(rename = "isPrivate")]
    is_private: Option<bool>,
}

#[derive(Deserialize)]
struct NumberItem {
    #[allow(dead_code)]
    number: u64,
}

pub async fn fetch(repo: &Path) -> GitHubMeta {
    let mut meta = GitHubMeta::default();

    if let Some(out) = run(repo, "gh", &["repo", "view", "--json", "description,isPrivate"]).await {
        if out.ok() {
            match serde_json::from_str::<RepoView>(&out.stdout) {
                Ok(v) => {
                    meta.description = v.description.filter(|s| !s.is_empty());
                    meta.is_private = v.is_private;
                }
                Err(e) => tracing::debug!(?repo, error = %e, "gh repo view: parse failed"),
            }
        } else {
            tracing::debug!(?repo, "gh repo view: non-zero exit (renamed/deleted?)");
        }
    }

    let (issues, prs) = tokio::join!(
        fetch_count(repo, &["issue", "list", "--state", "open", "--json", "number"]),
        fetch_count(repo, &["pr", "list", "--state", "open", "--json", "number"]),
    );
    meta.open_issues = issues;
    meta.open_prs = prs;

    meta
}

async fn fetch_count(repo: &Path, args: &[&str]) -> Option<u32> {
    let mut full = vec!["gh"];
    full.extend_from_slice(args);
    let out = run(repo, full[0], &full[1..]).await?;
    if !out.ok() {
        return None;
    }
    let items: Vec<NumberItem> = serde_json::from_str(&out.stdout).ok()?;
    Some(items.len() as u32)
}
