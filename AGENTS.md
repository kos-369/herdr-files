# AGENTS.md

herdr-files adds search and path copy to the Files pane of the Herdr app: a
JavaScript mod (`mods/`) and a Rust CLI (`crates/`) that the mod runs.

## Always

- Run `scripts/check` before every commit. The pre-commit hook runs it; set
  the hook up once per clone with `git config core.hooksPath .githooks`.
- If a check fails, fix the cause. Don't skip or weaken the check.
- Don't write comments, doc comments included. Names and tests say what the
  code does. Write one only where leaving it out could lead to a serious
  bug, such as a security hole the code doesn't show. Comments in code
  copied from herdr-app stay, and so do the version notes after pinned
  action SHAs, which Dependabot updates.

## Before you change something, read its rules

Read only the file for what you are changing.

| You are changing | Read |
|---|---|
| Rust code in `crates/`, `Cargo.toml`, a dependency | [docs/rules/rust.md](docs/rules/rust.md) |
| The mod in `mods/` | [docs/rules/mod.md](docs/rules/mod.md) |
| `.github/workflows/` | [docs/rules/workflows.md](docs/rules/workflows.md) |
| Branches, commits, pull requests | [docs/rules/git.md](docs/rules/git.md) |

Rules that a tool enforces are not written down anywhere. These are:
formatting and lints (`scripts/check`), workflow security (zizmor), pull
request titles (CI), signed commits, the protected `main` branch and secret
scanning (GitHub settings).
