use std::env;
use std::path::PathBuf;
use std::time::Instant;

use log::info;

use crate::api::Client;
use crate::api::manage_inject::{InjectorContractPayload};
use crate::handle::handle_execution::handle_execution_result;
use crate::process::command_exec::command_execution;

fn compute_working_dir() -> PathBuf {
    let current_exe_patch = env::current_exe().unwrap();
    return current_exe_patch.parent().unwrap().to_path_buf();
}

pub fn compute_command(command: &String) -> String {
    let executable_command = command.clone();
    let working_dir = compute_working_dir();
    return executable_command.replace("#{location}", working_dir.to_str().unwrap());
}

pub fn handle_execution_command(
    semantic: &str,
    api: &Client,
    inject_id: String,
    agent_id: String,
    command: &String,
    executor: &String,
    pre_check: bool,
) -> i32 {
    let now = Instant::now();
    info!("{} execution: {:?}", semantic, command);
    let command_result = command_execution(command.as_str(), executor.as_str(), pre_check);
    let elapsed = now.elapsed().as_millis();
    return handle_execution_result(semantic, api, inject_id, agent_id, command_result, elapsed);
}

pub fn handle_command(inject_id: String, agent_id: String, api: &Client, contract_payload: &InjectorContractPayload) {
    let command = contract_payload.command_content.clone().unwrap();
    let executor = contract_payload.command_executor.clone().unwrap();
    let executable_command = compute_command(&command);
    let _ = handle_execution_command(
        "implant execution",
        &api,
        inject_id.clone(),
        agent_id.clone(),
        &executable_command,
        &executor,
        false,
    );
}
