use crate::file_io::{FileIdentity, FileSnapshot, read_input, write_if_changed};
use clap::Parser;
use mdfmt::config::{ConfigResolver, ResolvedConfig, select_inputs};
use mdfmt::{FormatOptions, FormatResult, Severity, Source};
use std::{
    collections::HashSet,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    version,
    about = "Native Markdown formatter with configuration-aware MD013 reflow, conditional MD003 heading styles and safe rule fixes",
    after_help = "MD013 prose reflow, safe MD004 list markers, conditional MD009 trailing-whitespace and MD010 hard-tab repairs, MD011 reversed-link diagnostics, safe MD012 blank-line repairs, MD014 protected-code prompt diagnostics, MD018 missing-ATX-separator diagnostics, MD019/MD021 heading separators, MD020 closed-ATX-separator diagnostics, conditional MD023 heading indentation, MD024 duplicate-heading diagnostics, MD026 heading-punctuation fixes, conditional MD027 quote spacing, MD028 blockquote gap diagnostics, conditional MD029 ordered numbering and MD030 list-marker spacing, safe MD035 thematic-break styles, MD040 fenced-language, MD042 empty-link and MD045 missing-alt-text diagnostics, safe MD047 final newlines, conditional MD048 fence styles, MD055 table-pipe-style and MD056 table-column diagnostics, conditional MD058 table separators, MD001/MD025/MD041/MD043 heading-structure diagnostics, MD036 emphasis-as-heading and MD059 descriptive-link diagnostics and remaining-violation reporting are supported, including lists and blockquotes. Unsafe syntax is preserved except for approved MD026 suffix edits. Other rule fixes and findings are not implemented; successful processing does not imply full markdownlint compliance."
)]
struct Args {
    #[arg(long, conflicts_with = "check", help = "Update files in place")]
    write: bool,
    #[arg(long, help = "Check without writing or printing document content")]
    check: bool,
    #[arg(
        value_name = "PATTERN",
        help = "File/glob, :literal path, or - for stdin"
    )]
    files: Vec<String>,
    #[arg(long, help = "Declarative base configuration; discovery still applies")]
    config: Option<PathBuf>,
    #[arg(long, alias = "configPointer", requires = "config")]
    config_pointer: Option<String>,
    #[arg(long, help = "Do not append configuration globs")]
    no_globs: bool,
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
    config: &ResolvedConfig,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<(String, u8)> {
    let result = mdfmt::format_with_config(original, &FormatOptions::default(), config)
        .map_err(io::Error::other)?;
    let source = Source::new(result.output.clone());
    for diagnostic in &result.diagnostics {
        let position = diagnostic
            .span
            .as_ref()
            .and_then(|span| source.position(span.start).ok());
        let location = position
            .map(|p| {
                let bom = usize::from(p.line == 1 && result.output.starts_with('\u{feff}'));
                format!(":{}:{}", p.line, p.column_utf16 - bom)
            })
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
    let mut args = match Args::try_parse() {
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
    let raw: Vec<_> = std::env::args_os().collect();
    if let Some(index) = raw.iter().position(|a| a == "--") {
        let count = raw.len() - index - 1;
        let start = args.files.len().saturating_sub(count);
        for path in &mut args.files[start..] {
            *path = format!(":{path}");
        }
    }
    let resolver = match std::env::current_dir()
        .map_err(io::Error::other)
        .and_then(|directory| {
            ConfigResolver::new(
                &directory,
                args.config.as_deref(),
                args.config_pointer.as_deref(),
            )
            .map_err(io::Error::other)
        }) {
        Ok(resolver) => resolver,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "mdfmt: {error}");
            return 2;
        }
    };
    if args.files.is_empty()
        && (args.no_globs || resolver.base_options().strings("globs").is_empty())
    {
        let _ = writeln!(
            io::stderr().lock(),
            "mdfmt: provide input patterns, stdin -, or configuration globs"
        );
        return 2;
    }
    let stdin_count = args.files.iter().filter(|p| p.as_str() == "-").count();
    let selection = match select_inputs(&resolver, &args.files, args.no_globs) {
        Ok(selection) => selection,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "mdfmt: {error}");
            return 2;
        }
    };
    if stdin_count > 1
        || (selection.stdin && (!selection.files.is_empty() || mode == Mode::Write))
        || (mode == Mode::Stdout && selection.files.len() + usize::from(selection.stdin) != 1)
    {
        let _ = writeln!(
            io::stderr().lock(),
            "mdfmt: stdout requires one input; stdin cannot be mixed with files or used with --write"
        );
        return 2;
    }
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    if resolver.base_options().boolean("showFound")
        && !resolver.base_options().boolean("noProgress")
    {
        let mut names: Vec<_> = selection
            .files
            .iter()
            .map(|p| {
                p.strip_prefix(resolver.invocation())
                    .unwrap_or(p)
                    .display()
                    .to_string()
            })
            .collect();
        if selection.stdin {
            names.push("stdin".into());
        }
        names.sort();
        if writeln!(stderr, "Found:\n {}", names.join("\n ")).is_err() {
            return 2;
        }
    }
    let mut seen = SeenInputs::default();
    let mut status = 0;
    let mut paths = selection.files;
    if selection.stdin {
        paths.push("-".into());
    }
    for path in paths {
        let outcome = (|| -> io::Result<u8> {
            if path.as_os_str() == "-" {
                let mut text = String::new();
                io::stdin().lock().read_to_string(&mut text)?;
                let config = resolver
                    .resolve(&resolver.invocation().join("stdin"))
                    .map_err(io::Error::other)?;
                return process_document(&path, &text, mode, &config, &mut stdout, &mut stderr)
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
            let (output, code) = process_document(
                &path,
                &snapshot.text,
                mode,
                &resolver.resolve(&path).map_err(io::Error::other)?,
                &mut stdout,
                &mut stderr,
            )?;
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
        let disabled = mdfmt::format("# Original\n", &FormatOptions { fix: false }).unwrap();
        assert_eq!(document_status(Mode::Check, "# Original\n", &disabled), 0);
    }
}
