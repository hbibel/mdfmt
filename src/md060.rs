mod comments;
mod width;

use crate::{Diagnostic, Source, config::DocumentConfig, parse::ParsedDocument};
use std::{collections::HashSet, ops::Range};

struct Row {
    span: Range<usize>,
    pipes: Vec<usize>,
}

fn row(source: &Source, span: Range<usize>) -> Row {
    let text = &source.text()[span.clone()];
    let start = span.start + text.len() - text.trim_start_matches([' ', '\t']).len();
    let pipes = text
        .bytes()
        .enumerate()
        .filter(|(i, ch)| {
            *ch == b'|'
                && text.as_bytes()[..*i]
                    .iter()
                    .rev()
                    .take_while(|ch| **ch == b'\\')
                    .count()
                    % 2
                    == 0
        })
        .map(|(i, _)| span.start + i)
        .collect();
    Row {
        span: start..span.end,
        pipes,
    }
}

fn aligned(
    source: &Source,
    display: &Source,
    rows: &[Row],
    detail: &'static str,
) -> Vec<(usize, &'static str)> {
    let effective = |pipe: usize| width::width(comments::prefix(source, display, pipe));
    let Some(first) = rows.first() else {
        return vec![];
    };
    let header: HashSet<_> = first.pipes.iter().map(|pipe| effective(*pipe)).collect();
    let mut out = vec![];
    for row in rows.iter().skip(1) {
        let mut remaining = header.clone();
        for pipe in &row.pipes {
            if !remaining.is_empty() && !remaining.remove(&effective(*pipe)) {
                out.push((*pipe, detail));
            }
        }
    }
    out
}

fn padding(source: &Source, rows: &[Row], tight: bool) -> Vec<(usize, &'static str)> {
    let mut out = vec![];
    for row in rows {
        for pipe in &row.pipes {
            let left = &source.text()[row.span.start..*pipe];
            if !left.is_empty() {
                let spaces = left.len() - left.trim_end_matches([' ', '\t']).len();
                if spaces > 0 {
                    if tight {
                        out.push((
                            *pipe,
                            "Table pipe has space to the left for style \"tight\"",
                        ));
                    } else if spaces != 1 {
                        out.push((
                            *pipe,
                            "Table pipe has extra space to the left for style \"compact\"",
                        ));
                    }
                } else if !tight {
                    out.push((
                        *pipe,
                        "Table pipe is missing space to the left for style \"compact\"",
                    ));
                }
            }
            let right = &source.text()[pipe + 1..row.span.end];
            if !right.is_empty() {
                let spaces = right.len() - right.trim_start_matches([' ', '\t']).len();
                if spaces > 0 {
                    if spaces != right.len() {
                        if tight {
                            out.push((
                                *pipe,
                                "Table pipe has space to the right for style \"tight\"",
                            ));
                        } else if spaces != 1 {
                            out.push((
                                *pipe,
                                "Table pipe has extra space to the right for style \"compact\"",
                            ));
                        }
                    }
                } else if !tight {
                    out.push((
                        *pipe,
                        "Table pipe is missing space to the right for style \"compact\"",
                    ));
                }
            }
        }
    }
    out
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !(1..=source.line_count())
        .any(|line| document.state("MD060", line).is_some_and(|s| s.enabled))
    {
        return vec![];
    }
    let mut result = vec![];
    let display = comments::display_source(source, parsed);
    for table in crate::heading_spaces::literal_exclusions(source, parsed, true).tables {
        let Some(header) = table.rows.first() else {
            continue;
        };
        let line = source.position(header.span.start).unwrap().line;
        let Some(settings) = document.state("MD060", line) else {
            continue;
        };
        let style = settings
            .options
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("any");
        let delimiter = settings
            .options
            .get("aligned_delimiter")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let rows: Vec<_> = std::iter::once(header.span.clone())
            .chain(std::iter::once(table.delimiter))
            .chain(table.rows.iter().skip(1).map(|r| r.span.clone()))
            .map(|span| row(source, span))
            .collect();
        let mut best = None;
        if matches!(style, "any" | "aligned") {
            best = Some(aligned(
                source,
                &display,
                &rows,
                "Table pipe does not align with header for style \"aligned\"",
            ));
        }
        if style != "aligned" && !best.as_ref().is_some_and(Vec::is_empty) {
            for tight in [false, true] {
                if style != "any" && style != if tight { "tight" } else { "compact" } {
                    continue;
                }
                let mut findings = if delimiter {
                    aligned(
                        source,
                        &display,
                        &rows[..2],
                        "Table pipe does not align with header for option \"aligned_delimiter\"",
                    )
                } else {
                    vec![]
                };
                findings.extend(padding(source, &rows, tight));
                if best
                    .as_ref()
                    .is_none_or(|prior| findings.len() < prior.len())
                {
                    best = Some(findings);
                }
            }
        }
        for (pipe, detail) in best.unwrap_or_default() {
            let line = source.position(pipe).unwrap().line;
            let Some(state) = document.state("MD060", line).filter(|s| s.enabled) else {
                continue;
            };
            result.push(Diagnostic {
                code: "MD060".into(),
                rule_id: Some("MD060".into()),
                severity: state.severity,
                message: format!("Table column style [{detail}]"),
                span: Some(pipe..pipe + 1),
            });
        }
    }
    result.sort_by_key(|finding| {
        source
            .position(finding.span.as_ref().unwrap().start)
            .unwrap()
            .line
    });
    result
}
