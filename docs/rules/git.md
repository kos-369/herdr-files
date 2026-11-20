# Git rules

- Work on a branch and open a pull request. A maintainer squash-merges it, so
  the pull request title becomes the commit on `main`.
- Pull request titles follow Conventional Commits: `feat:`, `fix:`, `docs:`,
  `test:`, `refactor:`, `ci:` or `chore:`. CI checks this.
- Keep one change per pull request. Don't mix a feature with CI or
  refactoring. Add a crate in the pull request that first uses it.
- Bring in code from herdr-app (the cloned files mod) unchanged, in a pull
  request of its own. Change it in later pull requests, so the diff against
  upstream stays easy to see.
- Write pull request descriptions in short, plain English.
