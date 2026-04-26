use std::collections::HashSet;
use std::path::{Path, PathBuf};

const HARDCODED_PRUNE: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "target",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
    ".turbo",
    ".vercel",
    "coverage",
    "out",
];

pub struct DiscoverOptions {
    pub include_nested: bool,
    pub extra_excludes: HashSet<String>,
}

pub fn discover(root: &Path, opts: &DiscoverOptions) -> std::io::Result<Vec<PathBuf>> {
    let mut prune: HashSet<String> = HARDCODED_PRUNE.iter().map(|s| s.to_string()).collect();
    prune.extend(opts.extra_excludes.iter().cloned());

    let mut repos = Vec::new();
    walk(root, &prune, opts.include_nested, &mut repos)?;
    repos.sort();
    Ok(repos)
}

fn walk(
    dir: &Path,
    prune: &HashSet<String>,
    include_nested: bool,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if is_git_repo(dir) {
        out.push(dir.to_path_buf());
        if !include_nested {
            return Ok(());
        }
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if prune.contains(&name) {
            continue;
        }
        let _ = walk(&entry.path(), prune, include_nested, out);
    }

    Ok(())
}

fn is_git_repo(dir: &Path) -> bool {
    // .git can be a directory (normal repo) or a file (worktree / submodule).
    dir.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "").unwrap();
    }

    #[test]
    fn detects_repos_and_prunes() {
        let tmp = std::env::temp_dir().join(format!("reposnap-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);

        // repo A: has .git directory
        fs::create_dir_all(tmp.join("a/.git")).unwrap();
        // repo B: nested under group
        fs::create_dir_all(tmp.join("group/b/.git")).unwrap();
        // repo C: .git is a file (worktree)
        fs::create_dir_all(tmp.join("c")).unwrap();
        touch(&tmp.join("c/.git"));
        // pruned dir should be skipped
        fs::create_dir_all(tmp.join("node_modules/x/.git")).unwrap();
        // nested repo inside repo A — skipped without include_nested
        fs::create_dir_all(tmp.join("a/inner/.git")).unwrap();

        let opts = DiscoverOptions {
            include_nested: false,
            extra_excludes: HashSet::new(),
        };
        let repos = discover(&tmp, &opts).unwrap();
        let rels: Vec<_> = repos
            .iter()
            .map(|p| p.strip_prefix(&tmp).unwrap().to_string_lossy().to_string())
            .collect();
        assert!(rels.contains(&"a".to_string()));
        assert!(rels.contains(&"group/b".to_string()));
        assert!(rels.contains(&"c".to_string()));
        assert!(!rels.iter().any(|p| p.starts_with("node_modules")));
        assert!(!rels.iter().any(|p| p == "a/inner"));

        let opts2 = DiscoverOptions {
            include_nested: true,
            extra_excludes: HashSet::new(),
        };
        let repos2 = discover(&tmp, &opts2).unwrap();
        let rels2: Vec<_> = repos2
            .iter()
            .map(|p| p.strip_prefix(&tmp).unwrap().to_string_lossy().to_string())
            .collect();
        assert!(rels2.contains(&"a/inner".to_string()));

        fs::remove_dir_all(&tmp).unwrap();
    }
}
