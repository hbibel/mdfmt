use crate::{
    Diagnostic, Source,
    config::DocumentConfig,
    parse::{ParsedDocument, RegionKind, parse},
};

fn blank_lines(source: &Source, parsed: &ParsedDocument) -> Vec<usize> {
    let mut children = vec![vec![]; parsed.regions.len()];
    for (index, region) in parsed.regions.iter().enumerate() {
        if let Some(parent) = region.parent {
            children[parent].push(index);
        }
    }
    let mut lines = vec![];
    for siblings in children {
        for pair in siblings.windows(2) {
            let left = &parsed.regions[pair[0]];
            let right = &parsed.regions[pair[1]];
            if left.kind != RegionKind::Blockquote || right.kind != RegionKind::Blockquote {
                continue;
            }
            let start = crate::md013::covered_lines(source, left.span.clone()).end + 1;
            let end = source.position(right.span.start).unwrap().line;
            for line in start..end {
                let text = &source.text()[source.line_range(line).unwrap()];
                if text.trim_matches([' ', '\t']).is_empty() {
                    lines.push(line);
                }
            }
        }
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    let lines = if source.text().contains('\t') {
        let (expanded, _) = crate::parse::expanded_tabs(source);
        match parse(&expanded) {
            Ok(parsed) => blank_lines(&expanded, &parsed),
            Err(_) => vec![],
        }
    } else {
        blank_lines(source, parsed)
    };
    lines
        .into_iter()
        .filter_map(|line| {
            let state = document
                .state("MD028", line)
                .filter(|state| state.enabled)?;
            let start = source.line_range(line)?.start
                + usize::from(line == 1 && source.text().starts_with('\u{feff}')) * 3;
            Some(Diagnostic {
                code: "MD028".into(),
                rule_id: Some("MD028".into()),
                severity: state.severity,
                message: "Blank line inside blockquote".into(),
                span: Some(start..start),
            })
        })
        .collect()
}
