use crate::file_io::{FileIdentity, FileSnapshot, read_input, write_if_changed};
use clap::Parser;
use mdfmt::{FormatOptions, FormatResult, Severity, Source};
use std::{
    collections::HashSet,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    version,
    about = "Source-preserving Markdown foundation (no rule fixes yet)",
    after_help = "Only literal files are supported. Configuration loading and rule fixes are not implemented. Successful processing does not imply markdownlint compliance."
)]
struct Args {
    #[arg(long, conflicts_with = "check", help = "Update files in place")]
    write: bool,
    #[arg(long, help = "Check without writing or printing document content")]
    check: bool,
    #[arg(
        required = true,
        value_name = "FILE",
        help = "Literal file path, or - for stdin"
    )]
    files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Stdout,
    Write,
    Check,
}

#[derive(Default)]
struct SeenInputs {
    identities: HashSet<FileIdentity>,
    paths: HashSet<PathBuf>,
}

impl SeenInputs {
    fn first_visit(&mut self, snapshot: &FileSnapshot) -> io::Result<bool> {
        let path = snapshot.path.canonicalize()?;
        let new_identity = self.identities.insert(snapshot.identity.clone());
        let new_path = self.paths.insert(path);
        Ok(new_identity && new_path)
    }
}

fn document_status(mode: Mode, original: &str, result: &FormatResult) -> u8 {
    u8::from(
        (mode == Mode::Check && result.changed(original))
            || result
                .diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
    )
}

fn process_document(
    label: &Path,
    original: &str,
    mode: Mode,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<(String, u8)> {
    let result = mdfmt::format(original, &FormatOptions::default()).map_err(io::Error::other)?;
    let source = Source::new(result.output.clone());
    for diagnostic in &result.diagnostics {
        let position = diagnostic
            .span
            .as_ref()
            .and_then(|span| source.position(span.start).ok());
        let location = position
            .map(|p| format!(":{}:{}", p.line, p.column_utf16))
            .unwrap_or_default();
        writeln!(
            stderr,
            "{}{location}: {:?} {}: {}",
            label.display(),
            diagnostic.severity,
            diagnostic.code,
            diagnostic.message
        )?;
    }
    let status = document_status(mode, original, &result);
    if mode == Mode::Stdout {
        stdout.write_all(result.output.as_bytes())?;
    }
    if mode == Mode::Check && result.changed(original) {
        writeln!(stderr, "{}: formatting changes needed", label.display())?;
    }
    Ok((result.output, status))
}

pub(crate) fn run() -> u8 {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let status = if error.use_stderr() { 2 } else { 0 };
            return if error.print().is_ok() { status } else { 2 };
        }
    };
    let mode = if args.write {
        Mode::Write
    } else if args.check {
        Mode::Check
    } else {
        Mode::Stdout
    };
    let stdin_count = args.files.iter().filter(|p| p.as_os_str() == "-").count();
    if (stdin_count > 0 && (args.files.len() != 1 || mode == Mode::Write))
        || (mode == Mode::Stdout && args.files.len() != 1)
    {
        let _ = writeln!(
            io::stderr().lock(),
            "mdfmt: stdout requires one input; stdin cannot be mixed with files or used with --write"
        );
        return 2;
    }
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let mut seen = SeenInputs::default();
    let mut status = 0;
    for path in args.files {
        let outcome = (|| -> io::Result<u8> {
            if path.as_os_str() == "-" {
                let mut text = String::new();
                io::stdin().lock().read_to_string(&mut text)?;
                return process_document(&path, &text, mode, &mut stdout, &mut stderr)
                    .map(|(_, code)| code);
            }
            let snapshot = read_input(&path)?;
            if mode == Mode::Write {
                // Validate each spelling before deduplication, including symlink aliases.
                write_if_changed(&snapshot, &snapshot.text)?;
            }
            if !seen.first_visit(&snapshot)? {
                return Ok(0);
            }
            let (output, code) =
                process_document(&path, &snapshot.text, mode, &mut stdout, &mut stderr)?;
            if mode == Mode::Write {
                write_if_changed(&snapshot, &output)?;
            }
            Ok(code)
        })();
        match outcome {
            Ok(code) => status = status.max(code),
            Err(error) => {
                let _ = writeln!(stderr, "{}: {error}", path.display());
                status = 2;
            }
        }
    }
    if stdout.flush().is_err() || stderr.flush().is_err() {
        return 2;
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdfmt::{Coverage, Diagnostic};

    #[test]
    fn input_deduplication_survives_our_own_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "original").unwrap();
        let snapshot = read_input(&path).unwrap();
        let mut seen = SeenInputs::default();
        assert!(seen.first_visit(&snapshot).unwrap());
        assert!(write_if_changed(&snapshot, "changed").unwrap());
        let replaced = read_input(&path).unwrap();
        assert!(!seen.first_visit(&replaced).unwrap());
    }

    #[test]
    fn status_respects_changes_severity_and_fix_permission() {
        let mut result = FormatResult {
            output: "changed".into(),
            edits: vec![],
            diagnostics: vec![],
            coverage: Coverage {
                implemented_rules: vec![],
                complete: false,
            },
        };
        assert_eq!(document_status(Mode::Check, "original", &result), 1);
        assert_eq!(document_status(Mode::Write, "original", &result), 0);
        result.output = "original".into();
        result.diagnostics.push(Diagnostic {
            code: "test".into(),
            severity: Severity::Warning,
            message: "warning".into(),
            rule_id: None,
            span: None,
        });
        assert_eq!(document_status(Mode::Check, "original", &result), 0);
        result.diagnostics[0].severity = Severity::Error;
        assert_eq!(document_status(Mode::Stdout, "original", &result), 1);
        assert_eq!(document_status(Mode::Check, "original", &result), 1);
        let disabled = mdfmt::format("original", &FormatOptions { fix: false }).unwrap();
        assert_eq!(document_status(Mode::Check, "original", &disabled), 0);
    }
}
