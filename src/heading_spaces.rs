use crate::{
    Diagnostic, Severity, Source,
    config::DocumentConfig,
    edit::Edit,
    parse::{ParsedDocument, RegionKind},
};
use std::ops::Range;

pub(crate) fn excluded_flow_lines(source: &Source, parsed: &ParsedDocument) -> Vec<bool> {
    literal_exclusions(source, parsed, false).flow
}

pub(crate) struct LiteralExclusions {
    pub flow: Vec<bool>,
    pub inline: Vec<Range<usize>>,
    pub code: Vec<CodeBlock>,
    pub tables: Vec<TableBlock>,
}

pub(crate) struct CodeBlock {
    pub span: Range<usize>,
    pub value: String,
}

pub(crate) struct TableBlock {
    pub span: Range<usize>,
    pub rows: Vec<TableRow>,
    pub delimiter: Range<usize>,
}
pub(crate) struct TableRow {
    pub span: Range<usize>,
    pub cells: Vec<Range<usize>>,
}

pub(crate) fn literal_exclusions(
    source: &Source,
    parsed: &ParsedDocument,
    inline_math: bool,
) -> LiteralExclusions {
    fn collect(source: &Source, parsed: &ParsedDocument) -> LiteralExclusions {
        let mut ignored = vec![false; source.line_count()];
        let mut inline: Vec<Range<usize>> = vec![];
        let mut code = vec![];
        let mut tables = vec![];
        fn snapshots(
            node: &markdown::mdast::Node,
            base: usize,
            output: &mut Vec<CodeBlock>,
            tables: &mut Vec<TableBlock>,
        ) {
            if let markdown::mdast::Node::Code(code) = node
                && let Some(position) = &code.position
            {
                output.push(CodeBlock {
                    span: base + position.start.offset..base + position.end.offset,
                    value: code.value.clone(),
                });
            }
            if let markdown::mdast::Node::Table(table) = node
                && let Some(position) = &table.position
            {
                tables.push(TableBlock {
                    span: base + position.start.offset..base + position.end.offset,
                    delimiter: 0..0,
                    rows: table
                        .children
                        .iter()
                        .filter_map(|row| {
                            let position = row.position()?;
                            Some(TableRow {
                                span: base + position.start.offset..base + position.end.offset,
                                cells: row
                                    .children()?
                                    .iter()
                                    .filter_map(|cell| {
                                        let position = cell.position()?;
                                        Some(
                                            base + position.start.offset
                                                ..base + position.end.offset,
                                        )
                                    })
                                    .collect(),
                            })
                        })
                        .collect(),
                });
            }
            if let Some(children) = node.children() {
                for child in children {
                    snapshots(child, base, output, tables);
                }
            }
        }
        let base = parsed
            .regions
            .iter()
            .find(|region| region.kind == RegionKind::Root && region.parent.is_some())
            .map_or(0, |region| region.span.start);
        snapshots(&parsed.tree, base, &mut code, &mut tables);
        fn delimiters(source: &Source, parsed: &ParsedDocument, tables: &mut [TableBlock]) {
            for table in tables {
                let Some(header) = table.rows.first() else {
                    continue;
                };
                let line = source.position(header.span.start).unwrap().line + 1;
                let Some(range) = source.line_range(line) else {
                    continue;
                };
                let prefix = parsed
                    .regions
                    .iter()
                    .position(|region| {
                        region.kind == RegionKind::Table && region.span == table.span
                    })
                    .and_then(|index| {
                        crate::parse::ContainerPath::for_region(source, parsed, index)
                            .content_start(&source.text()[range.clone()])
                    })
                    .unwrap_or(0);
                table.delimiter = range.start + prefix..range.end;
            }
        }
        delimiters(source, parsed, &mut tables);
        // A lazy paragraph in a finished container can leave native table state
        // set across a blank separator; independent root paragraphs must reset it.
        for region in &parsed.regions {
            let first = source.position(region.span.start).unwrap().line;
            if region.kind == RegionKind::Paragraph
                && region
                    .parent
                    .is_some_and(|parent| parsed.regions[parent].kind == RegionKind::Root)
                && first > 1
                && source.text()[source.line_range(first - 1).unwrap()]
                    .trim_matches([' ', '\t'])
                    .is_empty()
                && source.text()[region.span.clone()].contains('|')
            {
                let isolated = Source::new(format!("\n{}", &source.text()[region.span.clone()]));
                if let Ok(parsed) = crate::parse::parse_flow_diagnostics(&isolated) {
                    let mut found = vec![];
                    snapshots(&parsed.tree, 0, &mut vec![], &mut found);
                    delimiters(&isolated, &parsed, &mut found);
                    for mut table in found {
                        let shift = |span: Range<usize>| {
                            region.span.start + span.start - 1..region.span.start + span.end - 1
                        };
                        table.span = shift(table.span);
                        table.delimiter = shift(table.delimiter);
                        for row in &mut table.rows {
                            row.span = shift(row.span.clone());
                            for cell in &mut row.cells {
                                *cell = shift(cell.clone());
                            }
                        }
                        tables.push(table);
                    }
                }
            }
        }
        let mut siblings = std::collections::HashMap::new();
        for (index, region) in parsed.regions.iter().enumerate() {
            let previous = region
                .parent
                .and_then(|parent| siblings.insert(parent, index));
            let html_flow = region.kind == RegionKind::Html
                && region.parent.is_some_and(|parent| {
                    matches!(
                        parsed.regions[parent].kind,
                        RegionKind::Root | RegionKind::Blockquote | RegionKind::ListItem
                    )
                });
            let first = source.position(region.span.start).unwrap().line;
            let after_container = html_flow
                && previous.is_some_and(|previous| {
                    let previous = &parsed.regions[previous];
                    matches!(previous.kind, RegionKind::List | RegionKind::Blockquote)
                        && source
                            .position(
                                previous.span.start
                                    + source.text()[previous.span.clone()]
                                        .trim_end_matches([' ', '\t', '\r', '\n'])
                                        .len(),
                            )
                            .unwrap()
                            .line
                            + 1
                            == first
                });
            if after_container {
                let container = &parsed.regions[previous.unwrap()];
                let last = container.span.start
                    + source.text()[container.span.clone()]
                        .trim_end_matches([' ', '\t', '\r', '\n'])
                        .len();
                let lazy = parsed.regions.iter().any(|region| {
                    region.kind == RegionKind::Paragraph
                        && region.span.start >= container.span.start
                        && region.span.start < last
                        && last <= region.span.end
                });
                let probe = Source::new(format!("x\n{}", &source.text()[region.span.clone()]));
                let generic = lazy
                    && crate::parse::parse(&probe).ok().is_some_and(|parsed| {
                        parsed.regions.iter().any(|region| {
                            region.kind == RegionKind::Paragraph
                                && probe.position(region.span.start).unwrap().line == 1
                                && probe.position(region.span.end).unwrap().line > 1
                        })
                    });
                if generic {
                    let active: Vec<_> = parsed
                        .regions
                        .iter()
                        .enumerate()
                        .filter(|(_, region)| {
                            matches!(region.kind, RegionKind::ListItem | RegionKind::Blockquote)
                                && region.span.start >= container.span.start
                                && region.span.start < last
                                && last <= region.span.end
                        })
                        .map(|(index, _)| index)
                        .collect();
                    let path = crate::parse::ContainerPath::new(source, parsed, active);
                    let boundary = (first + 1..=source.line_count()).find_map(|line| {
                        let raw = &source.text()[source.line_range(line).unwrap()];
                        if raw.trim_matches([' ', '\t']).is_empty() {
                            return Some((line, None));
                        }
                        path.first_missing(raw).map(|index| (line, Some(index)))
                    });
                    let Some((boundary, failed)) = boundary else {
                        ignored[first - 1..].fill(true);
                        code.retain(|block| block.span.start < region.span.start);
                        tables.retain(|table| table.span.start < region.span.start);
                        inline.retain(|span| span.start < region.span.start);
                        return LiteralExclusions {
                            flow: ignored,
                            inline,
                            code,
                            tables,
                        };
                    };
                    ignored[first - 1..boundary - 1].fill(true);
                    let next = source.line_range(boundary).unwrap().start;
                    let blank = source.text()[source.line_range(boundary).unwrap()]
                        .trim_matches([' ', '\t'])
                        .is_empty();
                    // Keep the old container across blank separators, but restart
                    // unindented root flow without bounding a recovered fence at HTML's end.
                    let prefix = if blank {
                        format!("{}\n", &source.text()[container.span.clone()])
                    } else if let Some(owner) = failed
                        .map(|index| &parsed.regions[index])
                        .filter(|owner| owner.span.start > container.span.start)
                    {
                        format!(
                            "{}\n",
                            source.text()[container.span.start..owner.span.start]
                                .trim_end_matches([' ', '\t'])
                        )
                    } else {
                        "\n".into()
                    };
                    let prefix_lines = Source::new(prefix.clone()).line_count() - 1;
                    let suffix = Source::new(format!("{}{}", prefix, &source.text()[next..]));
                    if let Ok(parsed) = crate::parse::parse_flow_diagnostics(&suffix) {
                        let recovered = collect(&suffix, &parsed);
                        inline.retain(|span| span.start < next);
                        code.retain(|block| block.span.start < next);
                        tables.retain(|table| table.span.start < next);
                        inline.extend(
                            recovered
                                .inline
                                .into_iter()
                                .filter(|span| span.start >= prefix.len())
                                .map(|span| {
                                    next + span.start - prefix.len()..next + span.end - prefix.len()
                                }),
                        );
                        code.extend(
                            recovered
                                .code
                                .into_iter()
                                .filter(|block| block.span.start >= prefix.len())
                                .map(|mut block| {
                                    block.span = next + block.span.start - prefix.len()
                                        ..next + block.span.end - prefix.len();
                                    block
                                }),
                        );
                        tables.extend(
                            recovered
                                .tables
                                .into_iter()
                                .filter(|table| table.span.start >= prefix.len())
                                .map(|mut table| {
                                    let shift = |span: Range<usize>| {
                                        next + span.start - prefix.len()
                                            ..next + span.end - prefix.len()
                                    };
                                    table.span = shift(table.span);
                                    table.delimiter = shift(table.delimiter);
                                    for row in &mut table.rows {
                                        row.span = shift(row.span.clone());
                                        for cell in &mut row.cells {
                                            *cell = shift(cell.clone());
                                        }
                                    }
                                    table
                                }),
                        );
                        for (line, excluded) in
                            recovered.flow.into_iter().skip(prefix_lines).enumerate()
                        {
                            if boundary - 1 + line < ignored.len() {
                                ignored[boundary - 1 + line] = excluded;
                            }
                        }
                    }
                    return LiteralExclusions {
                        flow: ignored,
                        inline,
                        code,
                        tables,
                    };
                }
            }
            if matches!(region.kind, RegionKind::Code | RegionKind::FrontMatter) || html_flow {
                for line in crate::md013::covered_lines(source, region.span.clone()) {
                    ignored[line] = true;
                }
            }
            if region.kind == RegionKind::InlineCode {
                inline.push(region.span.clone());
            }
        }
        LiteralExclusions {
            flow: ignored,
            inline,
            code,
            tables,
        }
    }
    if source.text().contains(if inline_math { "$" } else { "$$" }) || source.text().contains('|') {
        crate::parse::parse_flow_diagnostics(source)
            .map(|parsed| collect(source, &parsed))
            .unwrap_or_else(|_| collect(source, parsed))
    } else {
        collect(source, parsed)
    }
}

