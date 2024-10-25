use std::process::{Command, Output, Stdio};

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

pub fn invoke_command(command: &str, executor: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(executor)
        .args(args)
        .arg(command)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
}

// TODO refacto all on monday, just save for the weekend
#[cfg(target_os = "windows")]
pub fn command_execution(command: &str, executor: &str, pre_check: bool) -> Result<ExecutionResult, Error> {
    let invoke_output;
    if executor == "cmd" {
        // To manage multi lines (we need more than just base64 like the other executor)
        // TODO warning \n can be found in windows path !?
        let new_command = command.replace("\n", " & ");
        let invoke_expression = format!("([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\")))", BASE64_STANDARD.encode(new_command));
        let echo_cmd = Command::new("powershell.exe")
            .arg(&invoke_expression)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        invoke_output = Command::new("cmd")
            .stdin(Stdio::from(echo_cmd.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();
    } else if executor == "bash" {
        let echo_cmd = Command::new("bash")
            .arg("-c")
            .arg("echo ".to_owned() + &BASE64_STANDARD.encode(command) + " | base64 --d")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        invoke_output = Command::new("bash")
            .stdin(Stdio::from(echo_cmd.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();
    } else if executor == "sh" {
        let echo_cmd = Command::new("sh")
            .arg("-c")
            .arg("echo ".to_owned() + &BASE64_STANDARD.encode(command) + " | base64 --d")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        invoke_output = Command::new("sh")
            .stdin(Stdio::from(echo_cmd.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();
    } else {
        // For powershell complex command, we need to encode in base64 to manage escape caracters
        let invoke_expression = format!("Invoke-Expression ([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\")))", BASE64_STANDARD.encode(command));
        invoke_output = invoke_command(&invoke_expression, "cmd.exe", &[
            "/d",
            "/c",
            "powershell.exe",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-NonInteractive",
            "-NoProfile",
            "-Command"]);
    }
    let invoke_result = invoke_output.unwrap().clone();
    // 0 success | other = maybe prevented
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout = String::from_utf8(invoke_result.stdout).unwrap();
    let stderr = String::from_utf8(invoke_result.stderr).unwrap();
    let exit_status = match exit_code {
        0 => "SUCCESS",
        1 => {
            if pre_check {
                "SUCCESS"
            } else {
                "MAYBE_PREVENTED"
            }
        }
        -99 => "ERROR",
        _ => "MAYBE_PREVENTED",
    };
    return Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        status: String::from(exit_status),
    });
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn command_execution(command: &str, executor: &str, pre_check: bool) -> Result<ExecutionResult, Error> {
    let invoke_output;
    if executor == "bash" {
        // For shell complex command, we need to encode in base64 to manage escape caracters
        let echo_child = Command::new("echo")
            .arg(BASE64_STANDARD.encode(command))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let base64_child = Command::new("base64")
            .arg("-d")
            .stdin(Stdio::from(echo_child.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        invoke_output = Command::new("bash")
            .stdin(Stdio::from(base64_child.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();
    } else if executor == "psh" {
        let invoke_expression = format!("Invoke-Expression ([System.Text.Encoding]::UTF8.GetString([convert]::FromBase64String(\"{}\")))", BASE64_STANDARD.encode(command));
        invoke_output = invoke_command(&invoke_expression, "powershell", &["-c"]);
    } else {
        // For shell complex command, we need to encode in base64 to manage escape caracters
        let echo_child = Command::new("echo")
            .arg(BASE64_STANDARD.encode(command))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let base64_child = Command::new("base64")
            .arg("-d")
            .stdin(Stdio::from(echo_child.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        invoke_output = Command::new("sh")
            .stdin(Stdio::from(base64_child.stdout.unwrap()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output();
    }

    let invoke_result = invoke_output.unwrap().clone();
    // 0 success | other = maybe prevented
    let exit_code = invoke_result.status.code().unwrap_or_else(|| -99);
    let stdout = String::from_utf8(invoke_result.stdout).unwrap();
    let stderr = String::from_utf8(invoke_result.stderr).unwrap();
    let exit_status = match exit_code {
        0 => "SUCCESS",
        1 => {
            if pre_check {
                "SUCCESS"
            } else {
                "MAYBE_PREVENTED"
            }
        }
        -99 => "ERROR",
        _ => "MAYBE_PREVENTED",
    };
    return Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        status: String::from(exit_status),
    });
}
