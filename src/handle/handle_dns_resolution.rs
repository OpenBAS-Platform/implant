use std::net::{SocketAddr, ToSocketAddrs};

use log::info;

use crate::api::Client;
use crate::api::manage_inject::{InjectResponse, UpdateInput};
use crate::handle::ExecutionOutput;

pub fn handle_dns_resolution(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let hostname_raw = &inject_data
        .inject_injector_contract
        .injector_contract_payload
        .dns_resolution_hostname;
    let data = hostname_raw.clone().unwrap();
    let hostnames = data.split("\n");
    for hostname in hostnames {
        // to_socket_addrs required a port to check. By default, using http 80.
        info!("dns resolution execution: {:?}", format!("{}:80", hostname));
        let addrs_command = format!("{}:80", hostname).to_socket_addrs();
        let input = match addrs_command {
            Ok(addrs) => {
                let stdout = format!(
                    "{hostname}: {}",
                    addrs
                        .map(|socket_addr: SocketAddr| {
                            return match socket_addr {
                                SocketAddr::V4(v4) => v4.ip().to_string(),
                                SocketAddr::V6(v6) => v6.ip().to_string(),
                            };
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let stderr = String::new();
                let message = ExecutionOutput {
                    action: String::from("dns resolution"),
                    stdout,
                    stderr,
                    exit_code: 0,
                };
                UpdateInput {
                    execution_message: serde_json::to_string(&message).unwrap(),
                    execution_status: String::from("SUCCESS"),
                    execution_duration: 0,
                }
            }
            Err(error) => {
                let stdout = String::new();
                let stderr = error.to_string();
                let message = ExecutionOutput {
                    action: String::from("dns resolution"),
                    stdout,
                    stderr,
                    exit_code: 1,
                };
                UpdateInput {
                    execution_message: serde_json::to_string(&message).unwrap(),
                    execution_status: String::from("ERROR"),
                    execution_duration: 0,
                }
            }
        };
        let _ = api.update_status(inject_id.clone(), input);
    }
}