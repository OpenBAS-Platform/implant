use std::process::{Child, Command, Output, Stdio};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::Deserialize;

use crate::common::error_model::Error;

#[derive(Debug, Deserialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub status: String,
    pub exit_code: i32,
}

pub fn invoke_command(echo_cmd: Child, executor: &str) -> std::io::Result<Output> {
    Command::new(executor)
        .stdin(Stdio::from(echo_cmd.stdout.unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
}

pub fn invoke_powershell_command(command: &str, executor: &str, args: &[&str]) -> std::io::Result<Output> {
    // For powershell complex command, we need to encode in base64 to manage escape caracters and multi lines commands
    let invoke_expression = format!("$ErrorActionPreference = 'Stop'; Invoke-Expression ([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\"))); exit $LASTEXITCODE", BASE64_STANDARD.encode(command));
    Command::new(executor)
        .args(args)
        .arg(invoke_expression)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
}

pub fn invoke_shell_command(command: &str, executor: &str) -> std::io::Result<Output> {
    // For shell complex command, we need to encode in base64 to manage escape caracters and multi lines commands
    let base64_child = Command::new(executor)
        .arg("-c")
        .arg("echo ".to_owned() + &BASE64_STANDARD.encode(command) + " | base64 --d")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    invoke_command(base64_child, executor)
}

pub fn invoke_windows_command(command: &str) -> std::io::Result<Output> {
    // To manage multi lines (we need more than just base64 like the other executor), we replace break line (\n) by &
    // \n can be found in Windows path (ex: C:\\newFile) but \n replaces only break line and not \\n in path
    let new_command = format!(
        "setlocal & {} & exit /b errorlevel",
        command.trim().replace("\n", " & ") // trim "cleans" the start and the end of the command (see the trim doc)
    );
    let invoke_expression = format!("([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\")))", BASE64_STANDARD.encode(new_command));
    let base64_child = Command::new("powershell.exe")
        .arg(&invoke_expression)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    invoke_command(base64_child, "cmd")
}

pub fn manage_result(invoke_output: Output, pre_check: bool) -> Result<ExecutionResult, Error>  {
    let invoke_result = invoke_output.clone();
    // 0 success | other = maybe prevented
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout = String::from_utf8_lossy(&invoke_result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&invoke_result.stderr).to_string();

    let exit_status = match exit_code {
        0 if stderr.is_empty() => "SUCCESS",
        0 if !stderr.is_empty() => "WARNING",
        1 if pre_check => "SUCCESS",
        -99 => "ERROR",
        127 => "COMMAND_NOT_FOUND",
        126 => "COMMAND_CANNOT_BE_EXECUTED",
        _ => "MAYBE_PREVENTED",
    };

    Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        status: String::from(exit_status),
    })
}

#[cfg(target_os = "windows")]
pub fn command_execution(command: &str, executor: &str, pre_check: bool) -> Result<ExecutionResult, Error> {
    let invoke_output;
    if executor == "cmd" {
        invoke_output = invoke_windows_command(command);
    } else if executor == "bash" || executor == "sh" {
        invoke_output = invoke_shell_command(command, executor);
    } else {
        invoke_output = invoke_powershell_command(command,"powershell.exe", &[
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-NonInteractive",
            "-NoProfile",
            "-Command"]);
    }
    manage_result(invoke_output?, pre_check)
}

pub fn invoke_unix_command(command: &str, executor: &str) -> std::io::Result<Output> {
    // For unix shell complex command, we need to encode in base64 to manage escape caracters (pipes,...) and multi lines commands
    let echo_child = Command::new("echo")
        .arg(BASE64_STANDARD.encode(command))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let base64_child = Command::new("base64")
        .arg("-d")
        .stdin(Stdio::from(echo_child.stdout.unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    invoke_command(base64_child, executor)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn command_execution(command: &str, executor: &str, pre_check: bool) -> Result<ExecutionResult, Error> {
    let invoke_output;
    if executor == "bash" {
        invoke_output = invoke_unix_command(command, "bash");
    } else if executor == "psh" {
        invoke_output = invoke_powershell_command(command, "powershell", &[
            "-ExecutionPolicy",
            "Bypass",
            "-NonInteractive",
            "-NoProfile",
            "-Command"]);
    } else {
        invoke_output = invoke_unix_command(command, "sh");
    }
    manage_result(invoke_output?, pre_check)
}
