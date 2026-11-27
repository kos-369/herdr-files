# Contributing

Bug reports and pull requests are welcome.

To report a bug, open an issue. Say what you did, what you expected, what
happened, and which Herdr version you use. For a larger change, open an
issue first, so we can agree on it before you write the code.

## Set up

You need [Rust](https://rustup.rs), `cargo-deny` and `zizmor` (for
example `brew install cargo-deny zizmor`), and the Herdr app, which
provides `herdr-app`. Then turn on the pre-commit hook once per clone:

```sh
git config core.hooksPath .githooks
```

The hook runs `scripts/check`: formatting, Clippy, the tests, the crate
licenses, `herdr-app check` on the mod, and zizmor on the workflows. You
can also run `scripts/check` yourself.

## Where things are

- `crates/herdr-files/`: the Rust program that searches.
- `mods/herdr-files/`: the mod that draws the Files pane. It began as a
  copy of the Files pane that comes with Herdr.
- `AGENTS.md` and `docs/rules/`: the rules for each part. Read the file for
  what you change. They are written for coding agents, and they apply to
  people too.

## Try the mod

Run a second Herdr with its own folders, so your usual setup stays as it
is:

```sh
dev=/tmp/herdr-files-dev
mkdir -p "$dev/home/mods"
HERDR_APP_HOME="$dev/home" HC_SESSION=herdr-files-dev XDG_STATE_HOME="$dev/state" herdr-app
```

Do the rest in a terminal inside that Herdr. There `HERDR_APP_HOME` points
at `$dev/home`, so `herdr-app do` talks to this Herdr and not to your usual
one. From your clone of the repository, copy the mod and a build of the
program into that folder:

```sh
cargo build --release
cp -R mods/herdr-files "$HERDR_APP_HOME/mods/"
mkdir -p "$HERDR_APP_HOME/mods/herdr-files/bin"
cp target/release/herdr-files "$HERDR_APP_HOME/mods/herdr-files/bin/"
```

Then run `home=$HERDR_APP_HOME` and follow steps 2 and 3 of the README's
[Install](README.md#install) section. After that, copy the mod again
whenever you change it; Herdr loads changed mod files on its own.

## Pull requests

Keep one change in each pull request. The title follows
[Conventional Commits](https://www.conventionalcommits.org) (`fix:`,
`feat:`, `docs:` and so on), and CI checks it. Pull requests are
squash-merged, so the title becomes the commit on `main`.
[docs/rules/git.md](docs/rules/git.md) has the rest.

In the description, say in plain words what changed and how you tried it.
For a change to the Files pane, a screenshot helps.

## License

By sending a contribution, you agree that it is licensed under the
Apache License 2.0, like the rest of the project ([LICENSE](LICENSE)).
