use std::collections::HashMap;
use std::process::Command;



pub fn is_executor_present(executor: &str) -> bool {
    Command::new(actual_executor)
        .spawn()
        .map(|mut child| child.kill().is_ok()) // Kill immediately to avoid hanging.
        .unwrap_or(false)
}