use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::cli::Args;
use crate::types::{scale_from_loc, RepoEntry};
use crate::{docker, git, github, loc, tags};

pub async fn collect_all(root: &Path, repos: Vec<PathBuf>, args: Arc<Args>) -> Vec<RepoEntry> {
    let sem = Arc::new(Semaphore::new(args.concurrency.max(1)));
    let mut set: JoinSet<RepoEntry> = JoinSet::new();

    for repo in repos {
        let permit_sem = sem.clone();
        let args = args.clone();
        let root = root.to_path_buf();
        set.spawn(async move {
            let _permit = permit_sem.acquire_owned().await.unwrap();
            collect_one(&root, &repo, &args).await
        });
    }

    let mut out = Vec::new();
    while let Some(r) = set.join_next().await {
        match r {
            Ok(entry) => out.push(entry),
            Err(e) => tracing::warn!(error = %e, "task join failed"),
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

async fn collect_one(root: &Path, repo: &Path, args: &Args) -> RepoEntry {
    let rel = repo
        .strip_prefix(root)
        .unwrap_or(repo)
        .to_string_lossy()
        .to_string();
    let name = repo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| rel.clone());

    let mut entry = RepoEntry::new(rel, name);
    entry.has_readme = has_readme(repo);
    entry.tech_tags = tags::detect(repo);

    tracing::info!(path = %entry.path, "scanning");

    // Run independent collectors in parallel.
    let branch_fut = git::current_branch(repo);
    let remote_fut = git::remote_origin(repo);
    let last_commit_fut = git::last_commit(repo);
    let uncommitted_fut = git::uncommitted(repo);
    let unpushed_fut = git::unpushed_commits(repo);
    let unmerged_fut = git::unmerged_branches(repo);
    let docker_fut = docker::collect(repo, args.no_docker);
    let dir_size_fut = loc::dir_size_bytes(repo);

    let (branch, remote, last, (dirty, unc), unpushed, unmerged, dock, dsize) = tokio::join!(
        branch_fut,
        remote_fut,
        last_commit_fut,
        uncommitted_fut,
        unpushed_fut,
        unmerged_fut,
        docker_fut,
        dir_size_fut,
    );

    entry.current_branch = branch;
    if let Some(url) = &remote {
        entry.github_repo = git::parse_github_repo(url);
    }
    entry.last_commit = last;
    entry.has_uncommitted = dirty;
    entry.uncommitted = unc;
    entry.unpushed_commits = unpushed;
    entry.unmerged_branches = unmerged;
    entry.has_dockerfile = dock.has_dockerfile;
    entry.compose_file = dock.compose_file;
    entry.compose_ports = dock.compose_ports;
    entry.compose_running = dock.compose_running;
    entry.dir_size_bytes = dsize;

    if !args.no_loc {
        if let Some(l) = loc::count(repo).await {
            entry.loc = Some(l);
            entry.scale = Some(scale_from_loc(l));
        }
    }

    if !args.no_github && entry.github_repo.is_some() {
        let gh = github::fetch(repo).await;
        entry.github_description = gh.description;
        entry.is_private = gh.is_private;
        entry.open_issues = gh.open_issues;
        entry.open_prs = gh.open_prs;
    }

    entry
}

fn has_readme(repo: &Path) -> bool {
    let candidates = ["README.md", "readme.md", "README.MD", "Readme.md", "README"];
    candidates.iter().any(|f| repo.join(f).exists())
}
