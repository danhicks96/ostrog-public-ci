# GitHub settings for the public repo

Only the repo owner can set these. Do them in this order; items 1-4 work on a private repo too.

1. **Settings > General > Features:** Issues off, Discussions off, Wiki off, Projects off.
2. **Settings > General > Pull Requests:** Allow merge commits on (default message: pull request
   title and description); Allow squash merging off; Allow rebase merging off; Allow auto-merge
   **on**; Automatically delete head branches on.
3. **Settings > Actions > General:**
   - Allow all actions (or only GitHub-owned and verified creators).
   - Fork pull request workflows: **Require approval for all outside collaborators**.
   - Workflow permissions: **Read repository contents and packages permissions**; leave "Allow
     GitHub Actions to create and approve pull requests" off. (automerge.yml and ci.yml declare
     the few write permissions they need.)
4. **Settings > Rules > Rulesets > New branch ruleset** for `main` (the default branch):
   - Enforcement: Active. Target: default branch.
   - Restrict deletions, Block force pushes.
   - Require a pull request before merging; required approvals **0** (so auto-merge needs no
     reviewer).
   - Require status checks to pass: add **`rust`**, **`hygiene`** and **`export-structure`** (the jobs
     in `.github/workflows/ci.yml`; they appear in the list only after each has run once, so run ci
     via Actions > ci > Run workflow first). Do **not** require `box-ci`, `digest` or the soaks.
   - Require branches to be up to date before merging: on only if you want strict, which makes
     each PR re-test after another merge.
   - Bypass list: your own account (so the box's `tools/merge/gated_merge.sh` and emergency
     fixes can still push).
5. **Settings > Code security:** Secret scanning on, **Push protection** on, Dependabot alerts
   on, Private vulnerability reporting on. Do not supply confidential inputs to public jobs.
   The generic hygiene and export-structure jobs do not establish private audit clearance.
6. **Settings > General > Pull Requests** (if GitHub offers "Pull request creation: collaborators
   only", pick it); otherwise rely on CONTRIBUTING.md and close outside PRs. Optionally set
   **Moderation options > Interaction limits > Limit to existing users** (repeats up to 6 months).
7. **Settings > Collaborators and teams:** keep only the accounts that open PRs. Agent PRs are
   opened by your login; automerge.yml allows `danhicks96`. Another fixer account must be added
   to the list in `.github/workflows/automerge.yml` and to `BOXCI_ALLOW` in
   `tools/boxci/compose.yaml`.
8. **Your profile:** Settings > Emails: "Keep my email address private" and "Block command line
   pushes that expose my email".
9. **Last, after step 4 is in place and `rust` has gone green once:** add the repository variable
   `AUTOMERGE_ENABLED` = `true` (Settings > Secrets and variables > Actions > Variables).
   Until then automerge.yml does nothing. Never set it before the ruleset requires `rust` and
   `hygiene` and `export-structure`: with no required checks, auto-merge merges at once with no CI.
10. Do **not** switch visibility to Public until the publishing runbook has been followed.

How auto-merge works with these settings: a PR from this repo by an allowed login gets
`gh pr merge --auto --merge` from automerge.yml. GitHub waits for the required checks `rust`,
`hygiene` and `export-structure`,
then merges with a merge commit. Nothing merges if `rust` fails. Merges made this way do not
create the `cNN` tag or run the mirror checkpoint; the box's checkpoint script catches up later
(`tools/checkpoint.sh`).

Not tested end to end: Actions could not be run from the session that wrote this. First use:
open a trivial PR, confirm `rust` goes green and the PR merges itself, before relying on it.
