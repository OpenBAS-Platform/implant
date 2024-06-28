use crate::api::Client;
use crate::api::manage_inject::{InjectorContract, InjectorContractPayload, InjectResponse};
use crate::handle::handle_file::handle_file;

pub fn handle_file_drop(inject_id: String, api: &Client, inject_data: &InjectResponse) {
    let InjectResponse {
        inject_injector_contract,
        ..
    } = inject_data;
    let InjectorContract {
        injector_contract_payload,
        ..
    } = inject_injector_contract;
    let InjectorContractPayload { file_drop_file, .. } = injector_contract_payload;
    let _ = handle_file(inject_id, api, file_drop_file, false);
}
