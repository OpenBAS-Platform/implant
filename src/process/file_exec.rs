use std::env;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use crate::common::error_model::Error;
use crate::process::command_exec::ExecutionResult;

fn compute_working_file(filename: &str) -> PathBuf {
    let current_exe_patch = env::current_exe().unwrap();
    let executable_path = current_exe_patch.parent().unwrap();
    return executable_path.join(filename);
}

pub fn manage_result(invoke_output: Output) -> Result<ExecutionResult, Error>  {
    let invoke_result = invoke_output.clone();
    // 0 success | other = maybe prevented
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout = String::from_utf8_lossy(&invoke_result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&invoke_result.stderr).to_string();

    let exit_status = match exit_code {
        0 if stderr.is_empty() => "SUCCESS",
        0 if !stderr.is_empty() => "WARNING",
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
pub fn file_execution(filename: &str) -> Result<ExecutionResult, Error> {

    let script_file_name = compute_working_file(filename);
    let win_path = format!("$ErrorActionPreference = 'Stop'; & '{}'; exit $LASTEXITCODE", script_file_name.to_str().unwrap());
    let command_args = &[
        "-ExecutionPolicy",
        "Bypass",
        "-WindowStyle",
        "Hidden",
        "-NonInteractive",
        "-NoProfile",
        "-Command",
    ];
    let invoke_output = Command::new("powershell.exe")
        .args(command_args)
        .arg(win_path)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output();
    manage_result(invoke_output?)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn file_execution(filename: &str) -> Result<ExecutionResult, Error> {
    let script_file_name = compute_working_file(filename);
    // Prepare and execute the command
    let command_args = &[script_file_name.to_str().unwrap()];
    let invoke_output = Command::new("bash")
        .args(command_args)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output();
    manage_result(invoke_output?)
}
