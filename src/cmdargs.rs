use super::clap;
use clap::{App, Arg};

pub fn build_cli() -> clap::App<'static, 'static> {
    App::new("Process memory watcher")
        .about("Kills a process when it exceeds a given threshold.")
        .version("0.1")
        .arg(
            Arg::with_name("NAME")
                .long("--name")
                .short("-n")
                .value_name("name")
                .takes_value(true)
                .required(true)
                .help(
                    "Program name (as in `comm` in `/proc/[pid]/stat`, man 5 proc).",
                ),
        )
        .arg(
            Arg::with_name("THRESHOLD")
                .long("--threshold")
                .short("-t")
                .value_name("bytes")
                .takes_value(true)
                .required(true)
                .help(
                    "'Resident Set Size' limit (multiplied by a page size in bytes).",
                ),
        )
        .arg(
            Arg::with_name("TIMEOUT")
                .long("--timeout")
                .value_name("seconds")
                .takes_value(true)
                .required(false)
                .default_value("60")
                .help(
                    "When a SIGKILL signal is sent wait for the specified timeout for the process \
                    to terminate.",
                ),
        )
        .arg(
            Arg::with_name("LOG_CONFIG")
                .long("--log-config")
                .value_name("path")
                .takes_value(true)
                .required(false)
                .default_value("log4rs.yml")
                .help("Logs configuration."),
        )
        .arg(
            Arg::with_name("CHECK")
                .long("--check")
                .required(false)
                .help("Check if the process has been relaunched."),
        )
        .arg(
            Arg::with_name("CMD")
                .long("--command")
                .short("-c")
                .value_name("command")
                .takes_value(true)
                .required(true)
                .help("Command to launch."),
        )
        .arg(
            Arg::with_name("ARGS")
                .takes_value(true)
                .multiple(true)
                .help("Command arguments."),
        )
}
