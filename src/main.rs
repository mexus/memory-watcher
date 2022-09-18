//! Process memory watcher.

#![deny(missing_docs)]

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
    thread,
    time::Duration,
};

mod cmdargs;
mod process_utils;

use clap::Parser;
use display_error_chain::DisplayErrorChain;
use libc::pid_t;
use process_utils::{find_processes, launch_process, ProcessInfo};
use snafu::{OptionExt, ResultExt, Snafu};

#[derive(Debug, Snafu)]
enum CheckError {
    CheckFindProcesses {
        source: process_utils::FindProcessError,
    },
    CheckLaunchProcess {
        source: process_utils::LaunchError,
    },
}

fn check_process<I, S>(
    program_name: &str,
    pid: pid_t,
    cmd: &str,
    args: I,
    env: HashMap<OsString, OsString>,
) -> Result<(), CheckError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    thread::sleep(Duration::from_secs(5));
    log::trace!["Checking if the process is running after the launch."];
    let processes = find_processes(program_name).context(CheckFindProcessesSnafu)?;
    if processes
        .into_iter()
        .any(|proc_info| proc_info.pid() == pid)
    {
        log::trace!["Ok, the process is running."];
    } else {
        log::warn!["Recently launched process not found :( Let's try to start it again."];
        launch_process(cmd, args, env).context(CheckLaunchProcessSnafu)?;
    }
    Ok(())
}

#[derive(Debug, Snafu)]
enum RunError {
    #[snafu(display("Can't initialize the logs"))]
    InitLogs {
        path: PathBuf,
        source: anyhow::Error,
    },

    #[snafu(display("Can't find a process"))]
    FindProcess {
        source: process_utils::FindProcessError,
    },

    #[snafu(display("Multiple processes found: {}", DisplayProcesses(processes)))]
    MultipleFound { processes: Vec<ProcessInfo> },

    #[snafu(display("Process not found"))]
    ProcessNotFound,

    #[snafu(display("Unable to restart the process"))]
    Restart { source: process_utils::RestartError },

    #[snafu(display("Restart check has failed"))]
    Check { source: CheckError },
}

fn run() -> Result<(), RunError> {
    let cmdargs::Args {
        name: program_name,
        threshold,
        timeout,
        log_config,
        check: should_check_process,
        command: cmd,
        args: cmd_args,
    } = cmdargs::Args::parse();

    log4rs::init_file(&log_config, Default::default())
        .context(InitLogsSnafu { path: log_config })?;

    let processes = find_processes(&program_name).context(FindProcessSnafu)?;
    if processes.len() > 1 {
        return MultipleFoundSnafu { processes }.fail();
    }

    let process = processes.into_iter().next().context(ProcessNotFoundSnafu)?;

    let memory = process.rss();
    log::info!["Memory: {} kilobytes", memory as f64 / 1024.];
    if memory > threshold {
        log::warn!["Threshold exceeded: {} > {}", memory, threshold];
        let env = process.env().clone();
        let pid = process
            .restart_process(timeout, &cmd, &cmd_args)
            .context(RestartSnafu)?;

        if should_check_process {
            check_process(&program_name, pid, &cmd, cmd_args, env).context(CheckSnafu)?;
        }
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e @ RunError::InitLogs { .. }) => {
            eprintln![
                "Program terminated with an error: {}",
                DisplayErrorChain::new(&e)
            ];
            std::process::exit(1)
        }
        Err(e) => {
            log::error![
                "Program terminated with an error: {}",
                DisplayErrorChain::new(&e)
            ];
            std::process::exit(1)
        }
    }
}

struct DisplayProcesses<'a>(&'a [ProcessInfo]);
impl std::fmt::Display for DisplayProcesses<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut processes = self.0.iter();
        write!(f, "[")?;
        if let Some(process) = processes.next() {
            write!(f, "#{} ({})", process.pid(), process.command())?;
        }
        for process in processes {
            write!(f, ", #{} ({})", process.pid(), process.command())?;
        }
        write!(f, "]")
    }
}
