# Project guidance

## Scope and sources

- This project is a native Rust Markdown formatter, not a Node wrapper.
- Read `README.md` and the relevant roadmap/compatibility documents before
  changing behavior. User-approved decisions supersede historical audit proposals.
- There is no application scaffold yet. Do not claim formatter behavior is
  implemented because an acceptance example exists.
- Treat the pinned upstream sources as the compatibility reference. Distinguish
  source-reviewed behavior, runtime evidence, and proposed policy.

## Formatting requirements

- Respect supported markdownlint configuration, rule enablement, and inline
  directives.
- When MD013 is enabled, safely reflow prose to its configured width even when
  upstream would not report the original line. Exactly-at-limit lines are valid.
- Preserve inline links as indivisible units. Do not convert them to references.
- Preserve tables. Do not damage Markdown meaning to force a line-length fix.
- Apply MD026's configured trailing-punctuation fix when enabled; preservation
  is not a reason to withhold this approved rule-prescribed edit.
- Preserve original code if an external formatter fails or is unavailable.
- Require idempotence and regression coverage for implemented transformations.

## Working practices

- Use the Rust toolchain declared in `mise.toml`. Keep `Cargo.lock` tracked when
  the CLI project is created.
- Keep changes focused. Preserve unrelated user work.
- Do not add decorative or obvious comments; explain only surprising behavior.
- Verify relevant behavior before claiming success. Until Cargo exists, do not
  present audit consistency checks as application tests.
- The repository excludes the `git-commit` skill. Do not add a separate commit
  approval ceremony when the user has already requested a commit.
- Broad tool permissions are intentional, not permission to expand task scope.

## Worktrees and OpenCode configuration

- The main checkout is `/home/hannes/projects/mdfmt/main`. Feature worktrees must
  be siblings under `/home/hannes/projects/mdfmt/`; do not use OpenCode's default
  data-directory location. Reuse an existing suitable worktree when possible.
- Shared configuration lives at `/home/hannes/projects/mdfmt/opencode.jsonc`, above
  all worktrees. Do not move it into a checkout or add shadowing local copies.
- Before and after a session move, verify the effective model, permissions,
  policies, and plugins. Stop and restore configuration if settings or skills
  change unexpectedly; do not continue under fallback defaults.
