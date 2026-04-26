use std::path::Path;

use serde::Deserialize;

use crate::util::run;

#[derive(Deserialize)]
struct TokeiTotal {
    code: u64,
}

#[derive(Deserialize)]
struct TokeiOutput {
    #[serde(rename = "Total")]
    total: TokeiTotal,
}

pub async fn count(repo: &Path) -> Option<u64> {
    if let Some(loc) = via_tokei(repo).await {
        return Some(loc);
    }
    via_wc(repo).await
}

async fn via_tokei(repo: &Path) -> Option<u64> {
    let out = run(repo, "tokei", &["--output", "json"]).await?;
    if !out.ok() {
        return None;
    }
    let parsed: TokeiOutput = serde_json::from_str(&out.stdout).ok()?;
    Some(parsed.total.code)
}

async fn via_wc(repo: &Path) -> Option<u64> {
    let ls = run(repo, "git", &["ls-files"]).await?;
    if !ls.ok() {
        return None;
    }
    let mut total: u64 = 0;
    for file in ls.stdout.lines() {
        let path = repo.join(file);
        if let Ok(text) = std::fs::read_to_string(&path) {
            total += text.lines().count() as u64;
        }
    }
    Some(total)
}

pub async fn dir_size_bytes(repo: &Path) -> Option<u64> {
    let out = run(repo, "du", &["-sk", "."]).await?;
    if !out.ok() {
        return None;
    }
    let kb: u64 = out.stdout.split_whitespace().next()?.parse().ok()?;
    Some(kb * 1024)
}
