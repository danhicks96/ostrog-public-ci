# What the hygiene check enforces

The `hygiene` check (`.github/workflows/hygiene.yml`, scanner `tools/hygiene/scan_identifiers.py`)
is the repository's main protection. It runs on every pull request, docs-only ones included, and
runs again when the title, description or base is edited. A failure names the rule and the line,
never the matched text. Issues get the text rules from `issue-hygiene.yml`.

| Where | What fails |
|---|---|
| Added file lines | email addresses, IP addresses, home paths, dynamic-DNS hosts, SSH logins, tokens; the owner's personal name (say "the maintainer"; `NOTICE` and `LICENSE-ASSETS` are exempt); the private repository's name (links to the public development repository are fine); "the box" (name the role, such as "the build server"; GDScript/scene files are exempt); AI tool and vendor names |
| PR title and description, issue title and body | all of the above, plus a "Generated with ..." footer, an attribution HTML comment, a "Requested by" line, project-thread or session links |
| Commit messages in the PR | AI or co-author trailers (`Co-Authored-By`, session trailers), and all of the text rules |
| Commit author and committer | an email that is not a noreply or example address; a name that is the owner's or an AI tool's (use the neutral identity "Ostrog CI") |
| Head branch name | a prefix naming an AI tool (`claude/`, `copilot/`, `codex/`, `gemini/`); use a neutral name such as `fix/<topic>` |

There are no inline suppression markers. The owner's name is matched by salted hash of whole
words, so no real name is stored in the scanner. Guard tests are `tools/tests/hygiene_rules_guard_test.py`
and `tools/tests/public_boundary_guard_test.py`.

To require it: Settings, Rules (or Branch protection), `main`, require status check **`hygiene`**
(see REPO-SETTINGS.md). Turn that on only after open pull requests' titles and descriptions are clean,
or it blocks them.
