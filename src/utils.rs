use std::process::{Command as StdCommand, ExitStatus};

use tokio::process::Command as TokioCommand;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub status: u8,
}

impl Output {
    pub fn is_success(&self) -> bool {
        self.status == 0
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

impl From<std::process::Output> for Output {
    fn from(output: std::process::Output) -> Self {
        let status = exit_status_to_u8(output.status);

        Self {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            status,
        }
    }
}

fn exit_status_to_u8(status: ExitStatus) -> u8 {
    match status.code() {
        Some(code) => match code {
            0 => 0,
            c if c >= 0 && c <= u8::MAX as i32 => c as u8,
            _ => u8::MAX,
        },
        None => {
            if status.success() {
                0
            } else {
                1
            }
        }
    }
}

pub(crate) fn run_command(mut command: StdCommand) -> Result<Output> {
    let output = command.output()?;
    Ok(Output::from(output))
}

pub(crate) async fn run_command_async(
    mut command: TokioCommand,
) -> Result<Output> {
    let output = command.output().await?;
    Ok(Output::from(output))
}

macro_rules! run {
    ($program:expr $(, $arg:expr )* $(,)?) => {{
        let mut command = std::process::Command::new($program);
        $(command.arg($arg);)*
        $crate::utils::run_command(command)
    }};
    ($program:expr; $args:expr) => {{
        let mut command = std::process::Command::new($program);
        command.args($args);
        $crate::utils::run_command(command)
    }};
}

macro_rules! run_async {
    ($program:expr $(, $arg:expr )* $(,)?) => {{
        let mut command = tokio::process::Command::new($program);
        $(command.arg($arg);)*
        $crate::utils::run_command_async(command).await
    }};
    ($program:expr; $args:expr) => {{
        let mut command = tokio::process::Command::new($program);
        command.args($args);
        $crate::utils::run_command_async(command).await
    }};
}

pub(crate) use run;
pub(crate) use run_async;
