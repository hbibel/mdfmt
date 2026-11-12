use super::FormatterFailure;
use command_group::CommandGroup;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub(crate) struct RunLimits {
    pub timeout: Duration,
    pub stdout: usize,
    pub stderr: usize,
}
enum Completion {
    Input(Result<(), FormatterFailure>),
    Output(Result<Vec<u8>, FormatterFailure>),
    Error(Result<Vec<u8>, FormatterFailure>),
}

fn read(stream: impl Read, limit: usize) -> Result<Vec<u8>, FormatterFailure> {
    let mut output = Vec::new();
    stream
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut output)
        .map_err(|error| FormatterFailure::Failed(error.to_string()))?;
    if output.len() > limit {
        Err(FormatterFailure::OutputLimit)
    } else {
        Ok(output)
    }
}

pub(crate) fn run(
    mut command: Command,
    input: &[u8],
    limits: &RunLimits,
) -> Result<Vec<u8>, FormatterFailure> {
    if limits.timeout.is_zero() {
        return Err(FormatterFailure::Timeout);
    }
    if input.len() > 1024 * 1024 {
        return Err(FormatterFailure::Skipped("code body exceeds 1 MiB".into()));
    }
    let deadline = Instant::now() + limits.timeout;
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .group_spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FormatterFailure::Unavailable(format!("{program} is unavailable"))
            } else {
                FormatterFailure::Failed(format!("cannot start {program}: {error}"))
            }
        })?;
    let mut stdin = child.inner().stdin.take().unwrap();
    let stdout = child.inner().stdout.take().unwrap();
    let stderr = child.inner().stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let input = input.to_vec();
    let sent = sender.clone();
    let writer = std::thread::spawn(move || {
        let result = stdin
            .write_all(&input)
            .map_err(|error| FormatterFailure::Failed(format!("formatter stdin: {error}")));
        drop(stdin);
        let _ = sent.send(Completion::Input(result));
    });
    let sent = sender.clone();
    let limit = limits.stdout;
    let reader = std::thread::spawn(move || {
        let _ = sent.send(Completion::Output(read(stdout, limit)));
    });
    let limit = limits.stderr;
    let errors = std::thread::spawn(move || {
        let _ = sender.send(Completion::Error(read(stderr, limit)));
    });
    let mut input_done = false;
    let mut output = None;
    let mut errors_done = false;
    let mut status = None;
    let result = loop {
        if Instant::now() >= deadline {
            break Err(FormatterFailure::Timeout);
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(Some(exit)) => {
                    status = Some(exit);
                    // A successful leader may leave descendants holding its pipes.
                    let _ = child.kill();
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
        if status.is_some() && input_done && errors_done && output.is_some() {
            break Ok(output.take().unwrap());
        }
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(5));
        match receiver.recv_timeout(remaining) {
            Ok(Completion::Input(Ok(()))) => input_done = true,
            Ok(Completion::Output(Ok(bytes))) => output = Some(bytes),
            Ok(Completion::Error(Ok(_))) => errors_done = true,
            Ok(
                Completion::Input(Err(error))
                | Completion::Output(Err(error))
                | Completion::Error(Err(error)),
            ) => break Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                if !input_done || !errors_done || output.is_none() {
                    break Err(FormatterFailure::Failed(
                        "formatter I/O worker stopped".into(),
                    ));
                }
                std::thread::sleep(remaining);
            }
        }
    };
    let _ = child.kill();
    let waited = child.wait();
    let joined = writer.join().is_ok() & reader.join().is_ok() & errors.join().is_ok();
    if !joined {
        return Err(FormatterFailure::Failed(
            "formatter I/O worker panicked".into(),
        ));
    }
    waited.map_err(|error| FormatterFailure::Failed(error.to_string()))?;
    result
}
