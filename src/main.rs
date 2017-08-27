#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate log;

extern crate clap;
extern crate libc;
extern crate log4rs;
extern crate procinfo;

mod cmdargs;
mod errors;
mod process_utils;

use cmdargs::build_cli;
use errors::Error;
use process_utils::{find_processes, restart_process};
use std::time::Duration;

fn run() -> Result<(), Error> {
    let matches = build_cli().get_matches();

    log4rs::init_file(
        matches.value_of("LOG_CONFIG").expect("No log config"),
        Default::default(),
    )?;
    let program_name = matches.value_of("NAME").expect("Name not found");
    let threshold: usize = matches
        .value_of("THRESHOLD")
        .expect("Threshold not found")
        .parse()?;
    let cmd = matches.value_of("CMD").expect("Command not found");
    let cmd_args = matches.values_of("ARGS").unwrap_or_default();
    let timeout = Duration::from_secs(matches
        .value_of("TIMEOUT")
        .expect("Can't get timeout.")
        .parse()?);

    let processes = find_processes(&program_name)?;
    if processes.len() > 1 {
        Err(Error::MoreThanOne)?;
    }
    let process = match processes.into_iter().nth(0) {
        Some(x) => x,
        None => Err(Error::NotFound)?,
    };
    let memory = process.get_memory();
    info!["Memory: {} kilobytes", memory as f64 / 1024f64];
    if memory > threshold {
        warn!["Threshold exceeded"];
        restart_process(process, timeout, cmd, cmd_args)?;
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => {
            error![
                "Program ended with an error: {:?}, caused by ",
                e,
            ]
        }
    }
}
