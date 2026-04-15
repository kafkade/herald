---
name: prep-pr
description: >
  Prepare a pull request description and update the changelog — but never
  create the PR itself. Generates a PR description using the repo's template,
  updates the Unreleased section of CHANGELOG.md with user-facing changes, and
  copies the PR description to the clipboard. Invoke when the user asks to
  "generate a PR description", "describe this PR", "write PR notes",
  "prepare a PR", "pr ready", or "prep this PR".
---

# PR Ready — Description + Changelog

Prepare a branch for pull request: generate a PR description from the diff AND update the changelog, then copy the PR description to the clipboard.

## Steps

### Phase 1: Gather context

1. **Detect the base branch**
   Run `git remote show origin` or check for `main` / `master` to determine the default branch.

2. **Identify the current branch**
   Run `git branch --show-current` to get the feature branch name.

3. **Gather the diff context**
   - Run `git log <base>..<current> --oneline` to get the commit list.
   - Run `git diff <base>..<current> --stat` to get the file change summary.
   - For each changed source file, read the diff to understand what changed. Focus on the `crates/` directory for code changes.
   - Skip binary files, lock files, and generated files.

4. **Read the PR template**
   - Look for `.github/pull_request_template.md` in the repository.
   - Use its exact structure (sections, checkboxes) as the PR description format.

5. **Read the current changelog**
   - Read `CHANGELOG.md` and note the existing `## [Unreleased]` section contents.
   - Understand the Keep a Changelog format used (Added, Changed, Deprecated, Removed, Fixed, Security).

### Phase 2: Run quality checks

6. **Verify the branch passes checks** (for the PR checklist)
   - Run `cargo fmt -- --check`
   - Run `cargo clippy -- -D warnings`
   - Run `cargo test`
   - Note which checks pass/fail to fill in the checklist accurately.

### Phase 3: Generate the PR description

7. **Write the PR description** using the PR template structure:
   - **Description section**: Write a clear summary of what the PR does. Include:
     - A one-line summary of the purpose
     - A "What's included" subsection listing key changes with brief explanations
     - Reference specific files/modules only when it adds clarity
   - **Related Issues**: Check commit messages for issue references (#123). If none, leave placeholder.
   - **Type of Change**: Check the appropriate box(es) based on diff content:
     - New files in `crates/` → New feature
     - Modified existing logic fixing bugs → Bug fix
     - Changes only in docs/ or *.md → Documentation update
     - Changes in `.github/` → CI / infrastructure
     - Restructuring without behavior change → Refactoring
   - **Checklist**: Mark items based on the results from step 6.

8. **Quality guidelines for the PR description**
   - Do NOT reference internal planning documents (PLAN.md stages, etc.) — describe actual features
   - Write from the user/contributor perspective
   - Be specific about what was added/changed
   - Keep the description concise but complete — aim for 15-30 lines

### Phase 4: Update the changelog

9. **Identify user-facing changes** from the diff. A change is user-facing if it:
   - Adds a feature the user can see or interact with
   - Fixes a bug the user could encounter
   - Changes behavior the user would notice (API endpoints, board behavior, UI, output format)
   - Adds or changes configuration options
   - **Is NOT user-facing**: refactoring, CI changes, test additions, internal restructuring, dependency updates, code style fixes

10. **Categorize each change** using Keep a Changelog categories:
    - **Added** — new features or capabilities
    - **Changed** — changes to existing functionality
    - **Deprecated** — features that will be removed
    - **Removed** — features that were removed
    - **Fixed** — bug fixes
    - **Security** — vulnerability fixes

11. **Update the `## [Unreleased]` section** of `CHANGELOG.md`:
    - **Append** new entries to the existing Unreleased section — do NOT delete what's already there
    - If a category header (e.g., `### Added`) already exists with entries, add new entries below the existing ones
    - If a category header doesn't exist yet, add it
    - Write entries as concise, user-facing descriptions — no implementation details
    - Each entry starts with `- ` (markdown list item)
    - Do NOT include entries for: CI changes, refactoring, dependency bumps, test-only changes, documentation-only changes

12. **Changelog entry style guide**
    - ✅ Good: `- REST API with full CRUD for messages, countdowns, queue, and config`
    - ✅ Good: `- Fixed countdown label validation rejecting valid 44-character labels`
    - ❌ Bad: `- Refactored db module` (not user-facing)
    - ❌ Bad: `- Added integration tests for queue endpoints` (not user-facing)
    - ❌ Bad: `- Implements Phase 1 from ROADMAP.md` (references internals)

### Phase 5: Output

13. **Copy the PR description to the clipboard**
    - Use PowerShell `Set-Clipboard` (Windows), `pbcopy` (macOS), or `xclip` (Linux)
    - Confirm to the user that the description has been copied

14. **Suggest a PR title**
    - Based on the changes, suggest a conventional-commit-style PR title
    - Format: `feat: add REST API for messages and countdowns` or `fix: correct queue reorder validation`

15. **Show a summary** to the user:
    - The suggested PR title
    - Confirmation that the PR description is on the clipboard
    - A summary of what was added to CHANGELOG.md (list the new entries)
    - Note any changelog entries that were already present and preserved

## Guardrails

**This skill MUST NOT:**
- Create a pull request (via GitHub MCP tools, `gh pr create`, or any other mechanism)
- Run `git commit`, `git commit --amend`, `git push`, `git push --force`, or any command that modifies git history
- Merge or close pull requests

PR creation and all git write operations are the sole responsibility of the human developer. See the Git Policy in `copilot-instructions.md`.
