use serde::{Deserialize, Serialize};
use ureq::serde_json::Value;
use crate::common::error_model::Error;

use super::Client;

#[derive(Debug, Deserialize)]
pub struct PayloadArg {
    pub r#type: String,
    pub key: String,
    pub description: Option<String>,
    pub default_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InjectorContractPayload {
    pub payload_id: String,
    pub payload_type: String,
    pub payload_arguments: Vec<PayloadArg>,
    // FileDrop
    pub file_drop_file: Option<String>,
    // Prerequisites
    pub payload_prerequisites: Option<String>,
    // Command
    pub command_executor: Option<String>,
    pub command_content: Option<String>,
    // Cleanup
    pub payload_cleanup_executor: Option<String>,
    pub payload_cleanup_command: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InjectorContract {
    pub injector_contract_id: String,
    pub injector_contract_payload: InjectorContractPayload,
}

#[derive(Debug, Deserialize)]
pub struct InjectResponse {
    pub inject_id: String,
    pub inject_title: String,
    pub inject_description: Option<String>,
    pub inject_content: Value,
    pub inject_injector_contract: InjectorContract,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInjectResponse {
    #[allow(dead_code)]
    pub inject_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateInput {
    pub execution_message: String,
    pub execution_status: String,
    pub execution_duration: u128,
}

impl Client {
    pub fn get_inject(&self, inject_id: String) -> Result<InjectResponse, Error> {
        return match self.get(&format!("/api/injects/{}", inject_id)).call() {
            Ok(response) => {
                Ok(response.into_json()?)
            }
            Err(ureq::Error::Status(_, response)) => {
                Err(Error::Api(response.into_string().unwrap()))
            }
            Err(err) => {
                Err(Error::Internal(err.to_string()))
            }
        };
    }

    pub fn update_status(&self, inject_id: String, input: UpdateInput) -> Result<UpdateInjectResponse, Error> {
        let post_data = ureq::json!(input);
        return match self.post(&format!("/api/injects/execution/callback/{}", inject_id)).send_json(post_data) {
            Ok(response) => Ok(response.into_json()?),
            Err(ureq::Error::Status(_, response)) => {
                let test = response.into_string().unwrap();
                println!("ERROR> {}", test);
                Err(Error::Api(test))
            }
            Err(err) => Err(Error::Internal(err.to_string())),
        };
    }
}

/*
    DRAFT,
    INFO,
    QUEUING,
    EXECUTING,
    PENDING,
    PARTIAL,
    ERROR,
    MAYBE_PARTIAL_PREVENTED,
    MAYBE_PREVENTED,
    SUCCESS
 */