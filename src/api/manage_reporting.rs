use crate::api::Client;
use crate::api::manage_inject::UpdateInput;
use crate::handle::ExecutionOutput;

pub fn report_success(
    api: &Client,
    semantic: &str,
    inject_id: String,
    agent_id: String,
    stdout: String,
    stderr: Option<String>,
    duration: u128,
) {
    let message = ExecutionOutput {
        action: String::from(semantic),
        stderr: stderr.unwrap_or_default(),
        stdout,
        exit_code: -1,
    };
    let execution_message = serde_json::to_string(&message).unwrap();
    let _ = api.update_status(
        inject_id.clone(),
        agent_id.clone(),
        UpdateInput {
            execution_message,
            execution_status: String::from("SUCCESS"),
            execution_duration: duration,
        },
    );
}

pub fn report_error(
    api: &Client,
    semantic: &str,
    inject_id: String,
    agent_id: String,
    stdout: Option<String>,
    stderr: String,
    duration: u128,
) {
    let message = ExecutionOutput {
        action: String::from(semantic),
        stdout: stdout.unwrap_or_default(),
        stderr,
        exit_code: -1,
    };
    let execution_message = serde_json::to_string(&message).unwrap();
    let _ = api.update_status(
        inject_id.clone(),
        agent_id.clone(),
        UpdateInput {
            execution_message,
            execution_status: String::from("ERROR"),
            execution_duration: duration,
        },
    );
}
