use super::FormatterFailure;
use std::{process::Command, time::Duration};

#[cfg(unix)]
#[path = "runner_unix.rs"]
mod unix;

#[derive(Clone, Copy)]
pub(crate) struct RunLimits {
    pub timeout: Duration,
    pub stdout: usize,
    pub stderr: usize,
}

pub(crate) fn run(
    command: Command,
    input: &[u8],
    limits: &RunLimits,
) -> Result<Vec<u8>, FormatterFailure> {
    if limits.timeout.is_zero() {
        return Err(FormatterFailure::Timeout);
    }
    if limits.stdout == 0 || limits.stderr == 0 {
        return Err(FormatterFailure::OutputLimit);
    }
    if input.len() > 1024 * 1024 {
        return Err(FormatterFailure::Skipped("code body exceeds 1 MiB".into()));
    }
    #[cfg(unix)]
    {
        unix::run(command, input, limits)
    }
    #[cfg(not(unix))]
    {
        let _ = command;
        Err(FormatterFailure::Skipped(
            "bounded nonblocking formatter transport is not available on this platform".into(),
        ))
    }
}
