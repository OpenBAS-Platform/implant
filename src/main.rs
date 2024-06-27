mod common;
mod api;
mod process;

use std::env;
use std::path::{PathBuf};
use std::sync::atomic::{AtomicBool};
use std::time::Instant;
use clap::Parser;
use regex::Regex;
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use serde::{Deserialize, Serialize};
use ureq::serde_json;
use std::net::{SocketAddr, ToSocketAddrs};
use log::info;
use crate::api::Client;
use crate::api::manage_inject::{InjectorContract, InjectorContractPayload, InjectResponse, UpdateInput};
use crate::common::error_model::Error;
use crate::process::command_exec::{command_execution, ExecutionResult};
use crate::process::file_exec::file_execution;

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
    let re = Regex::new(r"#\{([^#{]+)}").unwrap();
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
                let InjectResponse{ inject_injector_contract, ..} = inject_data;
                let InjectorContract { injector_contract_payload, ..} = inject_injector_contract;
                let InjectorContractPayload { payload_arguments, ..} = injector_contract_payload;
                let arguments = match payload_arguments {
                    None => &vec![],
                    Some(args) => args
                };
                let arg = arguments.iter().find(|arg| arg.key == parameter);
                if arg.is_some() {
                    let arg_value = arg.unwrap();
                    let default_value = arg_value.default_value.clone();
                    if default_value.is_some() {
                        executable_command = executable_command.replace(key.as_str(), default_value.unwrap().as_str())
                    }
                }
            }
        }
    }
    let working_dir = compute_working_dir();
    return executable_command.replace("#{location}", working_dir.to_str().unwrap());
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionOutput {
    pub action: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn handle_execution_result(semantic: &str, api: &Client, inject_id: String, command_result: Result<ExecutionResult, Error>, elapsed: u128) -> i32 {
    return match command_result {
        Ok(res) => {
            info!("{} execution stdout: {:?}", semantic, res.stdout);
            info!("{} execution stderr: {:?}", semantic, res.stderr);
            let stdout = res.stdout;
            let stderr = res.stderr;
            let exit_code = res.exit_code;
            let message = ExecutionOutput { action: String::from(semantic), stdout, stderr, exit_code };
            let execution_message = serde_json::to_string(&message).unwrap();
            let _ = api.update_status(inject_id, UpdateInput {
                execution_message,
                execution_status: res.status,
                execution_duration: elapsed
            });
            // Return execution code
            res.exit_code
        }
        Err(err) => {
            info!("implant execution error: {:?}", err);
            let stderr = format!("{:?}", err);
            let stdout = String::new();
            let message = ExecutionOutput { action: String::from(semantic), stderr, stdout, exit_code: -1 };
            let execution_message = serde_json::to_string(&message).unwrap();
            let _ = api.update_status(inject_id, UpdateInput {
                execution_message,
                execution_status: String::from("ERROR"),
                execution_duration: elapsed
            });
            // Return error code
            -1
        }
    };
}

pub fn handle_execution_command(semantic: &str, api: &Client, inject_id: String, command: &String, pre_check: bool) -> i32 {
    let now = Instant::now();
    info!("{} execution: {:?}", semantic, command);
    let command_result = command_execution(command.as_str(), pre_check);
    let elapsed = now.elapsed().as_millis();
    return handle_execution_result(semantic, api, inject_id, command_result, elapsed)
}

pub fn handle_execution_file(semantic: &str, api: &Client, inject_id: String, filename: &String) -> i32 {
    let now = Instant::now();
    info!("{} execution: {:?}", semantic, filename);
    let command_result = file_execution(filename.as_str());
    let elapsed = now.elapsed().as_millis();
    return handle_execution_result(semantic, api, inject_id, command_result, elapsed)
}

pub fn handle_command(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let contract_payload = &inject_data.inject_injector_contract.injector_contract_payload;
    let command = contract_payload.command_content.clone().unwrap();
    let executable_command = compute_command(&command, &inject_data);
    let _ = handle_execution_command("implant execution", &api,
                                    inject_id.clone(), &executable_command, false);
}

pub fn handle_dns_resolution(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let hostname_raw = &inject_data.inject_injector_contract.injector_contract_payload.dns_resolution_hostname;
    let data = hostname_raw.clone().unwrap();
    let hostnames = data.split("\n");
    for hostname in hostnames {
        // to_socket_addrs required a port to check. By default, using http 80.
        info!("dns resolution execution: {:?}", format!("{}:80", hostname));
        let addrs_command = format!("{}:80", hostname).to_socket_addrs();
        let input = match addrs_command {
            Ok(addrs) => {
                let stdout = format!("{hostname}: {}", addrs.map(|socket_addr: SocketAddr| {
                    return match socket_addr {
                        SocketAddr::V4(v4) => v4.ip().to_string(),
                        SocketAddr::V6(v6) => v6.ip().to_string()
                    }
                }).collect::<Vec<_>>().join(", "));
                let stderr = String::new();
                let message = ExecutionOutput { action: String::from("dns resolution"), stdout, stderr, exit_code: 0 };
                UpdateInput {
                    execution_message: serde_json::to_string(&message).unwrap(),
                    execution_status: String::from("SUCCESS"),
                    execution_duration: 0
                }
            }
            Err(error) => {
                let stdout = String::new();
                let stderr = error.to_string();
                let message = ExecutionOutput { action: String::from("dns resolution"), stdout, stderr, exit_code: 1 };
                UpdateInput {
                    execution_message: serde_json::to_string(&message).unwrap(),
                    execution_status: String::from("ERROR"),
                    execution_duration: 0
                }
            }
        };
        let _ = api.update_status(inject_id.clone(), input);
    }
}

pub fn report_success(api: &Client, semantic: &str, inject_id: String, stdout: String, stderr: Option<String>, duration: u128) {
    let message = ExecutionOutput { action: String::from(semantic), stderr: stderr.unwrap_or(String::new()), stdout, exit_code: -1 };
    let execution_message = serde_json::to_string(&message).unwrap();
    let _ = api.update_status(inject_id.clone(), UpdateInput {
        execution_message,
        execution_status: String::from("SUCCESS"),
        execution_duration: duration
    });
}

pub fn report_error(api: &Client, semantic: &str, inject_id: String, stdout: Option<String>, stderr: String, duration: u128) {
    let message = ExecutionOutput { action: String::from(semantic), stdout: stdout.unwrap_or(String::new()), stderr, exit_code: -1 };
    let execution_message = serde_json::to_string(&message).unwrap();
    let _ = api.update_status(inject_id.clone(), UpdateInput {
        execution_message,
        execution_status: String::from("ERROR"),
        execution_duration: duration
    });
}

pub fn handle_file(inject_id: String, api: &Client, file_target: &Option<String>, in_memory: bool) -> Result<String, Error> {
    return match file_target {
        None => {
            let stderr = String::from("Payload download fail, document not specified");
            report_error(api, "file drop", inject_id.clone(), None, stderr.clone(), 0);
            Err(Error::Internal(stderr))
        }
        Some(document_id) => {
            let now = Instant::now();
            let download = api.download_file(document_id, in_memory);
            let elapsed = now.elapsed().as_millis();
            match download {
                Ok(filename) => {
                    let stdout = String::from("File downloaded with success");
                    report_success(api, "file drop", inject_id.clone(), stdout, None, elapsed);
                    Ok(filename)
                }
                Err(err) => {
                    let stderr = format!("{:?}", err);
                    report_error(api, "file drop", inject_id.clone(), None, stderr, elapsed);
                    Err(err)
                }
            }
        }
    }
}

pub fn handle_file_drop(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let InjectResponse{ inject_injector_contract, ..} = inject_data;
    let InjectorContract { injector_contract_payload, ..} = inject_injector_contract;
    let InjectorContractPayload { file_drop_file, ..} = injector_contract_payload;
    let _ = handle_file(inject_id, api, file_drop_file, false);
}

pub fn handle_file_execute(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let InjectResponse{ inject_injector_contract, ..} = inject_data;
    let InjectorContract { injector_contract_payload, ..} = inject_injector_contract;
    let InjectorContractPayload { executable_file, ..} = injector_contract_payload;
    let handle_file = handle_file(inject_id.clone(), api, executable_file, false);
    match handle_file {
        Ok(filename) => {
            handle_execution_file("file execution", api, inject_id.clone(), &filename);
        }
        Err(_) => {
            // Nothing to do here as handle by handle_file
        }
    }
}

pub fn handle_payload(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let mut prerequisites_code = 0;
    let contract_payload = &inject_data.inject_injector_contract.injector_contract_payload;
    // region prerequisite execution
    let prerequisites_data = &contract_payload.payload_prerequisites;
    let prerequisites = match prerequisites_data {
        None => &vec![],
        Some(args) => args
    };
    for prerequisite in prerequisites.iter() {
        let mut check_status = 0;
        let check_cmd = &prerequisite.check_command;
        if check_cmd.is_some() {
            let check_prerequisites = compute_command(check_cmd.as_ref().unwrap(), &inject_data);
            check_status = handle_execution_command("prerequisite check", &api,
                                                   inject_id.clone(), &check_prerequisites, true);
        }
        // If exit 0, prerequisite are already satisfied
        if check_status != 0 {
            let install_prerequisites = compute_command(&prerequisite.get_command, &inject_data);
            prerequisites_code += handle_execution_command("prerequisite execution", &api,
                                                   inject_id.clone(), &install_prerequisites, false);
        }
    }
    // endregion
    // region implant execution
    // If prerequisites succeed to be executed, execute the command.
    if prerequisites_code == 0 {
        let payload_type = &contract_payload.payload_type;
        match payload_type.as_str() {
            "Command" => handle_command(inject_id.clone(), &api, &inject_data),
            "DnsResolution" => handle_dns_resolution(inject_id.clone(), &api, &inject_data),
            "Executable" => handle_file_execute(inject_id.clone(), &api, &inject_data),
            "FileDrop" => handle_file_drop(inject_id.clone(), &api, &inject_data),
            // "NetworkTraffic" => {}, // Not implemented yet
            _ => {
                let _ = api.update_status(inject_id.clone(), UpdateInput {
                    execution_message: String::from("Payload execution type not supported."),
                    execution_status: String::from("ERROR"),
                    execution_duration: 0
                });
            }
        }
    } else {
        let _ = api.update_status(inject_id.clone(), UpdateInput {
            execution_message: String::from("Payload execution not executed due to dependencies failure."),
            execution_status: String::from("ERROR"),
            execution_duration: 0
        });
    }
    // endregion
    // region cleanup execution
    // Cleanup command will be executed independently of the previous commands success.
    let cleanup = contract_payload.payload_cleanup_command.clone();
    if cleanup.is_some() {
        let executable_cleanup = compute_command(&cleanup.unwrap(), &inject_data);
        let _ = handle_execution_command("prerequisite cleanup", &api,
                                        inject_id.clone(), &executable_cleanup, false);
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
    handle_payload(args.inject_id.clone(), &api, &inject_data);
    // endregion
    return Ok(())
}


