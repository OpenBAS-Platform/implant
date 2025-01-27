use log::info;

use crate::api::manage_inject::UpdateInput;
use crate::api::Client;
use crate::common::error_model::Error;
use crate::handle::ExecutionOutput;
use crate::process::command_exec::ExecutionResult;

pub fn handle_execution_result(
    semantic: &str,
    api: &Client,
    inject_id: String,
    agent_id: String,
    command_result: Result<ExecutionResult, Error>,
    elapsed: u128,
) -> i32 {
    match command_result {
        Ok(res) => {
            info!("{} execution stdout: {:?}", semantic, res.stdout);
            info!("{} execution stderr: {:?}", semantic, res.stderr);
            let stdout = res.stdout;
            let stderr = res.stderr;
            let exit_code = res.exit_code;
            let message = ExecutionOutput {
                stdout,
                stderr,
                exit_code,
            };
            let execution_message = serde_json::to_string(&message).unwrap();
            let _ = api.update_status(
                inject_id,
                agent_id,
                UpdateInput {
                    execution_message,
                    execution_status: res.status,
                    execution_duration: elapsed,
                    execution_action: String::from(semantic),
                },
            );
            // Return execution code
            res.exit_code
        }
        Err(err) => {
            info!("implant execution error: {:?}", err);
            let stderr = format!("{:?}", err);
            let stdout = String::new();
            let message = ExecutionOutput {
                stderr,
                stdout,
                exit_code: -1,
            };
            let execution_message = serde_json::to_string(&message).unwrap();
            let _ = api.update_status(
                inject_id,
                agent_id,
                UpdateInput {
                    execution_message,
                    execution_status: String::from("ERROR"),
                    execution_duration: elapsed,
                    execution_action: String::from(semantic),
                },
            );
            // Return error code
            -1
        }
    }
}
