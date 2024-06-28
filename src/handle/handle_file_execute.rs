use crate::api::Client;
use crate::api::manage_inject::{InjectorContract, InjectorContractPayload, InjectResponse};
use crate::handle::handle_file::{handle_execution_file, handle_file};

pub fn handle_file_execute(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let InjectResponse {
        inject_injector_contract,
        ..
    } = inject_data;
    let InjectorContract {
        injector_contract_payload,
        ..
    } = inject_injector_contract;
    let InjectorContractPayload {
        executable_file, ..
    } = injector_contract_payload;
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
