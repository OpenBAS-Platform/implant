use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::common::error_model::Error;
use crate::process::command_exec::ExecutionResult;

fn compute_working_file(filename: &str) -> PathBuf {
    let current_exe_patch = env::current_exe().unwrap();
    let executable_path = current_exe_patch.parent().unwrap();
    return executable_path.join(filename);
}

#[cfg(target_os = "windows")]
pub fn file_execution(filename: &str) -> Result<ExecutionResult, Error> {
    use std::os::windows::process::CommandExt;

    let script_file_name = compute_working_file(filename);
    let win_path = format!("\"{}\"", script_file_name.to_str().unwrap());
    let command_args = &["/d", "/c", "powershell.exe", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden",
        "-NonInteractive", "-NoProfile", "-File"];
    let invoke_output = Command::new("cmd.exe")
        .args(command_args)
        .raw_arg(win_path.as_str())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?.wait_with_output();
    // 0 success | other = maybe prevented
    let invoke_result = invoke_output.unwrap().clone();
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout =  String::from_utf8 (invoke_result.stdout).unwrap();
    let stderr = String::from_utf8 (invoke_result.stderr).unwrap();
    let exit_status = match exit_code {
        0 => "SUCCESS",
        1 => "MAYBE_PREVENTED",
        -99 => "ERROR",
        _ => "MAYBE_PREVENTED"
    };
    return Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        status: String::from(exit_status)
    })
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
        .spawn()?.wait_with_output();
    // 0 success | other = maybe prevented
    let invoke_result = invoke_output.unwrap().clone();
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout =  String::from_utf8 (invoke_result.stdout).unwrap();
    let stderr = String::from_utf8 (invoke_result.stderr).unwrap();
    let exit_status = match exit_code {
        0 => "SUCCESS",
        1 => "MAYBE_PREVENTED",
        -99 => "ERROR",
        _ => "MAYBE_PREVENTED"
    };
    return Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        status: String::from(exit_status)
    })
}