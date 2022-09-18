use std::{path::PathBuf, time::Duration};

use clap::Parser;

/// Process memory watcher.
///
/// Kills a process when it exceeds the given memory threshold.
#[derive(Debug, Parser)]
pub struct Args {
    /// Program name (as in `comm` in `/proc/[pid]/stat`, man 5 proc).
    #[clap(short, long)]
    pub name: String,

    /// 'Resident Set Size' limit (in bytes, not in pages!).
    pub threshold: u64,

    /// When a SIGKILL signal is sent wait for the specified timeout for the
    /// process to terminate.
    #[clap(long, short, value_parser = humantime::parse_duration, default_value = "60s")]
    pub timeout: Duration,

    /// Path to the logs configuration.
    #[clap(long, default_value = "log4rs.yml")]
    pub log_config: PathBuf,

    /// Checks if the process has been relaunched.
    #[clap(long)]
    pub check: bool,

    /// Command to launch.
    #[clap(long, short)]
    pub command: String,

    /// Command arguments.
    pub args: Vec<String>,
}