fn findings(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<(Range<usize>, &'static str, Severity)> {
    let mut result = vec![];
    for region in &parsed.regions {
        if region.kind != RegionKind::Heading {
            continue;
        }
        let raw = source.text()[region.span.clone()].trim_start_matches([' ', '\t']);
        if !raw.starts_with('#') || raw.contains(['\r', '\n']) {
            continue;
        }
        let base = region.span.end - raw.len();
        let opening_end = raw.bytes().take_while(|&byte| byte == b'#').count();
        let opening_gap_end = raw.len() - raw[opening_end..].trim_start_matches([' ', '\t']).len();
        let trimmed = raw.trim_end_matches([' ', '\t']);
        let closing_start = trimmed.trim_end_matches('#').len();
        let closed = closing_start < trimmed.len()
            && closing_start >= opening_end
            && raw[..closing_start].ends_with([' ', '\t']);
        let rule = if closed { "MD021" } else { "MD019" };
        let line = source.position(region.span.start).unwrap().line;
        let Some(state) = document.state(rule, line).filter(|state| state.enabled) else {
            continue;
        };
        let mut gaps = Vec::with_capacity(2);
        gaps.push(opening_end..opening_gap_end);
        if closed {
            let start = raw[..closing_start].trim_end_matches([' ', '\t']).len();
            gaps.push(start..closing_start);
        }
        for gap in gaps {
            if gap.len() > 1 {
                result.push((base + gap.start + 1..base + gap.end, rule, state.severity));
            }
        }
    }
    result
}

pub(crate) fn propose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Edit> {
    let mut edits: Vec<_> = findings(source, parsed, document)
        .into_iter()
        .map(|(span, rule, _)| Edit {
            span,
            replacement: String::new(),
            origin: rule.into(),
        })
        .collect();
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
    edits.dedup();
    edits
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    findings(source, parsed, document)
        .into_iter()
        .map(|(span, rule, severity)| Diagnostic {
            code: rule.into(),
            rule_id: Some(rule.into()),
            severity,
            message: if rule == "MD019" {
                "Multiple spaces after hash on atx style heading"
            } else {
                "Multiple spaces inside hashes on closed atx style heading"
            }
            .into(),
            span: Some(span),
        })
        .collect()
}
