use std::path::Path;

use tokei::{Config, Languages};

use crate::util::run;

pub async fn count(repo: &Path) -> Option<u64> {
    let repo = repo.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut langs = Languages::new();
        langs.get_statistics(&[&repo], &[], &Config::default());
        let total = langs.total();
        Some(total.code as u64)
    })
    .await
    .ok()
    .flatten()
}

pub async fn dir_size_bytes(repo: &Path) -> Option<u64> {
    let out = run(repo, "du", &["-sk", "."]).await?;
    if !out.ok() {
        return None;
    }
    let kb: u64 = out.stdout.split_whitespace().next()?.parse().ok()?;
    Some(kb * 1024)
}
