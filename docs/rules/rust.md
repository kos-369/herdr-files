# Rust rules

- Every new or changed feature needs a test. Break the implementation on
  purpose once and confirm the test fails.
- Keep logic (search, scoring, `path:line` parsing) apart from I/O (CLI
  arguments, stdout, file walking), so the logic can be tested on its own.

## Adding a crate

- Check that it is maintained. Say why it is needed in the pull request.
- If cargo-deny rejects its license, find out whether that license works
  with Apache-2.0. If it does, allow it for that crate only in `deny.toml`,
  and add the notice the license asks for to NOTICE, which ships with the
  binaries: the license text for MIT, where the source is for MPL-2.0.
- Keep the dependency tree small. `build.rs` and proc-macros run code on the
  developer's machine at build time.
