use std::process::Command;

pub struct Tool {
    pub name: &'static str,
    pub required: bool,
    pub purpose: &'static str,
    pub install_macos: &'static [&'static str],
    pub install_linux: &'static [&'static str],
    pub url: &'static str,
}

pub const TOOLS: &[Tool] = &[
    Tool {
        name: "git",
        required: true,
        purpose: "all repository scanning",
        install_macos: &["xcode-select --install", "brew install git"],
        install_linux: &["apt install git", "dnf install git", "pacman -S git"],
        url: "https://git-scm.com/",
    },
    Tool {
        name: "gh",
        required: false,
        purpose: "GitHub description, is_private, open issues, open PRs",
        install_macos: &["brew install gh"],
        install_linux: &["apt install gh", "dnf install gh", "pacman -S github-cli"],
        url: "https://cli.github.com/",
    },
    Tool {
        name: "docker",
        required: false,
        purpose: "compose runtime status (compose ps)",
        install_macos: &["brew install --cask docker"],
        install_linux: &["see https://docs.docker.com/engine/install/"],
        url: "https://docs.docker.com/get-docker/",
    },
];

pub fn is_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn print_diagnosis() {
    let os = std::env::consts::OS;
    println!("reposnap tool diagnosis  (platform: {})\n", os);

    let max_name = TOOLS.iter().map(|t| t.name.len()).max().unwrap_or(0);
    for tool in TOOLS {
        let status = if is_available(tool.name) {
            "OK     "
        } else {
            "missing"
        };
        let req = if tool.required { " (required)" } else { "" };
        println!(
            "  {:width$}  {}  {}{}",
            tool.name,
            status,
            tool.purpose,
            req,
            width = max_name
        );
    }

    let missing: Vec<&Tool> = TOOLS.iter().filter(|t| !is_available(t.name)).collect();
    if missing.is_empty() {
        println!("\nAll tools available.");
        return;
    }

    println!("\nInstall instructions for missing tools:");
    for tool in missing {
        println!("\n  {}", tool.name);
        let hints = match os {
            "macos" => tool.install_macos,
            "linux" => tool.install_linux,
            _ => &[][..],
        };
        for hint in hints {
            println!("    {}", hint);
        }
        println!("    {}", tool.url);
    }
}
