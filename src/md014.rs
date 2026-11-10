use crate::{
    Diagnostic, Source,
    config::{DocumentConfig, js_space},
    parse::ParsedDocument,
};

pub(crate) fn diagnose(
    source: &Source,
    parsed: &ParsedDocument,
    document: &DocumentConfig,
) -> Vec<Diagnostic> {
    if !source.text().contains('$')
        || !(1..=source.line_count()).any(|line| {
            document
                .state("MD014", line)
                .is_some_and(|state| state.enabled)
        })
    {
        return vec![];
    }
    let mut result = vec![];
    for block in crate::parse::diagnostic_code_blocks(source, parsed) {
        let fenced = block.fenced;
        let first = source.position(block.span.start).unwrap().line + usize::from(fenced);
        // A consumed tab can leave an empty codeFlowValue, including at an
        // unclosed fence's EOF where mdast drops the trailing empty value.
        if fenced
            && (first..=crate::md013::covered_lines(source, block.span.clone()).end).any(|line| {
                source.line_range(line).is_some_and(|range| {
                    let raw = &source.text()[range];
                    raw.contains('\t') && raw.trim_matches([' ', '\t', '>']).is_empty()
                })
            })
        {
            continue;
        }
        let values: Vec<_> = block
            .value
            .split('\n')
            .enumerate()
            .filter(|(_, value)| !value.is_empty())
            .collect();
        let commands = values.iter().all(|(_, value)| {
            let value = value.trim_start_matches(js_space);
            value.starts_with('$') && value[1..].chars().next().is_some_and(js_space)
        });
        if !commands {
            continue;
        }
        for (offset, value) in values {
            let line = first + offset;
            let Some(state) = document.state("MD014", line).filter(|state| state.enabled) else {
                continue;
            };
            let Some(range) = source.line_range(line) else {
                continue;
            };
            let raw = &source.text()[range.clone()];
            let Some(dollar) = raw.find('$') else {
                continue;
            };
            let gap = raw[dollar + 1..]
                .chars()
                .take_while(|ch| js_space(*ch))
                .map(char::len_utf8)
                .sum::<usize>();
            let context = if crate::utf16_len(value) > 30 {
                format!(
                    "{}...",
                    String::from_utf16_lossy(&value.encode_utf16().take(30).collect::<Vec<_>>())
                )
            } else {
                value.into()
            };
            result.push(Diagnostic{code:"MD014".into(),rule_id:Some("MD014".into()),severity:state.severity,
                message:format!("Dollar signs used before commands without showing output [Context: \"{context}\"]"),span:Some(range.start+dollar..range.start+dollar+1+gap)});
        }
    }
    result.sort_by_key(|finding| finding.span.as_ref().unwrap().start);
    result
}
