use super::{FormatterFailure, RunLimits};
use command_group::{CommandGroup, GroupChild};
use rustix::{
    fs::{OFlags, fcntl_getfl, fcntl_setfl},
    process::{Pid, test_kill_process_group},
};
use std::{
    io::{self, Read, Write},
    os::fd::AsFd,
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static PENDING: Mutex<Vec<Retained>> = Mutex::new(Vec::new());
struct Slot;
impl Drop for Slot {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Retained {
    child: GroupChild,
    _slot: Slot,
}

fn reserve() -> Result<Slot, FormatterFailure> {
    let mut pending = PENDING.lock().unwrap_or_else(|error| error.into_inner());
    pending.retain_mut(|retained| {
        let reaped = matches!(retained.child.try_wait(), Ok(Some(_)));
        let gone = Pid::from_raw(retained.child.id() as i32)
            .is_some_and(|pid| test_kill_process_group(pid) == Err(rustix::io::Errno::SRCH));
        !reaped || !gone
    });
    if !pending.is_empty() {
        return Err(FormatterFailure::Cleanup(
            "previous formatter cleanup is pending; execution disabled".into(),
        ));
    }
    ACTIVE
        .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            (count < 64).then_some(count + 1)
        })
        .map_err(|_| FormatterFailure::Skipped("64 formatter groups are already active".into()))?;
    Ok(Slot)
}

fn nonblocking(stream: &impl AsFd) -> io::Result<()> {
    fcntl_setfl(stream, fcntl_getfl(stream)? | OFlags::NONBLOCK)?;
    Ok(())
}

fn pump(
    stream: &mut impl Read,
    output: &mut Vec<u8>,
    limit: usize,
    eof: &mut bool,
) -> Result<bool, FormatterFailure> {
    if *eof {
        return Ok(false);
    }
    let mut buffer = [0u8; 8192];
    match stream.read(&mut buffer) {
        Ok(0) => {
            *eof = true;
            Ok(true)
        }
        Ok(count) => {
            if count > limit.saturating_sub(output.len()) {
                return Err(FormatterFailure::OutputLimit);
            }
            output.extend_from_slice(&buffer[..count]);
            Ok(true)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(FormatterFailure::Failed(error.to_string())),
    }
}

fn terminate(child: &mut GroupChild) -> Result<(), FormatterFailure> {
    match child.kill() {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(rustix::io::Errno::SRCH.raw_os_error()) => {
            Ok(())
        }
        Err(error) => Err(FormatterFailure::Cleanup(format!(
            "cannot terminate formatter group: {error}"
        ))),
    }
}

pub(super) fn run(
    mut command: Command,
    input: &[u8],
    limits: &RunLimits,
) -> Result<Vec<u8>, FormatterFailure> {
    let deadline = Instant::now() + limits.timeout;
    let slot = reserve()?;
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .group_spawn()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                FormatterFailure::Unavailable(format!("{program} is unavailable"))
            } else {
                FormatterFailure::Failed(format!("cannot start {program}: {error}"))
            }
        })?;
    let mut stdin = child.inner().stdin.take();
    let mut stdout = child.inner().stdout.take().unwrap();
    let mut stderr = child.inner().stderr.take().unwrap();
    let setup = nonblocking(stdin.as_ref().unwrap())
        .and_then(|_| nonblocking(&stdout))
        .and_then(|_| nonblocking(&stderr));
    let mut written = 0;
    let mut output = vec![];
    let mut errors = vec![];
    let mut out_eof = false;
    let mut err_eof = false;
    let mut status = None;
    let result = if let Err(error) = setup {
        Err(FormatterFailure::Failed(format!(
            "cannot configure nonblocking formatter pipes: {error}"
        )))
    } else {
        loop {
            if Instant::now() >= deadline {
                break Err(FormatterFailure::Timeout);
            }
            let mut progress = false;
            if written == input.len() {
                stdin.take();
            }
            if let Some(stream) = stdin.as_mut() {
                match stream.write(&input[written..]) {
                    Ok(0) => {
                        break Err(FormatterFailure::Failed(
                            "formatter stdin closed before consuming input".into(),
                        ));
                    }
                    Ok(count) => {
                        written += count;
                        progress = true;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) => {}
                    Err(error) => {
                        break Err(FormatterFailure::Failed(format!(
                            "formatter stdin: {error}"
                        )));
                    }
                }
            }
            match pump(&mut stdout, &mut output, limits.stdout, &mut out_eof) {
                Ok(changed) => progress |= changed,
                Err(error) => break Err(error),
            }
            match pump(&mut stderr, &mut errors, limits.stderr, &mut err_eof) {
                Ok(changed) => progress |= changed,
                Err(error) => break Err(error),
            }
            if status.is_none() {
                match child.try_wait() {
                    Ok(Some(exit)) => {
                        status = Some(exit);
                        if let Err(error) = terminate(&mut child) {
                            break Err(error);
                        }
                        if !exit.success() {
                            break Err(FormatterFailure::Failed(format!(
                                "{program} exited with {exit}"
                            )));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => break Err(FormatterFailure::Failed(error.to_string())),
                }
            }
            if status.is_some() && stdin.is_none() && out_eof && err_eof {
                break Ok(output);
            }
            if !progress {
                std::thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(1)),
                );
            }
        }
    };
    drop(stdin);
    drop(stdout);
    drop(stderr);
    let termination = terminate(&mut child);
    if let Err(error) = termination {
        // Retain bounded ownership for later nonblocking reaping, never a waiter thread.
        PENDING
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(Retained { child, _slot: slot });
        return Err(error);
    }
    let cleanup = Instant::now() + Duration::from_millis(100);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return result,
            Ok(None) => {}
            Err(error) => {
                PENDING
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .push(Retained { child, _slot: slot });
                return Err(FormatterFailure::Cleanup(format!(
                    "cannot reap formatter group: {error}"
                )));
            }
        }
        if Instant::now() >= cleanup {
            PENDING
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(Retained { child, _slot: slot });
            return Err(FormatterFailure::Cleanup(
                "formatter group did not reap within 100ms cleanup budget".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
