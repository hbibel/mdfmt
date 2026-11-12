# Formatting convergence and conflicts

Formatting settles the complete pipeline before returning edits. Each existing
normalization loop has its own exact source history; repeated text at the same
phase proves a cycle, while stable completion exits without another observation.
Inner-loop histories reset for each invocation, so later outer passes do not
mistake a previously stable inner state for a cycle.

On a proven cycle, the transaction discards every attempted edit and returns the
original bytes, ordinary no-fix findings and a `rule-conflict` explanation naming
contributing edit origins. Origins are captured before composition can cancel
edits. This is not a minimal unsatisfiable-rule analysis. Cycle severity is Error
if a contributor is enabled as Error in the original document, otherwise Warning.
Warnings do not introduce an error exit.

Each tracker stores at most 256 observations per phase and source payload bytes
bounded by 32 times its invocation's input size plus 1 MiB. Exhaustion returns
`ProcessingError::WorkLimit`, never successful partial output or an invented
conflict. Existing CLI processing errors exit 2 before replacement. Trackers exist
at a fixed nesting depth; their aggregate bound is a constant multiple, not one
shared global budget. These bounds do not limit parser work within one pass.

## Evidence

The optimized complete suite passes 533 tests across 73 targets. Tracker tests
cover exact state/phase identity, distinct equal-length states, history and count
limits, and canceled-edit origins. Transaction tests exercise rollback with
ordinary original-source findings and bounded growth using injected pass outputs;
no newly discovered real-rule cycle is claimed. Public tests cover mixed rule
families, original-coordinate edit reconstruction, BOM/CRLF, disabled fixes and
bounded warning-only CLI priority cases. Existing unchanged assertions and CPU
limits also pass in the optimized suite; a complete debug run is not established.

Native gates pass 687 MD005, 1,680 MD007, 1,206 MD003, 440 MD026, 29 reflow,
1,145 each MD049/MD050, 180 combined inline-style, 792 MD022, 1,144 MD051 and
6,305 reference comparisons. One initial inline-style command lacked its required
selector; the corrected individual and combined commands pass.

Document-specific explanations from rejected list/separator proposals and the
independent batch review remain pending. Existing formatting priorities are
unchanged. All 53 built-ins remain listed with `Coverage.complete: false`;
convergence does not establish complete fixability, parser fidelity or later
roadmap milestones.
