# Rust rules

- Every new or changed feature needs a test. Break the implementation on
  purpose once and confirm the test fails.
- Keep logic (search, scoring, `path:line` parsing) apart from I/O (CLI
  arguments, stdout, file walking), so the logic can be tested on its own.

## Adding a crate

- Check that it is maintained and that its license works with Apache-2.0.
  Say why it is needed in the pull request.
- Keep the dependency tree small. `build.rs` and proc-macros run code on the
  developer's machine at build time.
