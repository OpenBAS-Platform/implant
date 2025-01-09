use std::env;
use std::sync::atomic::AtomicBool;

use clap::Parser;
use log::info;
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};

use crate::api::manage_inject::{InjectorContractPayload, UpdateInput};
use crate::api::Client;
use crate::common::error_model::Error;
use crate::handle::handle_command::{compute_command, handle_command, handle_execution_command};
use crate::handle::handle_dns_resolution::handle_dns_resolution;
use crate::handle::handle_file_drop::handle_file_drop;
use crate::handle::handle_file_execute::handle_file_execute;

mod api;
mod common;
mod handle;
mod process;

#[cfg(test)]
mod tests;

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
    unsecured_certificate: String,
    #[arg(short, long)]
    with_proxy: String,
    #[arg(short, long)]
    agent_id: String,
    #[arg(short, long)]
    inject_id: String,
}

pub fn mode() -> String {
    env::var("env").unwrap_or_else(|_| ENV_PRODUCTION.into())
}

pub fn handle_payload(
    inject_id: String,
    agent_id: String,
    api: &Client,
    contract_payload: &InjectorContractPayload,
) {
    let mut prerequisites_code = 0;
    // region prerequisite execution
    let prerequisites_data = &contract_payload.payload_prerequisites;
    let empty_prerequisites = vec![];
    let prerequisites = match prerequisites_data {
        None => &empty_prerequisites,
        Some(args) => args,
    };
    for prerequisite in prerequisites.iter() {
        let mut check_status = 1;
        let check_cmd = &prerequisite.check_command;
        if check_cmd.is_some() && !check_cmd.clone().unwrap().is_empty() {
            let check_prerequisites = compute_command(check_cmd.as_ref().unwrap());
            check_status = handle_execution_command(
                "prerequisite check",
                api,
                inject_id.clone(),
                agent_id.clone(),
                &check_prerequisites,
                &prerequisite.executor,
                true,
            );
        }
        // If exit 0, prerequisite are already satisfied
        if check_status != 0 {
            let install_prerequisites = compute_command(&prerequisite.get_command);
            prerequisites_code += handle_execution_command(
                "prerequisite execution",
                api,
                inject_id.clone(),
                agent_id.clone(),
                &install_prerequisites,
                &prerequisite.executor,
                false,
            );
        }
    }
    // endregion
    // region implant execution
    // If prerequisites succeed to be executed, execute the command.
    if prerequisites_code == 0 {
        let payload_type = &contract_payload.payload_type;
        match payload_type.as_str() {
            "Command" => {
                handle_command(inject_id.clone(), agent_id.clone(), &api, &contract_payload)
            }
            "DnsResolution" => {
                handle_dns_resolution(inject_id.clone(), agent_id.clone(), &api, &contract_payload)
            }
            "Executable" => {
                handle_file_execute(inject_id.clone(), agent_id.clone(), &api, &contract_payload)
            }
            "FileDrop" => {
                handle_file_drop(inject_id.clone(), agent_id.clone(), &api, &contract_payload)
            }
            // "NetworkTraffic" => {}, // Not implemented yet
            _ => {
                let _ = api.update_status(
                    inject_id.clone(),
                    agent_id.clone(),
                    UpdateInput {
                        execution_message: String::from("Payload execution type not supported."),
                        execution_status: String::from("ERROR"),
                        execution_duration: 0,
                    },
                );
            }
        }
    } else {
        let _ = api.update_status(
            inject_id.clone(),
            agent_id.clone(),
            UpdateInput {
                execution_message: String::from(
                    "Payload execution not executed due to dependencies failure.",
                ),
                execution_status: String::from("ERROR"),
                execution_duration: 0,
            },
        );
    }
    // endregion
    // region cleanup execution
    // Cleanup command will be executed independently of the previous commands success.
    let cleanup = contract_payload.payload_cleanup_command.clone();
    if cleanup.is_some() && !cleanup.clone().unwrap().is_empty() {
        let executable_cleanup = compute_command(&cleanup.unwrap());
        let executor = contract_payload.payload_cleanup_executor.clone().unwrap();
        let _ = handle_execution_command(
            "cleanup execution",
            api,
            inject_id.clone(),
            agent_id.clone(),
            &executable_cleanup,
            &executor,
            false,
        );
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
    tracing_subscriber::fmt()
        .json()
        .with_writer(file_writer)
        .init();
    // endregion
    // region Process execution

    let args = Args::parse();
    info!("Starting OpenBAS implant {} {}", VERSION, mode());
    let api = Client::new(
        args.uri,
        args.token,
        args.unsecured_certificate == "true",
        args.with_proxy == "true",
    );
    let payload = api.get_executable_payload(args.inject_id.clone());
    let contract_payload = payload.unwrap_or_else(|err| panic!("Fail getting payload {}", err));
    handle_payload(
        args.inject_id.clone(),
        args.agent_id.clone(),
        &api,
        &contract_payload,
    );
    // endregion
    Ok(())
}
