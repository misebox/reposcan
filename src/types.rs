use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastCommit {
    pub hash: String,
    pub date: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uncommitted {
    pub files: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scale {
    Small,
    Medium,
    Large,
    Huge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    pub path: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_repo: Option<String>,
    pub github_description: Option<String>,
    pub is_private: Option<bool>,
    pub last_commit: Option<LastCommit>,
    pub has_uncommitted: bool,
    pub uncommitted: Option<Uncommitted>,
    pub unpushed_commits: Option<u32>,
    pub unmerged_branches: Option<u32>,
    pub dir_size_bytes: Option<u64>,
    pub tech_tags: Vec<String>,
    pub has_readme: bool,
    pub scale: Option<Scale>,
    pub loc: Option<u64>,
    pub has_dockerfile: bool,
    pub compose_file: Option<String>,
    pub compose_ports: Vec<u16>,
    pub compose_running: bool,
    pub open_issues: Option<u32>,
    pub open_prs: Option<u32>,
}

impl RepoEntry {
    pub fn new(path: String, name: String) -> Self {
        Self {
            path,
            name,
            github_repo: None,
            github_description: None,
            is_private: None,
            last_commit: None,
            has_uncommitted: false,
            uncommitted: None,
            unpushed_commits: None,
            unmerged_branches: None,
            dir_size_bytes: None,
            tech_tags: Vec::new(),
            has_readme: false,
            scale: None,
            loc: None,
            has_dockerfile: false,
            compose_file: None,
            compose_ports: Vec::new(),
            compose_running: false,
            open_issues: None,
            open_prs: None,
        }
    }
}

pub fn scale_from_loc(loc: u64) -> Scale {
    match loc {
        0..=999 => Scale::Small,
        1000..=9_999 => Scale::Medium,
        10_000..=99_999 => Scale::Large,
        _ => Scale::Huge,
    }
}
