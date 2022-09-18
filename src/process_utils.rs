use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use libc::{self, c_int, pid_t};
use procfs::ProcError;
use snafu::{ResultExt, Snafu};

/// Information on running program.
#[derive(Debug)]
pub struct ProcessInfo {
    /// Process ID.
    pid: pid_t,

    /// 'Resident Set Size' in bytes.
    rss: u64,

    /// Environment variables.
    env: HashMap<OsString, OsString>,

    /// The time the process started after system boot.
    start_time: u64,

    /// The filename of the executable, in parentheses.
    command: String,
}

#[derive(Debug, Snafu)]
pub enum KillError {
    #[snafu(display("Invalid signal detected"))]
    InvalidSignal { signal: c_int },
    #[snafu(display("Permission denied to send a signal {signal} to process #{pid}"))]
    PermissionDenied { signal: c_int, pid: pid_t },
    #[snafu(display("Process #{pid} not found"))]
    NotFound { pid: pid_t },
}

/// Sends a signal to a process.
pub fn send_signal(pid: pid_t, signal: c_int) -> Result<(), KillError> {
    log::trace!["Sending signal {} to process {}", signal, pid];
    match unsafe { libc::kill(pid, signal) } {
        0 => Ok(()),
        -1 => {
            let errno: c_int = unsafe { *libc::__errno_location() };
            match errno {
                libc::EINVAL => InvalidSignalSnafu { signal }.fail(),
                libc::EPERM => PermissionDeniedSnafu { pid, signal }.fail(),
                libc::ESRCH => NotFoundSnafu { pid }.fail(),
                x => unreachable!["Unexpected error value {x}"],
            }
        }
        x => panic!["Unexpected return code {x}"],
    }
}

#[derive(Debug, Snafu)]
pub enum FindProcessError {
    #[snafu(display("Unable to get a list of processes"))]
    GetProcessList { source: ProcError },

    #[snafu(display("Unable to fetch next process info"))]
    GetProcess { source: ProcError },

    #[snafu(display("Can't get environment variables of a process #{pid}"))]
    GetEnv { source: ProcError, pid: pid_t },

    #[snafu(display("Can't get stats of a process #{pid}"))]
    GetStat { source: ProcError, pid: pid_t },

    #[snafu(display("Can't calculate RSS size in bytes of a process #{pid}"))]
    RssBytes { source: ProcError, pid: pid_t },
}

/// Finds running processes with the given command name.
pub fn find_processes(cmd_name: &str) -> Result<Vec<ProcessInfo>, FindProcessError> {
    procfs::process::all_processes()
        .context(GetProcessListSnafu)?
        .map(|process| {
            let process = process.context(GetProcessSnafu)?;
            let pid = process.pid();

            let stat = process.stat().context(GetStatSnafu { pid })?;
            let rss = stat.rss_bytes().context(RssBytesSnafu { pid })?;

            let command = stat.comm;

            if command == cmd_name {
                log::trace!("Found process #{pid}");
                Ok(Some(ProcessInfo {
                    pid,
                    env: process.environ().context(GetEnvSnafu { pid })?,
                    rss,
                    start_time: stat.starttime,
                    command,
                }))
            } else {
                Ok(None)
            }
        })
        .filter_map(Result::transpose)
        .collect()
}

/// A high-level error.
#[derive(Debug, Snafu)]
pub enum WaitStopError {
    #[snafu(display("Sending signal 0 to #{pid}"))]
    SendSignal0 { pid: pid_t, source: KillError },

    #[snafu(display("Can't get process information of #{pid}"))]
    GetProcessInfo { pid: pid_t, source: ProcError },

    #[snafu(display("Can't get process stats of #{pid}"))]
    GetProcessStats { pid: pid_t, source: ProcError },
}

#[derive(Debug, Snafu)]
pub struct LaunchError {
    source: std::io::Error,
}

/// Launches and detaches a process.
///
/// Returns PID of the detached process.
pub fn launch_process<I, S>(
    cmd: &str,
    args: I,
    environment: HashMap<OsString, OsString>,
) -> Result<pid_t, LaunchError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    log::trace!["Launching '{cmd}'"];
    let child = Command::new(cmd)
        .args(args)
        .env_clear()
        .envs(environment)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context(LaunchSnafu)?;

    Ok(child.id() as pid_t)
}

/// An error encountered during a process restart.
#[derive(Debug, Snafu)]
pub enum RestartError {
    /// Unable to terminate process.
    #[snafu(display("Unable to terminate process"))]
    Terminate {
        /// Source error.
        source: KillError,
    },

    /// Wait for a process to stop.
    #[snafu(display("Wait for a process to stop"))]
    WaitStop {
        /// Source error.
        source: WaitStopError,
    },

    /// Re-launch the process.
    #[snafu(display("Re-launch the process"))]
    LaunchProcess {
        /// Source error.
        source: LaunchError,
    },
}

impl ProcessInfo {
    /// Checks whether a given process has stopped.
    fn has_stopped(&self) -> Result<bool, WaitStopError> {
        let pid = self.pid;
        match send_signal(pid, 0) {
            Err(KillError::NotFound { pid }) => {
                log::trace!["Process #{pid} not found"];
                return Ok(true);
            }
            Err(e) => Err(e).context(SendSignal0Snafu { pid })?,
            Ok(_) => {}
        };
        log::trace!["Process found. Let's check if its `start_time` is the same"];

        Ok(procfs::process::Process::new(pid)
            .context(GetProcessInfoSnafu { pid })?
            .stat()
            .context(GetProcessStatsSnafu { pid })?
            .starttime
            != self.start_time)
    }

    /// Waits for a process to stop.
    pub fn wait_stop(&self, timeout: Duration) -> Result<(), WaitStopError> {
        const INTERVAL: Duration = Duration::from_secs(1);

        log::trace!["Waiting for the pid #{} to stop.", self.pid];
        let started = Instant::now();
        loop {
            if self.has_stopped()? {
                log::trace!["Process #{} has stopped.", self.pid];
                break;
            }
            if started.elapsed() > timeout {
                log::trace!["Timeout has been reached, leaving the process as it is."];
                break;
            }
            thread::sleep(INTERVAL);
        }
        Ok(())
    }

    /// Restarts the given process.
    ///
    /// Returns the PID of the detached process.
    pub fn restart_process<I, S>(
        self,
        wait_timeout: Duration,
        cmd: &str,
        args: I,
    ) -> Result<pid_t, RestartError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        send_signal(self.pid, libc::SIGTERM).context(TerminateSnafu)?;
        self.wait_stop(wait_timeout).context(WaitStopSnafu)?;
        launch_process(cmd, args, self.env).context(LaunchProcessSnafu)
    }

    /// PID of the process.
    pub fn pid(&self) -> i32 {
        self.pid
    }

    /// Returns environment of the process.
    pub fn env(&self) -> &HashMap<OsString, OsString> {
        &self.env
    }

    /// Returns the filename of the executable, in parentheses.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// 'Resident Set Size' in bytes.
    pub fn rss(&self) -> u64 {
        self.rss
    }
}
