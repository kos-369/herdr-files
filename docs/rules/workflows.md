# Workflow rules

This is a public repository. Code or text from a pull request must never run
with write access or secrets.

zizmor checks every workflow (`scripts/check` and CI). Its settings are in
`.github/zizmor.yml`. It catches `pull_request_target` and `workflow_run`,
missing or broad `permissions:`, `${{ }}` inside `run:`, actions not pinned to
a commit SHA, `actions/checkout` without `persist-credentials: false`, and
caches in release workflows. To fix an injection, pass the value through
`env:` and use it quoted: `"$TITLE"`.

zizmor does not check these:

- Release from a separate workflow that runs only on a tag push.
- Never store a personal access token in secrets. Use `GITHUB_TOKEN`.
- Never use self-hosted runners.
- Only GitHub-owned actions are allowed (a repository setting). To use
  another, add it to the allowlist first and say why in the pull request.
