mod common;
mod api;
mod process;

use std::env;
use std::sync::atomic::{AtomicBool};
use std::time::Instant;
use clap::Parser;
use log::{info};
use regex::Regex;
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use crate::api::Client;
use crate::api::manage_inject::{InjectResponse, UpdateInput};
use crate::common::error_model::Error;
use crate::process::command_exec::{command_execution};

pub static THREADS_CONTROL: AtomicBool = AtomicBool::new(true);
const ENV_PRODUCTION: &str = "production";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const PREFIX_LOG_NAME: &str = "openbas-implant.log";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    uri: String,
    #[arg(short, long)]
    token: String,
    #[arg(short, long)]
    inject_id: String,
}

pub fn mode() -> String {
    return env::var("env").unwrap_or_else(|_| ENV_PRODUCTION.into())
}

pub fn compute_parameters(command: &String) -> Vec<&str> {
    let re = Regex::new(r"#\{([^#]+)}").unwrap();
    let mut command_parameters = vec![];
    for (_, [id]) in re.captures_iter(command).map(|c| c.extract()) {
        command_parameters.push(id);
    }
    return command_parameters;
}

pub fn compute_command(command: &String, inject_data: &InjectResponse) -> String {
    let contract_payload = &inject_data.inject_injector_contract.injector_contract_payload;
    let command_parameters = compute_parameters(&command);
    if command_parameters.len() > 0 {
        let mut executable_command = command.clone();
        let config_map = inject_data.inject_content.as_object().unwrap();
        for parameter in command_parameters {
            let key = format!("#{{{}}}", parameter);
            // Try to fill the values with user arguments
            let param_value = config_map.get(parameter).unwrap().as_str();
            if param_value.is_some() {
                executable_command = executable_command.replace(key.as_str(), param_value.unwrap())
            } else {
                // Try to fill the values with default
                let arg = contract_payload.payload_arguments.iter().find(|arg| arg.key == parameter);
                if arg.is_some() {
                    let arg_value = arg.unwrap();
                    let default_value = arg_value.default_value.clone();
                    if default_value.is_some() {
                        executable_command = executable_command.replace(key.as_str(), default_value.unwrap().as_str())
                    }
                }
            }
        }
        return executable_command;
    }
    return command.clone();
}

pub fn handle_execution_result(semantic: &str, api: &Client, inject_id: String, command: &String) -> i32 {
    let now = Instant::now();
    println!("{} execution: {:?}", semantic, command);
    let command_result = command_execution(command.as_str());
    let elapsed = now.elapsed().as_millis();
    return match command_result {
        Ok(res) => {
            println!("{} execution stdout: {:?}", semantic, res.stdout);
            println!("{} execution stderr: {:?}", semantic, res.stderr);
            let stdout = res.stdout;
            let stderr = res.stderr;
            let _ = api.update_status(inject_id, UpdateInput {
                execution_message: format!("stdout: {stdout}, stderr: {stderr}"),
                execution_status: res.status,
                execution_duration: elapsed
            });
            // Return execution code
            res.exit_code
        }
        Err(err) => {
            println!("implant execution error {:?}", err);
            let _ = api.update_status(inject_id, UpdateInput {
                execution_message: format!("stderr: {err}"),
                execution_status: String::from("ERROR"),
                execution_duration: elapsed
            });
            // Return error code
            -1
        }
    };
}

pub fn handle_command(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let mut prerequisites_code = 0;
    let contract_payload = &inject_data.inject_injector_contract.injector_contract_payload;
    // region prerequisite check
    // TODO
    // endregion
    // region prerequisite execution
    let prerequisites = contract_payload.payload_prerequisites.clone();
    if prerequisites.is_some() {
        let executable_prerequisites = compute_command(&prerequisites.unwrap(), &inject_data);
        prerequisites_code += handle_execution_result("prerequisite", &api,
                                                      inject_id.clone(), &executable_prerequisites);
    }
    // endregion
    // region implant execution
    // If prerequisites succeed to be executed, execute the command.
    if prerequisites_code == 0 {
        let command = contract_payload.command_content.clone().unwrap();
        let executable_command = compute_command(&command, &inject_data);
        let _ = handle_execution_result("implant", &api,
                                        inject_id.clone(), &executable_command);
    }
    // endregion
    // region cleanup execution
    // Cleanup command will be executed independently of the previous commands success.
    let cleanup = contract_payload.payload_cleanup_command.clone();
    if cleanup.is_some() {
        let executable_cleanup = compute_command(&cleanup.unwrap(), &inject_data);
        let _ = handle_execution_result("cleanup", &api,
                                        inject_id.clone(), &executable_cleanup);
    }
    // endregion
}

fn main() -> Result<(), Error> {
    // region Init logger
    let current_exe_patch = env::current_exe().unwrap();
    let parent_path = current_exe_patch.parent().unwrap();
    let log_file = parent_path.join(PREFIX_LOG_NAME);
    let condition = RollingConditionBasic::new().daily();
    let file_appender = BasicRollingFileAppender::new(log_file, condition, 3).unwrap();
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().json().with_writer(file_writer).init();
    // endregion
    // region Process execution
    let args = Args::parse();
    info!("Starting OpenBAS implant {} {}", VERSION, mode());
    let api = Client::new(args.uri, args.token);
    let inject = api.get_inject(args.inject_id.clone());
    let inject_data = inject.unwrap_or_else(|err| panic!("Fail getting inject {}", err));
    let contract_payload = &inject_data.inject_injector_contract.injector_contract_payload;
    let payload_type = &contract_payload.payload_type;
    match payload_type.as_str() {
        "Command" => handle_command(args.inject_id.clone(), &api, &inject_data),
        "DnsResolution" => {},
        "Executable" => {},
        "FileDrop" => {},
        "NetworkTraffic" => {},
        _ => panic!("Unsupported payload type {}", payload_type)
    }
    // endregion
    return Ok(())
}


