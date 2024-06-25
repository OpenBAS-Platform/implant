use base64::prelude::BASE64_STANDARD;
use std::process::{Command, Stdio};
use base64::Engine;
use serde::Deserialize;

use crate::common::error_model::Error;

#[derive(Debug, Deserialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub status: String,
    pub exit_code: i32,
}

#[cfg(target_os = "windows")]
pub fn command_execution(command: &str, pre_check: bool) -> Result<ExecutionResult, Error> {
    let invoke_expression = format!("Invoke-Expression ([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\")))", BASE64_STANDARD.encode(command));
    let invoke_output = Command::new("cmd.exe")
        .args(&["/d", "/c", "powershell.exe", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-NonInteractive", "-NoProfile", "-Command", &invoke_expression])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?.wait_with_output();
    let invoke_result = invoke_output.unwrap().clone();
    // 0 success | other = maybe prevented
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout =  String::from_utf8 (invoke_result.stdout).unwrap();
    let stderr = String::from_utf8 (invoke_result.stderr).unwrap();
    let exit_status = match exit_code {
        0 => "SUCCESS",
        1 => if pre_check { "SUCCESS" } else { "MAYBE_PREVENTED" }
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
pub fn command_execution(command: &str) -> Result<ExecutionResult, Error> {
    let command = command_with_context(asset_agent_id, raw_command);
    let working_dir = compute_working_dir(asset_agent_id);
    info!(identifier = asset_agent_id, command = &command.as_str(); "Invoking execution");
    // Write the script in specific directory
    create_dir(working_dir.clone())?;
    let script_file_name = working_dir.join("execution.sh");
    {
        let mut file = File::create(script_file_name.clone())?;
        file.write_all(command.as_bytes())?;
    }
    // Prepare and execute the command
    let command_args = &[script_file_name.to_str().unwrap(), "&"];
    let child_execution = Command::new("bash")
        .args(command_args)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()?;
    // Save execution pid
    let pid_file_name = working_dir.join("execution.pid");
    {
        let mut file = File::create(pid_file_name.clone())?;
        file.write_all(child_execution.id().to_string().as_bytes())?;
    }
    info!(identifier = asset_agent_id; "Revoking execution");
    return Ok(())
}