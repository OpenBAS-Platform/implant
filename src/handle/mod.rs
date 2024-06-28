use serde::{Deserialize, Serialize};

pub mod handle_command;
pub mod handle_dns_resolution;
mod handle_execution;
pub mod handle_file;
pub mod handle_file_drop;
pub mod handle_file_execute;

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionOutput {
    pub action: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
