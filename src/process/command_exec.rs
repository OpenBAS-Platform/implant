use std::process::{Child, Command, Output, Stdio};

use serde::Deserialize;

use crate::common::error_model::Error;
use crate::process::exec_utils::is_executor_present;

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
    let invoke_expression = format!("$ErrorActionPreference = 'Stop'; Invoke-Expression ([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\"))); exit $LASTEXITCODE", command);
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
    let base64_command = format!("echo {} | base64 -d", command);
    let base64_child = Command::new(executor)
        .arg("-c")
        .arg(&base64_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    invoke_command(base64_child, executor)
}

pub fn invoke_windows_command(command: &str) -> std::io::Result<Output> {
    let invoke_expression = format!("([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\")))", command);
    let base64_child = Command::new("powershell.exe")
        .arg("-Command")
        .arg(&invoke_expression)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let decoded_command = String::from_utf8_lossy(&base64_child.stdout).trim().to_string();

    let cmd_expression = format!(
        "setlocal & {} & exit /b errorlevel",
        decoded_command
    );

    Command::new("cmd.exe")
        .arg("/C")
        .arg(cmd_expression)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
}

pub fn manage_result(invoke_output: Output, pre_check: bool) -> Result<ExecutionResult, Error>  {
    let invoke_result = invoke_output.clone();
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
        if !is_executor_present(executor){
            return Err(Error::Internal(format!("Executor {} is not available.", executor)));
        }
        invoke_output = invoke_windows_command(command);
    } else if executor == "bash" || executor == "sh" {
        if !is_executor_present(executor){
            return Err(Error::Internal(format!("Executor {} is not available.", executor)));
        }
        invoke_output = invoke_shell_command(command, executor);
    } else {
        if !is_executor_present("powershell.exe"){
            return Err(Error::Internal(format!("Executor powershell.exe is not available.")));
        }
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn command_execution(command: &str, executor: &str, pre_check: bool) -> Result<ExecutionResult, Error> {

    let invoke_output;
    if executor == "bash" {
        if !is_executor_present(executor){
            return Err(Error::Internal(format!("Executor '{}' is not available.", executor)));
        }
        invoke_output = invoke_shell_command(command, executor);
    } else if executor == "psh" {
        if !is_executor_present(executor){
            return Err(Error::Internal(format!("Executor '{}' is not available.", executor)));
        }
        invoke_output = invoke_powershell_command(command, "powershell", &[
            "-ExecutionPolicy",
            "Bypass",
            "-NonInteractive",
            "-NoProfile",
            "-Command"]);
    } else {
        if !is_executor_present("sh"){
            return Err(Error::Internal(format!("Executor sh is not available.")));
        }
        invoke_output = invoke_shell_command(command, "sh");
    }
    manage_result(invoke_output?, pre_check)
}
