use crate::renderer::PreparedCommand;
#[cfg(unix)]
use anyhow::Context;
use anyhow::Result;
use std::process::ExitStatus;
#[cfg(unix)]
use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

#[cfg(unix)]
pub fn execute_command(command: &PreparedCommand) -> Result<ExitStatus> {
    execute_with_shell(command, &shell_program())
}

#[cfg(windows)]
pub fn execute_command(_command: &PreparedCommand) -> Result<ExitStatus> {
    anyhow::bail!("safe parameter execution is not supported with cmd.exe on Windows")
}

#[cfg(unix)]
fn execute_with_shell(command: &PreparedCommand, shell: &str) -> Result<ExitStatus> {
    println!("{}", command.display_text);
    io::stdout()
        .flush()
        .context("failed to flush command preview")?;

    let mut child = shell_command(shell, &command.execution_text);
    child
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to start command shell '{shell}'"))
}

#[cfg(unix)]
fn shell_program() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

#[cfg(unix)]
fn shell_command(shell: &str, command: &str) -> Command {
    let mut child = Command::new(shell);
    child.arg("-c").arg(command);
    child
}

#[cfg(unix)]
pub fn exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .unwrap_or_else(|| status.signal().map_or(1, |signal| 128 + signal))
}

#[cfg(not(unix))]
pub fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn returns_child_exit_status() {
        let status = execute_command(&PreparedCommand {
            execution_text: "exit 7".into(),
            display_text: "exit 7".into(),
        })
        .unwrap();

        assert_eq!(status.code(), Some(7));
    }

    #[cfg(unix)]
    #[test]
    fn startup_errors_do_not_include_execution_text_or_secret() {
        let command = PreparedCommand {
            execution_text: "login 'very-secret'".into(),
            display_text: "login '******'".into(),
        };
        let error = execute_with_shell(&command, "/definitely/missing/cmdp-shell")
            .unwrap_err()
            .to_string();
        assert!(!error.contains("very-secret"));
        assert!(!error.contains(&command.execution_text));
        assert!(error.contains("cmdp-shell"));
    }
}
