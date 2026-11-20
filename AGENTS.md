# AGENTS.md

Rules for people and coding agents working on herdr-files.

## Code

- Every new or changed feature needs a test. Break the implementation on
  purpose once and confirm the test fails.
- Keep logic (search, scoring, `path:line` parsing) apart from I/O (CLI
  arguments, stdout, file walking), so the logic can be tested on its own.
- Write short comments in English. Say what a piece does and why.
- Keep the mod cloned from herdr-app's `defaults/mods/files/main.js` close to
  upstream, so upstream fixes are easy to bring in. `NOTICE` states where it
  comes from and that it was changed.
- Never commit secrets.

## Checks before committing

- Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test`.
- Mod: `herdr-app check mods/herdr-files`.

## Dependencies

- Before adding a crate, check that it is maintained and that its license
  works with Apache-2.0. Say why it is needed in the pull request.
- Keep the dependency tree small. `build.rs` and proc-macros run code on the
  developer's machine at build time.

## Running herdr-app locally

- Point `HERDR_APP_HOME` at a scratch home. Never change
  `~/.config/herdr-app`.

## Git

- `main` is protected. Work on a branch and open a pull request. A maintainer
  squash-merges it.
- Keep one change per pull request. Don't mix a feature with CI or
  refactoring. Add a crate in the pull request that first uses it.
- Bring in code from herdr-app (the cloned files mod) unchanged, in a pull
  request of its own. Change it in later pull requests, so the diff against
  upstream stays easy to see.
- Commit messages and pull request titles follow Conventional Commits
  (`feat:`, `fix:`, `docs:`, `test:`, `ci:`, `chore:`). The title becomes the
  commit on `main`.
- Sign commits and use your GitHub noreply address.
- Write pull request descriptions in short, plain English.

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
