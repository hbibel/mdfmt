use crate::{
    ProcessingError,
    edit::{Edit, compose_edits},
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Phase {
    Complete,
    Normalize,
    Trailing,
    Headings,
    Blanks,
    Markers,
}

#[derive(Debug)]
pub(crate) enum AttemptFailure {
    Cycle { rules: Vec<String> },
    Processing(ProcessingError),
}
impl From<ProcessingError> for AttemptFailure {
    fn from(error: ProcessingError) -> Self {
        Self::Processing(error)
    }
}

#[derive(Default)]
struct History {
    states: HashSet<String>,
    origins: BTreeSet<String>,
}
pub(crate) struct StateTracker {
    phases: HashMap<Phase, History>,
    remaining: usize,
}
impl StateTracker {
    pub(crate) fn new(input_bytes: usize) -> Self {
        Self {
            phases: HashMap::new(),
            remaining: input_bytes.saturating_mul(32).saturating_add(1024 * 1024),
        }
    }
    pub(crate) fn observe(
        &mut self,
        phase: Phase,
        source: &str,
        origins: &[String],
    ) -> Result<(), AttemptFailure> {
        let history = self.phases.entry(phase).or_default();
        history.origins.extend(origins.iter().cloned());
        if history.states.contains(source) {
            return Err(AttemptFailure::Cycle {
                rules: history.origins.iter().cloned().collect(),
            });
        }
        if history.states.len() >= 256 || source.len() > self.remaining {
            return Err(ProcessingError::WorkLimit(format!("{phase:?} source history")).into());
        }
        self.remaining -= source.len();
        history.states.insert(source.into());
        Ok(())
    }
}

pub(crate) fn compose(
    source: &str,
    previous: &[Edit],
    next: &[Edit],
    origins: &mut BTreeSet<String>,
) -> Result<Vec<Edit>, ProcessingError> {
    origins.extend(next.iter().map(|edit| edit.origin.clone()));
    compose_edits(source, previous, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProcessingError, edit::Edit};
    use std::collections::BTreeSet;

    #[test]
    fn repeated_exact_phase_state_reports_actual_contributing_origins() {
        let mut tracker = StateTracker::new(64);
        tracker.observe(Phase::Normalize, "- # H\n", &[]).unwrap();
        tracker.observe(Phase::Headings, "- # H\n", &[]).unwrap();
        tracker
            .observe(Phase::Normalize, "  - # H\n", &["MD007".into()])
            .unwrap();
        let failure = tracker
            .observe(Phase::Normalize, "- # H\n", &["MD023".into()])
            .unwrap_err();
        assert!(matches!(failure,AttemptFailure::Cycle{rules}if rules==vec!["MD007","MD023"]));
    }

    #[test]
    fn distinct_equal_length_states_do_not_use_length_or_hash_only_equality() {
        let mut tracker = StateTracker::new(64);
        for index in 0..128 {
            tracker
                .observe(Phase::Normalize, &format!("# value{index:03}\n"), &[])
                .unwrap();
        }
    }

    #[test]
    fn oversized_history_is_a_work_limit_without_polluting_exact_state_history() {
        let mut tracker = StateTracker::new(1);
        let oversized = "x".repeat(1024 * 1024 + 33);
        for _ in 0..2 {
            assert!(matches!(
                tracker.observe(Phase::Normalize, &oversized, &[]),
                Err(AttemptFailure::Processing(ProcessingError::WorkLimit(_)))
            ));
        }
        tracker.observe(Phase::Normalize, "x", &[]).unwrap();
    }

    #[test]
    fn observation_limit_does_not_masquerade_as_a_cycle() {
        let mut tracker = StateTracker::new(64);
        for index in 0..256 {
            tracker
                .observe(Phase::Normalize, &format!("state{index:03}"), &[])
                .unwrap();
        }
        assert!(matches!(
            tracker.observe(Phase::Normalize, "new-state", &[]),
            Err(AttemptFailure::Processing(ProcessingError::WorkLimit(_)))
        ));
        assert!(matches!(
            tracker.observe(Phase::Normalize, "state000", &[]),
            Err(AttemptFailure::Cycle { .. })
        ));
    }

    #[test]
    fn edit_origins_survive_source_composition_that_undoes_an_insertion() {
        let mut origins = BTreeSet::new();
        let source = "- # H\n";
        let inserted = compose(
            source,
            &[],
            &[Edit {
                span: 0..0,
                replacement: "  ".into(),
                origin: "MD007".into(),
            }],
            &mut origins,
        )
        .unwrap();
        let restored = compose(
            source,
            &inserted,
            &[Edit {
                span: 0..2,
                replacement: String::new(),
                origin: "MD023".into(),
            }],
            &mut origins,
        )
        .unwrap();
        assert_eq!(
            origins.into_iter().collect::<Vec<_>>(),
            vec!["MD007", "MD023"]
        );
        let mut actual = String::new();
        let mut cursor = 0;
        for edit in restored {
            actual.push_str(&source[cursor..edit.span.start]);
            actual.push_str(&edit.replacement);
            cursor = edit.span.end;
        }
        actual.push_str(&source[cursor..]);
        assert_eq!(actual, source);
    }
}
