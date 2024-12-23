use std::fs::File;
use std::io::{BufWriter, Write};
use std::{env, fs};

use mailparse::{parse_content_disposition, parse_header};
use serde::{Deserialize, Serialize};

use crate::common::error_model::Error;

use super::Client;

pub fn write_response<W>(writer: W, response: ureq::Response) -> std::io::Result<u64>
where
    W: Write,
{
    let mut writer = BufWriter::new(writer);
    std::io::copy(&mut response.into_reader(), &mut writer)
}

#[derive(Debug, Deserialize)]
pub struct PayloadArg {
    pub r#type: String,
    pub key: String,
    pub description: Option<String>,
    pub default_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PayloadPrerequisite {
    pub executor: String,
    pub get_command: String,
    pub check_command: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InjectorContractPayload {
    pub payload_id: String,
    pub payload_type: String,
    pub payload_arguments: Option<Vec<PayloadArg>>,
    // FileDrop
    pub file_drop_file: Option<String>,
    // Executable
    pub executable_file: Option<String>,
    // DnsResolution
    pub dns_resolution_hostname: Option<String>,
    // Prerequisites
    pub payload_prerequisites: Option<Vec<PayloadPrerequisite>>,
    // Command
    pub command_executor: Option<String>,
    pub command_content: Option<String>,
    // Cleanup
    pub payload_cleanup_executor: Option<String>,
    pub payload_cleanup_command: Option<String>,
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
    pub fn get_executable_payload(
        &self,
        inject_id: String,
    ) -> Result<InjectorContractPayload, Error> {
        match self
            .get(&format!("/api/injects/{}/executable-payload", inject_id))
            .call()
        {
            Ok(response) => Ok(response.into_json()?),
            Err(ureq::Error::Status(_, response)) => {
                Err(Error::Api(response.into_string().unwrap()))
            }
            Err(err) => Err(Error::Internal(err.to_string())),
        }
    }

    pub fn update_status(
        &self,
        inject_id: String,
        agent_id: String,
        input: UpdateInput,
    ) -> Result<UpdateInjectResponse, Error> {
        let post_data = ureq::json!(input);
        match self
            .post(&format!("/api/injects/execution/{}/callback/{}", agent_id, inject_id))
            .send_json(post_data)
        {
            Ok(response) => Ok(response.into_json()?),
            Err(ureq::Error::Status(_, response)) => {
                Err(Error::Api(response.into_string().unwrap()))
            }
            Err(err) => Err(Error::Internal(err.to_string())),
        }
    }

    pub fn download_file(&self, document_id: &String, in_memory: bool) -> Result<String, Error> {
        match self
            .get(&format!("/api/documents/{}/file", document_id))
            .call()
        {
            Ok(response) => {
                let content_disposition = response.header("content-disposition").unwrap_or("");
                let content_to_parse = format!("Content-Disposition: {}", content_disposition);
                let (parsed, _) = parse_header(content_to_parse.as_bytes()).unwrap();
                let dis = parse_content_disposition(&parsed.get_value());
                let current_exe_patch = env::current_exe().unwrap();
                let executable_path = current_exe_patch.parent().unwrap();
                let name = dis.params.get("filename").unwrap();
                let file_directory = executable_path.join(name);
                if in_memory {
                    let buf = BufWriter::new(Vec::new());
                    let _ = write_response(buf, response);
                    Ok(String::from(name))
                } else {
                    let output_file = File::create(file_directory.clone()).unwrap();
                    let file_write = write_response(output_file, response);
                    return match file_write {
                        Ok(_) => Ok(String::from(name)),
                        Err(err) => {
                            let _ = fs::remove_file(file_directory.clone());
                            return Err(Error::Io(err));
                        }
                    };
                }
            }
            Err(ureq::Error::Status(_, response)) => {
                Err(Error::Api(response.into_string().unwrap()))
            }
            Err(err) => Err(Error::Internal(err.to_string())),
        }
    }
}
