# Audit ledger — plan: docs/superpowers/plans/2026-10-21-compatibility-audit.md

## Execution decisions

- Ruling: Work in the existing directory and keep this ledger with the audit.
  There is no Git repository, and the approved plan forbids initializing one.
  Git worktree/commit-based skill scripts therefore do not apply. Cost if wrong:
  no Git history or isolated branch protects these documentation changes.
- Ruling: This milestone produces documentation and acceptance data, not
  production code. Verify source inventories and fixture consistency rather
  than claim a formatter RED/GREEN test cycle. Cost if wrong: runtime behavior
  still needs oracle testing in a later milestone.
- Ruling: Add `verification.md` for reproducible audit commands and retain this
  ledger because no Git history exists. Cost if wrong: two additional audit
  documents need maintenance.
- Ruling: Prefer pinned source behavior over conflicting MD013 documentation,
  and record the discrepancy rather than claim experimental lint parity. Cost
  if wrong: runtime oracle checks may require updating the acceptance cases.
- Ruling: Treat preserved tables as unchanged row/delimiter source, allowing
  safe adjacent blank-line edits. Cost if wrong: fewer automatic fixes inside
  cells than the user might intend.
- Ruling: Propose canonical width-based prose reflow even where MD013's lint
  exceptions suppress a diagnostic. Mark this for approval, not as settled
  requirements. Cost if wrong: reflow examples need a narrower trigger policy.

## Pre-flight

- Tasks 1 → 2: pinned sources determine configuration semantics; consistent.
- Tasks 1 + 2 → 3: inventories determine acceptance expectations; consistent.
- Tasks 1–3 → 4: audit artifacts provide the review package; consistent.

## Tasks

- [x] Task 1: Pin the reference and inventory rules. npm metadata and Git tags
  agree; source/schema/registry checks pass for 53 rules and 33 fix emitters.
- [x] Task 2: Specify configuration compatibility. Source/schema comparison
  covers all 16 properties; unresolved boundaries are explicit in the baseline.
- [x] Task 3: Define formatting acceptance cases. All 43 normal and 8 error
  examples pass consistency checks in `verification.md`. Narrow source
  experiments pass; complete oracle and formatter tests remain pending.
- [x] Task 4: Review and hand off. Fresh read-only review completed; source
  checks confirmed both findings and the final consistency/source experiments
  pass. Contract decisions remain explicitly subject to user approval.

## Final review

- Fixed: explicit `--config` precedence was incomplete. Added competing explicit
  base/discovered cli2 and standalone config cases, documented source precedence,
  and ran a missing-fixture regression check RED → GREEN plus both verification
  blocks. Full resolution remains source-reviewed, not end-to-end tested.
- Fixed: quoted front-matter title default differs from schema. Regraded this
  from Minor to Important because incorrect defaults would report false
  violations. Recorded source defaults for all three rules, added a quoted-title
  acceptance case, and checked the inventory pattern against positive/negative
  examples and upstream's extracted helper. Both verification blocks pass.
- Ruling: Runtime completeness, full lint parity, semantic equivalence, and
  idempotence remain unverified until application/oracle execution exists.
  This is an audit, not an implementation. Cost if wrong: later runtime tests
  may require changes to the contract's expected behavior.
- Ruling: Regex/package resolution deferrals, conservative repair restrictions,
  and canonical reflow remain proposals for user approval, not authorizations
  silently added by this audit. Cost if wrong: first-release scope or acceptance
  cases must change.
- Ruling: Parser and formatter-adapter selection stay in later milestone plans.
  Cost if wrong: feasibility work could change the milestone ordering.
- Deferred minors: none.

## User-approved release staging

- Custom front-matter and rule regex support belongs to a later release, and
  remains on the long-term roadmap.
- Installed-package `extends` resolution also belongs to a later release.
  It reads already-installed declarative configs locally; it does not fetch
  from npm, install packages, or interpret `.npmrc`.
- Updated the roadmap, baseline, configuration contract, and error-case scope
  to distinguish first-release limitations from long-term compatibility goals.
- The content-changing repair policy and canonical MD013 reflow proposal remain
  open; this approval concerned release staging only.

## User-approved MD026 behavior

- Fix MD026 when enabled, including its default-enabled state. The rule-prescribed
  punctuation removal must not be blocked solely because it changes heading text.
- Reclassified MD026 from diagnostic-only to safe under that authorization;
  retained configuration and upstream exceptions, not a blanket rewrite policy.
- Added seven acceptance cases for default fixing, disabling, custom punctuation,
  question marks, HTML entities, and an empty punctuation set.
- Canonical MD013 reflow remains the next open proposal. The earlier blanket
  proposal to withhold MD026 repair is superseded by this user decision.

## User-approved width-based prose reflow

- Reflow eligible prose to the configured MD013 width even when upstream's
  diagnostic exceptions exempt the original overlong line. The configured width
  is a formatting goal, and a line exactly at the limit is allowed.
- Preserve the agreed boundaries for indivisible links/code, tables, hard breaks,
  disabled rules, and unsafe transformations. Keep upstream diagnostic exceptions
  when reporting remaining violations rather than silently enabling strict mode.
- Added the discussed 40-character example as an acceptance case. This approval
  supersedes earlier entries that described canonical reflow as an open proposal.
