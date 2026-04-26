use std::path::Path;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::types::{LastCommit, Uncommitted};
use crate::util::run;

static GITHUB_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:git@github\.com:|github\.com[:/])(.+?)(?:\.git)?/?$").unwrap());

static SHORTSTAT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(\d+)\s+files?\s+changed(?:,\s*(\d+)\s+insertions?\(\+\))?(?:,\s*(\d+)\s+deletions?\(-\))?",
    )
    .unwrap()
});

pub fn parse_github_repo(remote: &str) -> Option<String> {
    GITHUB_RE
        .captures(remote.trim())
        .map(|c| c.get(1).unwrap().as_str().trim_end_matches('/').to_string())
}

pub async fn remote_origin(repo: &Path) -> Option<String> {
    let out = run(repo, "git", &["remote", "get-url", "origin"]).await?;
    if !out.ok() || out.stdout.is_empty() {
        return None;
    }
    Some(out.stdout)
}

pub async fn last_commit(repo: &Path) -> Option<LastCommit> {
    let out = run(repo, "git", &["log", "-1", "--format=%h%n%aI%n%s"]).await?;
    if !out.ok() || out.stdout.is_empty() {
        return None;
    }
    let mut lines = out.stdout.splitn(3, '\n');
    let hash = lines.next()?.to_string();
    let date = lines.next()?.to_string();
    let message = lines.next().unwrap_or("").to_string();
    Some(LastCommit { hash, date, message })
}

fn parse_shortstat(s: &str) -> (u32, u32, u32) {
    if let Some(c) = SHORTSTAT_RE.captures(s) {
        let f = c.get(1).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
        let i = c.get(2).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
        let d = c.get(3).map(|m| m.as_str().parse().unwrap_or(0)).unwrap_or(0);
        (f, i, d)
    } else {
        (0, 0, 0)
    }
}

pub async fn uncommitted(repo: &Path) -> (bool, Option<Uncommitted>) {
    let porcelain = run(repo, "git", &["status", "--porcelain"]).await;
    let dirty = matches!(&porcelain, Some(o) if o.ok() && !o.stdout.is_empty());

    let unstaged = run(repo, "git", &["diff", "--shortstat"]).await;
    let staged = run(repo, "git", &["diff", "--shortstat", "--staged"]).await;

    let (uf, ui, ud) = unstaged
        .as_ref()
        .map(|o| parse_shortstat(&o.stdout))
        .unwrap_or((0, 0, 0));
    let (sf, si, sd) = staged
        .as_ref()
        .map(|o| parse_shortstat(&o.stdout))
        .unwrap_or((0, 0, 0));

    let untracked = porcelain
        .as_ref()
        .map(|o| {
            o.stdout
                .lines()
                .filter(|l| l.starts_with("??"))
                .count() as u32
        })
        .unwrap_or(0);

    let files = uf + sf + untracked;
    let insertions = ui + si;
    let deletions = ud + sd;

    if !dirty && files == 0 && insertions == 0 && deletions == 0 {
        return (false, None);
    }

    (
        true,
        Some(Uncommitted {
            files,
            insertions,
            deletions,
        }),
    )
}

pub async fn unpushed_commits(repo: &Path) -> Option<u32> {
    let upstream = run(repo, "git", &["rev-parse", "--abbrev-ref", "@{u}"]).await?;
    if !upstream.ok() {
        return None;
    }
    let out = run(repo, "git", &["rev-list", "--count", "@{u}..HEAD"]).await?;
    if !out.ok() {
        return None;
    }
    out.stdout.trim().parse().ok()
}

pub async fn default_branch(repo: &Path) -> Option<String> {
    let out = run(
        repo,
        "git",
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .await?;
    if !out.ok() || out.stdout.is_empty() {
        return None;
    }
    // "origin/main" → "main"
    Some(out.stdout.trim_start_matches("origin/").to_string())
}

pub async fn unmerged_branches(repo: &Path) -> Option<u32> {
    let default = default_branch(repo).await.unwrap_or_else(|| "main".into());
    let target = format!("origin/{}", default);
    let out = run(repo, "git", &["branch", "--no-merged", &target]).await?;
    if !out.ok() {
        return None;
    }
    let count = out
        .stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count() as u32;
    Some(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_github_remote_urls() {
        assert_eq!(
            parse_github_repo("git@github.com:misebox/reposcan.git"),
            Some("misebox/reposcan".into())
        );
        assert_eq!(
            parse_github_repo("https://github.com/misebox/reposcan.git"),
            Some("misebox/reposcan".into())
        );
        assert_eq!(
            parse_github_repo("https://github.com/misebox/reposcan"),
            Some("misebox/reposcan".into())
        );
        assert_eq!(parse_github_repo("git@gitlab.com:foo/bar.git"), None);
    }

    #[test]
    fn parses_shortstat_lines() {
        assert_eq!(
            parse_shortstat(" 3 files changed, 10 insertions(+), 5 deletions(-)"),
            (3, 10, 5)
        );
        assert_eq!(parse_shortstat(" 1 file changed, 2 insertions(+)"), (1, 2, 0));
        assert_eq!(parse_shortstat(" 1 file changed, 4 deletions(-)"), (1, 0, 4));
        assert_eq!(parse_shortstat(""), (0, 0, 0));
    }
}
