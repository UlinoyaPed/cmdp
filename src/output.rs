use anyhow::{Context, Result};
use std::{
    env,
    io::{self, Write},
    process::{Command, ExitStatus, Stdio},
};

pub fn execute_command(command: &str) -> Result<ExitStatus> {
    println!("{command}");
    io::stdout()
        .flush()
        .context("failed to flush command preview")?;

    let mut child = shell_command(command);
    child
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to execute generated command: {command}"))
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut child = Command::new(shell);
    child.arg("-c").arg(command);
    child
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut child = Command::new("cmd");
    child.arg("/C").arg(command);
    child
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_child_exit_status() {
        let status = execute_command("exit 7").unwrap();

        assert_eq!(status.code(), Some(7));
    }
}
