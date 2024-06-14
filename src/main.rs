mod common;

use std::env;
use std::sync::atomic::{AtomicBool};
use clap::Parser;
use log::info;
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use crate::common::error_model::Error;

pub static THREADS_CONTROL: AtomicBool = AtomicBool::new(true);
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
    payload_id: String,
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
    info!("Starting OpenBAS implant {} {}", VERSION, args.uri);
    println!("Starting OpenBAS implant {}", VERSION);
    // endregion
    return Ok(())
}


