# Mod rules

- The mod starts as a copy of herdr-app's `defaults/mods/files/main.js`. Keep
  it close to upstream, so upstream fixes are easy to bring in. `NOTICE` says
  where it comes from and that it was changed.
- Write for the released herdr-app, not its `master`. The type files the
  installed app writes (`~/.config/herdr-app/herdr.d.ts`, `gpui-kit.d.ts`)
  are the truth.
- Run herdr-app with `HERDR_APP_HOME` pointing at a scratch home. Never change
  `~/.config/herdr-app`.
- `herdr-app check` does not lay out the mod or run later async states. Try
  every change in the app too.
