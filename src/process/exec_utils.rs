use std::process::Command;

pub fn is_executor_present(executor: &str) -> bool {
    Command::new(executor)
        .spawn()
        .map(|mut child| child.kill().is_ok())
        .unwrap_or(false)
}
