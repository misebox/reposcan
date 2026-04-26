use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

pub struct CmdOutput {
    pub stdout: String,
    pub status: Option<i32>,
}

impl CmdOutput {
    pub fn ok(&self) -> bool {
        matches!(self.status, Some(0))
    }
}

pub async fn run(cwd: &Path, program: &str, args: &[&str]) -> Option<CmdOutput> {
    run_with_timeout(cwd, program, args, DEFAULT_TIMEOUT).await
}

pub async fn run_with_timeout(
    cwd: &Path,
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Option<CmdOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let fut = cmd.output();
    let output = match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            tracing::debug!(?cwd, program, ?args, error = %e, "command failed to spawn");
            return None;
        }
        Err(_) => {
            tracing::warn!(?cwd, program, ?args, "command timed out");
            return None;
        }
    };

    Some(CmdOutput {
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        status: output.status.code(),
    })
}
