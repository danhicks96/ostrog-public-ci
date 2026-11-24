# Automated protections

Every automatic check on this repository, what it guards, and what it needs. All workflows
declare least-privilege `permissions:`, pin third-party actions to a commit SHA (with the
version in a comment), and skip the Rust build, so none of them slows `ci`.

| Protection | File | Guards against | Runs | Needs |
|---|---|---|---|---|
| ci (rust, hygiene, export-structure, digests) | `workflows/ci.yml` | Broken code, identifying details in a PR | PRs, daily | nothing |
| Dependabot version updates | `dependabot.yml` | Stale cargo, actions and pip dependencies. Weekly, grouped minor/patch, 7-day cooldown, capped (3 cargo, 2 actions, 1 pip). `rayon` and major bumps of format-critical crates are ignored on purpose | Mondays | nothing |
| cargo-deny | `workflows/security-audit.yml`, `deny.toml` | Known advisories, yanked crates, licences incompatible with GPL-3.0-or-later, unknown registries | Weekly, and PRs touching Cargo files | nothing |
| Dependency review | `workflows/dependency-review.yml` | A PR adding a vulnerable or disallowed-licence dependency | PRs touching manifests | Public repo, or GitHub Code Security on a private one (the job tolerates its absence while private) |
| actionlint and zizmor | `workflows/workflow-lint.yml` | Workflow syntax errors, script injection, over-broad permissions, unpinned actions | PRs touching `.github/` | nothing |
| CodeQL (actions, rust) | `workflows/codeql.yml` | Injection and unsafe patterns in workflows and Rust; no build needed | Weekly, PRs touching `.github/` | Public repo, or Code Security plus repo variable `CODEQL_ENABLED=true` |
| OpenSSF Scorecard | `workflows/scorecard.yml` | Grades repo hygiene, branch protection and pinning | Weekly | Public repo (skipped while private) |
| Labeler | `workflows/labeler.yml`, `labeler.yml` | Triage noise: labels PRs by changed paths. Never runs PR code | PRs | nothing |
| Stale handling | `workflows/stale.yml` | Old open items; labels at 60 days, closes 30 days later, exempts `keep`, `pinned`, `security` | Weekly | nothing |
| CODEOWNERS | `CODEOWNERS` | Review requests for `.github/`, hygiene, licences | PRs | Code-owner review only matters if the ruleset requires it |
| Security policy | `/SECURITY.md` | Tells reporters to use private reporting | n/a | Private vulnerability reporting switched on |
| Issue and PR templates | `ISSUE_TEMPLATE/`, `PULL_REQUEST_TEMPLATE.md` | Reminds contributors of the rules and to share no private information | n/a | nothing |

## Settings these assume are on (not verifiable from files)

The repo owner reports these are already switched on. They live in GitHub settings, so a
pull request cannot confirm them; check them once by hand (`docs/REPO-SETTINGS.md` has the
steps).

- Dependabot alerts and Dependabot security updates (security PRs ignore the caps above).
- Secret scanning and push protection; private vulnerability reporting.
- A branch ruleset on `main` requiring `rust`, `hygiene` and `export-structure`. Add
  `deny`, `actionlint` and `zizmor` as required checks once each has run green once. Do not
  require `analyze` (CodeQL), `review` or `analysis` while the repo is private, since they
  are skipped or tolerated there.
- Actions allowed to use third-party actions from the list above.

## Notes

- A Dependabot Cargo PR still triggers the full `ci` run (about 1.5 hours). Grouping keeps
  that to roughly one PR a week.
- The hygiene scan reads PR titles and bodies, which Dependabot writes. If one trips on text
  copied from upstream release notes, edit the body rather than weakening the scanner.
- Actions pinned by SHA are refreshed by Dependabot's `github-actions` entry, which keeps the
  version comment current.
