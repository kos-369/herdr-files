# AGENTS.md

Rules for people and coding agents working on herdr-files.

## GitHub Actions workflows

This is a public repository. Code or text from a pull request must never run
with write access or secrets. Check every rule below whenever you add or change
a workflow.

- Trigger on `pull_request`, `push` or tags. Never use `pull_request_target`
  or `workflow_run`.
- Start every workflow with `permissions: contents: read`. Grant more per job,
  only where it is needed.
- Never put `${{ github.event.* }}` (titles, branch names, bodies) inside
  `run:`. Pass it through `env:` and use it quoted: `"$TITLE"`.
- Pin every action to a full commit SHA with a version comment:
  `actions/checkout@<40-char sha> # v4.2.2`. The repository enforces this.
- Use `persist-credentials: false` with `actions/checkout`.
- Release from a separate workflow that runs only on a tag push and uses no
  cache.
- Never store a personal access token in secrets. Use `GITHUB_TOKEN`.
- Never use self-hosted runners.
- Only GitHub-owned actions are allowed. Add any other action to the
  repository's allowlist first, and say why in the pull request.
- Check workflows with [zizmor](https://github.com/zizmorcore/zizmor) before
  committing.
