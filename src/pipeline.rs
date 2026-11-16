use crate::{
    Coverage, Diagnostic, FormatOptions, FormatResult, ProcessingError, Source,
    config::{DocumentConfig, ResolvedConfig},
    edit::{self, Edit},
    parse::{self, ParsedDocument},
};
use std::cell::OnceCell;

pub(crate) struct Snapshot {
    pub source: Source,
    pub parsed: ParsedDocument,
    pub document: DocumentConfig,
    authorized: OnceCell<Vec<Edit>>,
}

impl Snapshot {
    pub fn enabled(&self, rules: &[&str]) -> bool {
        rules
            .iter()
            .any(|rule| self.document.enabled_anywhere(rule))
    }

    pub fn diagnose(
        &self,
        rules: &[&str],
        diagnose: fn(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Diagnostic>,
    ) -> Vec<Diagnostic> {
        if self.enabled(rules) {
            diagnose(&self.source, &self.parsed, &self.document)
        } else {
            vec![]
        }
    }
    fn authorized(&self) -> &[Edit] {
        self.authorized.get_or_init(|| {
            let mut edits = vec![];
            for (rules, propose) in [
                (
                    &["MD026"][..],
                    crate::md026::propose
                        as fn(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>,
                ),
                (&["MD019", "MD021"][..], crate::heading_spaces::propose),
                (&["MD012"][..], crate::md012::propose),
                (&["MD009"][..], crate::md009::propose),
                (&["MD010"][..], crate::md010::propose),
                (&["MD023"][..], crate::md023::propose),
                (&["MD027"][..], crate::md027::propose),
                (&["MD029"][..], crate::md029::propose),
                (&["MD030"][..], crate::md030::propose),
                (&["MD005"][..], crate::md005::propose),
                (&["MD007"][..], crate::md007::propose),
            ] {
                if rules
                    .iter()
                    .any(|rule| self.document.enabled_anywhere(rule))
                {
                    edits.extend(propose(&self.source, &self.parsed, &self.document));
                }
            }
            edits
        })
    }
}

pub(crate) struct Context<'config> {
    config: &'config ResolvedConfig,
    current: Option<Snapshot>,
}

impl<'config> Context<'config> {
    pub fn new(config: &'config ResolvedConfig) -> Self {
        Self {
            config,
            current: None,
        }
    }

    pub fn snapshot(&mut self, text: &str) -> Result<&Snapshot, ProcessingError> {
        if self
            .current
            .as_ref()
            .is_none_or(|snapshot| snapshot.source.text() != text)
        {
            let source = self.config.source(text)?;
            let document = self
                .config
                .document(text)
                .map_err(ProcessingError::Configuration)?;
            let parsed = parse::parse(&source).map_err(ProcessingError::Parse)?;
            self.current = Some(Snapshot {
                source,
                parsed,
                document,
                authorized: OnceCell::new(),
            });
        }
        Ok(self.current.as_ref().unwrap())
    }

    pub fn process(
        &mut self,
        text: &str,
        options: &FormatOptions,
        rules: &[&str],
        produce: impl FnOnce(&Source, &ParsedDocument, &DocumentConfig) -> Vec<Edit>,
        diagnose: impl FnOnce(&str) -> Vec<Diagnostic>,
    ) -> Result<FormatResult, ProcessingError> {
        let requested = options.fix && self.config.fix;
        let snapshot = self.snapshot(text)?;
        let fix = requested && snapshot.enabled(rules);
        let source = &snapshot.source;
        let parsed = &snapshot.parsed;
        let document = &snapshot.document;
        let authorized = if fix { snapshot.authorized() } else { &[] };
        let mut edits = if fix && !parse::has_uncertain_container_prefixes(source, parsed) {
            let proposed = produce(source, parsed, document);
            edit::validate_ranges(source, &proposed)?;
            proposed
                .into_iter()
                .filter(|edit| {
                    authorized.contains(edit)
                        || document.allows_source(&edit.origin, edit.span.clone(), source)
                })
                .collect()
        } else {
            vec![]
        };
        edits.sort_by_key(|edit| (edit.span.start, edit.span.end));
        let output = edit::apply_authorized_edits(source, parsed, edits.clone(), authorized)?;
        let diagnostics = diagnose(&output);
        Ok(FormatResult {
            output,
            edits,
            diagnostics,
            coverage: Coverage {
                implemented_rules: vec![],
                complete: false,
            },
        })
    }
}
