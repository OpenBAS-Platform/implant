use std::env;
use std::path::PathBuf;
use std::time::Instant;

use log::info;
use regex::Regex;

use crate::api::Client;
use crate::api::manage_inject::{InjectorContract, InjectorContractPayload, InjectResponse};
use crate::handle::handle_execution::handle_execution_result;
use crate::process::command_exec::command_execution;

pub fn compute_parameters(command: &String) -> Vec<&str> {
    let re = Regex::new(r"#\{([^#{}]+)}").unwrap();
    let mut command_parameters = vec![];
    for (_, [id]) in re.captures_iter(command).map(|c| c.extract()) {
        command_parameters.push(id);
    }
    return command_parameters;
}

fn compute_working_dir() -> PathBuf {
    let current_exe_patch = env::current_exe().unwrap();
    return current_exe_patch.parent().unwrap().to_path_buf();
}

pub fn compute_command(command: &String, inject_data: &InjectResponse) -> String {
    let command_parameters = compute_parameters(&command);
    let mut executable_command = command.clone();
    if command_parameters.len() > 0 {
        let config_map = inject_data.inject_content.as_object().unwrap();
        for parameter in command_parameters {
            let key = format!("#{{{}}}", parameter);
            // Try to fill the values with user arguments
            let param_value = config_map.get(parameter).unwrap().as_str();
            if param_value.is_some() {
                executable_command = executable_command.replace(key.as_str(), param_value.unwrap())
            } else {
                // Try to fill the values with default
                let InjectResponse {
                    inject_injector_contract,
                    ..
                } = inject_data;
                let InjectorContract {
                    injector_contract_payload,
                    ..
                } = inject_injector_contract;
                let InjectorContractPayload {
                    payload_arguments, ..
                } = injector_contract_payload;
                let empty_arguments = vec![];
                let arguments = match payload_arguments {
                    None => &empty_arguments,
                    Some(args) => args,
                };
                let arg = arguments.iter().find(|arg| arg.key == parameter);
                if arg.is_some() {
                    let arg_value = arg.unwrap();
                    let default_value = arg_value.default_value.clone();
                    if default_value.is_some() {
                        executable_command = executable_command
                            .replace(key.as_str(), default_value.unwrap().as_str())
                    }
                }
            }
        }
    }
    let working_dir = compute_working_dir();
    return executable_command.replace("#{location}", working_dir.to_str().unwrap());
}

pub fn handle_execution_command(
    semantic: &str,
    api: &Client,
    inject_id: String,
    command: &String,
    executor: &String,
    pre_check: bool,
) -> i32 {
    let now = Instant::now();
    info!("{} execution: {:?}", semantic, command);
    let command_result = command_execution(command.as_str(), executor.as_str(), pre_check);
    let elapsed = now.elapsed().as_millis();
    return handle_execution_result(semantic, api, inject_id, command_result, elapsed);
}

pub fn handle_command(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let contract_payload = &inject_data
        .inject_injector_contract
        .injector_contract_payload;
    let command = contract_payload.command_content.clone().unwrap();
    let executor = contract_payload.command_executor.clone().unwrap();
    let executable_command = compute_command(&command, &inject_data);
    let _ = handle_execution_command(
        "implant execution",
        &api,
        inject_id.clone(),
        &executable_command,
        &executor,
        false,
    );
}
