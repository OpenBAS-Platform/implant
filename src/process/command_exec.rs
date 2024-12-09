use std::process::{Command, Output, Stdio};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;

use crate::common::error_model::Error;

#[derive(Debug, Deserialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub status: String,
    pub exit_code: i32,
}

pub fn invoke_command(executor: &str, cmd_expression:  &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(executor)
        .args(args)
        .arg(cmd_expression)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
}

pub fn decode_command(encoded_command: &str) -> String {
    let decoded_bytes = STANDARD
        .decode(encoded_command)
        .expect("Failed to decode Base64 command");
    String::from_utf8(decoded_bytes)
        .expect("Decoded command is not valid UTF-8")
}

pub fn format_powershell_command(command:String) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'; {} ; exit $LASTEXITCODE",
        command
    )
}

pub fn format_windows_command(command:String) -> String {
    format!(
        "setlocal & {} & exit /b errorlevel",
        command
    )
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
pub fn get_executor( executor: &str) -> &str {
    match executor {
        "cmd" | "bash" | "sh" => executor,
        _ => "powershell"
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_executor( executor: &str) -> &str {
    match executor {
        "bash" => executor,
        "psh" => "powershell",
        _ => "sh"
    }
}

#[cfg(target_os = "windows")]
pub fn get_psh_arg() -> Vec<&'static str> {
    Vec::from([
        "-ExecutionPolicy",
        "Bypass",
        "-WindowStyle",
        "Hidden",
        "-NonInteractive",
        "-NoProfile",
        "-Command"])
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_psh_arg() -> Vec<&'static str> {
    Vec::from([
        "-ExecutionPolicy",
        "Bypass",
        "-NonInteractive",
        "-NoProfile",
        "-Command"])
}

pub fn command_execution(command: &str, executor: &str, pre_check: bool) -> Result<ExecutionResult, Error> {
    let final_executor = get_executor(executor);
    let mut formatted_cmd= decode_command(command);
    let mut args: Vec<&str> = vec!["-c"];

    if final_executor == "cmd" {
        formatted_cmd = format_windows_command(formatted_cmd);
        args = vec!["/V", "/C"];

    }  else if final_executor == "powershell" {
        formatted_cmd = format_powershell_command(formatted_cmd);
        args = get_psh_arg();
    }

    let invoke_output = invoke_command(final_executor, &formatted_cmd, args.as_slice());
    manage_result(invoke_output?, pre_check)
}
