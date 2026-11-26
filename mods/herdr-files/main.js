// A lazy file browser for the focused pane's folder, with bounded read-only previews.
import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, Editor, EditorState } from "gpui-base";
import { spaces, panes, focused, exec, readTextFile, watchFolder, theme, settings, window } from "herdr";
import { props, openOverlay, closeOverlay, notify, appHome, home, layout, run, shared, setShared } from "herdr";
import { InputState, Input, Scrollbar, v_virtual_list, VirtualListScrollHandle } from "gpui-base";

function location() {
  const space = spaces().find(s => s.active), f = focused();
  const pane = f?.pane ? panes().find(p => p.pane === f.pane) : null;
  const on = f?.pane === null ? (f.terminal?.includes("/") ? f.terminal.split("/")[0] : null) : space?.machine ?? null;
  return { cwd: pane?.cwd || f?.cwd || space?.folder, on, space: space?.id };
}
const key = () => JSON.stringify(location());
const LANG = { ts: "js", tsx: "js", mts: "js", cts: "js", jsx: "js", mjs: "js", toml: "yaml", jsonc: "json", svelte: "html", vue: "html" };
const PER_FOLDER = 500, TOTAL = 5000;
const SEARCH_DELAY = 150, GREP_TIMEOUT = 5000;
const INSTALL_URL = "https://github.com/kos-369/herdr-files#install";

function lineRange(text, line, column) {
  const lines = text.split("\n");
  const n = Math.min(Math.max(line, 1), lines.length);
  const start = lines.slice(0, n - 1).reduce((sum, l) => sum + l.length + 1, 0);
  const length = lines[n - 1].length;
  if (column == null) return { start, end: start + length };
  const at = start + Math.min(Math.max(column, 1) - 1, length);
  return { start: at, end: at };
}

function grouped(hits) {
  const rows = [];
  let file;
  for (const hit of hits) {
    if (file?.path !== hit.path) rows.push(file = { key: hit.path, path: hit.path, count: 0 });
    file.count++;
    rows.push({ ...hit, key: `${hit.path}:${hit.line}` });
  }
  return rows;
}

function pathTo(node, mod) {
  if (node?.content?.mod === mod) return [node];
  for (const child of node?.children ?? []) {
    const path = pathTo(child, mod);
    if (path) return [node, ...path];
  }
  return null;
}

async function directory(path, on) {
  const windowsDirectory = async () => {
    // Windows PowerShell otherwise uses the console code page; exec decodes UTF-8.
    // Links keep directory attributes even when dangling; check their target.
    // Other reparse points (OneDrive's folders) have no LinkType and are folders.
    // PowerShell reads curly single quotes as quotes too, so they're doubled like '.
    const r = await exec("powershell", ["-NoProfile", "-Command", "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); & { param($p) @(Get-ChildItem -LiteralPath $p -Force -ErrorAction Stop | Where-Object Name -ne '.git' | Select-Object -First 501 | ForEach-Object { @{ name=$_.Name; dir=($_.PSIsContainer -and (-not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -or -not $_.LinkType -or ($_.Target -and [IO.Directory]::Exists(([IO.Path]::Combine([IO.Path]::GetDirectoryName($_.FullName), $_.Target[0])))))) } }) | ConvertTo-Json -Compress } '" + path.replace(/['‘’‚‛]/g, "$&$&") + "'"], { on });
    if (r.code !== 0) throw new Error(r.stderr || "Cannot list this folder");
    const result = JSON.parse(r.stdout || "[]");
    return Array.isArray(result) ? result : [result];
  };
  if (!on && window()?.platform === "windows") return windowsDirectory();
  const script = 'n=0; for p in "$1"/* "$1"/.[!.]* "$1"/..?*; do [ -e "$p" ] || [ -L "$p" ] || continue; name=${p##*/}; [ "$name" = .git ] && continue; if [ -d "$p" ]; then kind=d; else kind=f; fi; printf "%s\\0%s\\0" "$kind" "$name"; n=$((n+1)); [ "$n" -gt 500 ] && break; done; test -d "$1"';
  let r;
  try { r = await exec("sh", ["-c", script, "files", path], { on }); }
  catch (e) {
    if (on && String(e).includes("cannot start sh: program not found")) return windowsDirectory();
    throw e;
  }
  if (r.code !== 0) throw new Error(r.stderr || "Cannot list this folder");
  const fields = r.stdout.split("\0"), out = [];
  for (let i = 0; i + 1 < fields.length; i += 2) out.push({ name: fields[i + 1], dir: fields[i] === "d" });
  return out;
}

export default class Files extends View {
  init(_props, cx) {
    if (props()?.menu) return;
    if (props()?.launch) return this.launch();
    this.asked = shared("herdr-files.search");
    this.handle = cx.focus_handle();
    this.search = InputState.new({ placeholder: "Search files" });
    this.search.on("change", (_e, cx) => this.find(cx));
    this.search.on("submit", (_e, cx) => this.open(this.results[this.at], cx));
    this.exclude = InputState.new({ placeholder: "Files to exclude, e.g. target/, *.log", value: localStorage.getItem("exclude") ?? "" });
    this.exclude.on("change", (_e, cx) => { localStorage.setItem("exclude", this.exclude.value()); if (this.query) this.find(cx); });
    this.query = ""; this.mode = "names"; this.results = []; this.searchSeq = 0; this.at = 0; this.list = VirtualListScrollHandle.new();
    for (const [id, state] of [["files-search", this.search], ["files-exclude", this.exclude]]) {
      state.on("focus", (_e, cx) => { this.typing = id; cx.notify(); });
      state.on("blur", (_e, cx) => { if (this.typing === id) this.typing = null; cx.notify(); });
    }
    this.saved = new Map();
    this.entries = new Map(); this.folders = new Set(); this.pending = new Set(); this.marks = {};
    this.refresh(cx);
    cx.timer.every(10000, cx => this.refresh(cx));
  }

  watch(root, cx) {
    if (this.on || this.watched === root) return;
    this.watched = root;
    const context = this.context;
    cx.spawn(async cx => {
      try { while (this.context === context) { await watchFolder(root); if (this.current(context)) this.refresh(cx); } }
      catch { if (this.context === context) this.watched = null; }
    });
  }

  current(context) {
    return this.context === context && key() === context.key;
  }

  async loadFolder(path, context) {
    const { on, cwd: root } = context;
    const entries = await directory(path ? `${root}/${path}` : root, on);
    if (!this.current(context) || (path && !this.folders.has(path))) return;
    const used = [...this.entries.entries()].filter(([p]) => p !== path).reduce((n, [, e]) => n + e.length, 0);
    this.entries.set(path, entries.slice(0, Math.max(0, Math.min(PER_FOLDER, TOTAL - used))).sort((a, b) => Number(b.dir) - Number(a.dir) || a.name.localeCompare(b.name)));
    if (entries.length > PER_FOLDER || used + entries.length > TOTAL) this.limited = true;
  }

  refresh(cx) {
    const { cwd, on } = location(), next = key();
    if (next !== this.context?.key) {
      if (this.context) this.saved.set(this.context.key, { folders: this.folders, file: this.file });
      const saved = this.saved.get(next);
      this.context = { key: next, cwd, on }; this.root = cwd; this.on = on; this.entries = new Map(); this.folders = new Set(saved?.folders);
      this.pending = new Set(); this.marks = {}; this.show(null); this.watched = null; this.error = null; this.limited = false;
      this.loading = false;
      if (saved?.file) this.show(saved.file, cx);
    }
    if (!cwd || this.loading) return;
    const context = this.context;
    this.loading = true;
    this.limited = false;
    cx.spawn(async cx => {
      try {
        for (const path of ["", ...this.folders]) {
          if (!this.current(context)) return;
          await this.loadFolder(path, context);
        }
        const top = await exec("git", ["rev-parse", "--show-toplevel"], { cwd, on }).catch(() => null);
        const marks = {};
        if (top?.code === 0) {
          // Writes nothing under .git (index lock, fsmonitor cookie): the folder watch would refresh again.
          const status = await exec("git", ["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain=v1", "-z", "--no-renames", "--untracked-files=all"], { cwd, on }).catch(() => null);
          if (status?.code === 0) for (const entry of status.stdout.split("\0").filter(Boolean).slice(0, TOTAL)) {
            const xy = entry.slice(0, 2);
            marks[`${top.stdout.trim()}/${entry.slice(3)}`] = xy === "??" || xy.includes("A") ? "A" : xy.includes("U") ? "U" : xy.includes("D") ? "D" : "M";
          }
        }
        if (!this.current(context)) return;
        this.marks = marks; this.error = null; this.watch(cwd, cx);
      } catch (e) { if (this.current(context)) this.error = String(e); }
      finally { if (this.context === context) this.loading = false; cx.notify(); }
    });
  }

  toggle(path, cx) {
    if (this.folders.has(path)) {
      for (const folder of [...this.folders]) if (folder === path || folder.startsWith(path + "/")) {
        this.folders.delete(folder);
        this.entries.delete(folder);
      }
    } else {
      this.folders.add(path);
      if (!this.entries.has(path) && !this.pending.has(path)) {
        const context = this.context;
        this.pending.add(path);
        cx.spawn(async cx => {
          try { await this.loadFolder(path, context); }
          catch (e) { if (this.current(context)) this.error = String(e); }
          finally { if (this.context === context) this.pending.delete(path); cx.notify(); }
        });
      }
    }
    cx.notify();
  }

  show(path, cx, focus, target) {
    if (focus) this.handle.focus();
    this.code?.release(); this.code = null; this.file = path; this.fileError = null;
    if (!path || !cx) return;
    const context = this.context, root = this.root, on = this.on;
    cx.spawn(async cx => {
      try {
        const text = await readTextFile(`${root}/${path}`, { on });
        if (this.file !== path || !this.current(context)) return;
        if (text === null) { this.fileError = "This file is no longer available"; return; }
        if (text.includes("\0")) { this.fileError = "Binary file · no text preview"; return; }
        const t = theme(), ext = path.includes(".") ? path.split(".").pop() : "txt";
        this.code = EditorState.new({ language: LANG[ext] ?? ext, readonly: true,
          value: text.length > 400000 ? text.slice(0, 400000) + "\n… (file truncated)" : text,
          font_family: settings().font_family, font_size: 12,
          syntax: { keyword: t.ansi5, string: t.ansi2, comment: t.dim, function: t.ansi4, type: t.ansi3, number: t.ansi1, constant: t.ansi6 } });
        if (target?.line != null) this.code.set_selected_range(lineRange(this.code.value(), target.line, target.column));
        if (focus) this.code.focus();
      } catch { if (this.file === path && this.current(context)) this.fileError = "Cannot preview this file (binary, too large, or unreadable)"; }
      finally { cx.notify(); }
    });
  }

  rows(prefix, depth, out, t) {
    if (out.length >= TOTAL || depth > 40) return out;
    for (const entry of this.entries.get(prefix) ?? []) {
      if (out.length >= TOTAL) break;
      const path = prefix ? `${prefix}/${entry.name}` : entry.name, open = this.folders.has(path), mark = this.marks[`${this.root}/${path}`];
      out.push(h_flex().id(`file-${path}`).h(28).flex_none().pl(8 + depth * 18).pr(8).gap(7).items_center().rounded(7).cursor_pointer().hover(s => s.bg(t.hover))
        .on_click((_e, cx) => entry.dir ? this.toggle(path, cx) : this.show(path, cx))
        .on_mouse_down("right", (_e, cx) => { cx.stop_propagation(); this.openMenu(path); })
        .child(entry.dir ? svg(`mods/herdr-files/chevron-${open ? "down" : "right"}.svg`).size(12).flex_none().text_color(t.dim) : div().w(12).flex_none())
        .child(svg(`mods/herdr-files/${entry.dir ? "folder" : "file"}.svg`).size(14).flex_none().text_color(t.muted))
        .child(div().flex_1().min_w(0).truncate().child(entry.name))
        .when(Boolean(mark), el => el.child(div().text_size(11).text_color(mark === "A" ? t.ansi2 : mark === "D" || mark === "U" ? t.ansi1 : t.blocked).child(mark))));
      if (entry.dir && open) this.rows(path, depth + 1, out, t);
    }
    return out;
  }

  openMenu(path) {
    openOverlay({ mod: "herdr-files", kind: "popup", at: "pointer", width: 220, props: { menu: { root: this.root, on: this.on, path } } });
  }

  menu(t) {
    const { root, path } = props().menu;
    const absolute = root.endsWith("/") ? root + path : `${root}/${path}`;
    const copy = (text, cx) => {
      cx.write_to_clipboard(text);
      notify({ title: "Copied path", body: text, level: "success" });
      closeOverlay(props().overlay);
    };
    const item = (id, label, text) => h_flex().id(id).h(31).px(8).rounded(7).items_center().text_color(t.foreground)
      .hover(s => s.bg(t.hover)).on_click((_e, cx) => copy(text, cx)).child(label);
    return v_flex().w_full().p(5).bg(t.popover).text_size(12.8)
      .child(div().px(8).pt(4).pb(6).text_size(11).text_color(t.dim).truncate().child(path))
      .child(item("copy-absolute", "Copy absolute path", absolute))
      .child(item("copy-relative", "Copy relative path", path));
  }

  launch() {
    const path = pathTo(layout(), "herdr-files");
    if (path) {
      path.forEach((node, i) => {
        if (i > 0) run("layout.show", { id: node.id });
        if (node.type === "stack") run("layout.select", { stack: node.id, child: path[i + 1].id });
      });
      setShared("herdr-files.search", Date.now());
    } else notify({ title: "The Files pane is not in this layout", level: "warning" });
    closeOverlay(props().overlay);
  }

  find(cx) {
    const query = this.search.value().trim(), seq = ++this.searchSeq, context = this.context;
    this.query = query; this.searchKey = context?.key; this.searchError = null; this.missing = false; this.waiting = false;
    if (!query || !context?.cwd) { this.results = []; this.truncated = false; this.timedOut = false; this.searching = false; cx.notify(); return; }
    if (context.on) { this.results = []; this.searchError = "Search works only for folders on this computer"; cx.notify(); return; }
    this.searching = true; this.waiting = true; cx.notify();
    cx.timer.after(SEARCH_DELAY, cx => { if (seq === this.searchSeq) this.searchLatest(cx); });
  }

  searchLatest(cx) {
    if (this.running || !this.waiting) return;
    this.running = true; this.waiting = false;
    const seq = this.searchSeq, query = this.query, context = this.context, contents = this.mode === "contents";
    cx.spawn(async cx => {
      try {
        const excludes = this.exclude.value().split(",").map(p => p.trim()).filter(Boolean).flatMap(p => ["--exclude", p]);
        const limit = contents ? ["--timeout", String(GREP_TIMEOUT)] : [];
        const r = await this.runSearch([contents ? "grep" : "files", "--cwd", context.cwd, ...excludes, ...limit, "--", query]);
        if (seq !== this.searchSeq) return;
        if (r.code !== 0) throw new Error(r.stderr.trim() || `herdr-files exited with ${r.code}`);
        const out = JSON.parse(r.stdout);
        this.results = contents ? grouped(out.results) : out.results.map(hit => ({ key: hit.path, path: hit.path }));
        this.hits = out.results.length; this.truncated = out.truncated; this.timedOut = Boolean(out.timed_out);
        this.target = contents ? null : { line: out.line, column: out.column };
        this.at = contents ? 1 : 0; this.list.scroll_to_item(0);
      } catch (e) { if (seq === this.searchSeq) { this.results = []; this.missing = Boolean(e.missing); this.searchError = e.missing ? null : e.message ?? String(e); } }
      finally {
        this.running = false;
        if (seq === this.searchSeq) { this.searching = false; cx.notify(); }
        else this.searchLatest(cx);
      }
    });
  }

  async runSearch(args) {
    const missing = e => /cannot (start|run) .*: No such file or directory/.test(String(e));
    for (const bin of this.bin ? [this.bin] : [`${appHome()}/mods/herdr-files/bin/herdr-files`, "herdr-files"]) {
      try {
        const r = await exec(bin, args);
        if (r.code === 127 && missing(r.stderr)) continue;
        this.bin = bin;
        return r;
      } catch (e) { if (!missing(e)) throw e; }
    }
    this.bin = null;
    throw Object.assign(new Error("The herdr-files program is not installed"), { missing: true });
  }

  open(item, cx) {
    if (item) this.show(item.path, cx, true, item.line != null ? item : this.target);
  }

  setMode(mode, cx) {
    this.mode = mode; this.results = []; this.truncated = false; this.timedOut = false;
    this.find(cx);
  }

  keyDown(e, cx) {
    const n = this.results.length;
    if ((e.keystroke === "up" || e.keystroke === "down") && this.results.some(r => r.count == null)) {
      do this.at = (this.at + (e.keystroke === "up" ? n - 1 : 1)) % n; while (this.results[this.at].count != null);
      this.list.scroll_to_item(this.at);
      cx.notify();
    } else if (e.keystroke === "tab") {
      this.exclude.focus();
    } else if (e.keystroke === "escape" && this.search.value()) {
      this.search.set_value("");
      this.find(cx);
    } else return;
    cx.stop_propagation();
  }

  missingHelp(t) {
    const bin = `${appHome()}/mods/herdr-files/bin`, h = home();
    const shown = bin.startsWith(`${h}/`) ? `~${bin.slice(h.length)}` : bin;
    const link = (id, label, onClick) => div().id(id).text_color(t.accent).cursor_pointer().hover(s => s.text_color(t.foreground))
      .on_click((_e, cx) => onClick(cx)).child(label);
    return v_flex().flex_1().min_h(0).p(14).gap(8).text_size(12).text_color(t.dim)
      .child(div().child("Search needs the herdr-files program."))
      .child(div().child(`Put it in ${shown}/ or on your PATH.`))
      .child(h_flex().gap(14)
        .child(link("files-copy-bin", "Copy folder path", cx => { cx.write_to_clipboard(bin); notify({ title: "Copied path", body: bin, level: "success" }); }))
        .child(link("files-install", "How to install", cx => cx.open_url(INSTALL_URL))));
  }

  resultList(t) {
    if (this.missing) return this.missingHelp(t);
    const contents = this.mode === "contents";
    const status = this.searchError || (this.searching && !this.results.length ? "Searching…" : !this.results.length ? (contents ? "No matching lines" : "No matching files") : null);
    return v_flex().flex_1().min_h(0)
      .when(Boolean(status), el => el.child(div().p(14).text_size(12).text_color(t.dim).child(status)))
      .when(this.results.length > 0, el => el.child(div().flex_1().min_h(0).relative()
        .child(v_virtual_list("files-results", this.results.length, 28, i => this.results[i].key,
          range => this.results.slice(range.start, range.end).map((item, i) => this.resultRow(item, range.start + i === this.at, t))).size_full().p(6)
          .track_scroll(this.list)
          .on_item_click((key, cx) => this.open(this.results.find(r => r.key === key), cx)))
        .child(Scrollbar.vertical("files-results").absolute().inset_0())))
      .when(this.truncated || this.timedOut, el => el.child(div().px(14).py(6).text_size(11).text_color(t.dim).child(
        this.timedOut ? `Stopped after ${GREP_TIMEOUT / 1000} s · ${this.hits} lines found` : contents ? `Showing the first ${this.hits} lines` : `Showing the best ${this.hits} matches`)));
  }

  resultRow(item, selected, t) {
    const row = h_flex().id(`result-${item.key}`).h(28).px(8).gap(7).items_center().rounded(7).cursor_pointer().when(selected, el => el.bg(t.hover)).hover(s => s.bg(t.hover));
    if (item.line != null) return row.pl(29)
      .child(div().flex_none().min_w(24).text_size(11).text_color(t.dim).child(String(item.line)))
      .child(div().flex_1().min_w(0).truncate().text_size(12).font_family(settings().font_family).child(item.text.trimStart()));
    const slash = item.path.lastIndexOf("/");
    return row
      .child(svg("mods/herdr-files/file.svg").size(14).flex_none().text_color(t.muted))
      .child(div().flex_none().max_w_full().truncate().child(item.path.slice(slash + 1)))
      .child(div().flex_1().min_w(0).truncate().text_size(11).text_color(t.dim).child(slash < 0 ? "" : item.path.slice(0, slash)))
      .when(item.count != null, el => el.child(div().flex_none().text_size(11).text_color(t.dim).child(String(item.count))));
  }

  searchBox(t) {
    const field = (id, state) => div().id(id).min_w(0).h(30).px(10).flex().items_center().rounded(8).border_1().border_color(this.typing === id ? t.accent : t.border).cursor_text()
      .child(Input.new(state).h_full().text_size(12).text_color(t.foreground));
    const tab = (mode, label) => div().id(`files-mode-${mode}`).h(22).px(8).flex().items_center().rounded(6).cursor_pointer().text_size(11.5)
      .text_color(this.mode === mode ? t.foreground : t.dim).when(this.mode === mode, el => el.bg(t.hover)).hover(s => s.text_color(t.foreground))
      .on_click((_e, cx) => { this.search.focus(); if (this.mode !== mode) this.setMode(mode, cx); })
      .child(label);
    return v_flex().flex_none().p(6).gap(6).border_b(1).border_color(t.border)
      .child(h_flex().gap(4).child(tab("names", "Names")).child(tab("contents", "Contents")))
      .child(field("files-search", this.search).on_key_down((e, cx) => this.keyDown(e, cx)))
      .child(field("files-exclude", this.exclude).on_key_down((e, cx) => {
        if (e.keystroke !== "shift-tab") return;
        this.search.focus();
        cx.stop_propagation();
      }));
  }

  render(cx) {
    if (props()?.menu) return this.menu(theme());
    if (props()?.launch) return div();
    if (shared("herdr-files.search") !== this.asked) {
      this.asked = shared("herdr-files.search");
      const again = this.typing === "files-search";
      if (this.file) this.show(null);
      cx.timer.after(0, cx => {
        if (again) this.setMode(this.mode === "names" ? "contents" : "names", cx);
        this.search.focus();
        if (!again) this.search.set_selected_range({ start: 0, end: this.search.value().length });
      });
    }
    if (key() !== this.context?.key) this.refresh(cx);
    if (this.query && this.searchKey !== this.context?.key) this.find(cx);
    const t = theme();
    return v_flex().id("files").track_focus(this.handle).size_full().min_h(0).rounded(10).overflow_hidden().bg(t.background).border_1().border_color(t.border).text_color(t.foreground).text_size(13)
      .child(h_flex().h(42).flex_none().px(14).gap(8).items_center().border_b(1).border_color(t.border)
        .when(Boolean(this.file), el => el.child(div().id("files-back").w(20).cursor_pointer().text_color(t.muted).on_click((_e, cx) => { this.show(null); cx.notify(); }).child("←")))
        .child(div().font_semibold().child("Files"))
        .child(div().flex_1().min_w(0).truncate().text_right().text_size(12).text_color(t.dim).child(this.file ?? this.root?.split(/[\\/]/).pop() ?? "")))
      .when(!this.file, el => el.child(this.searchBox(t)))
      .when(!this.file && Boolean(this.query), el => el.child(this.resultList(t)))
      .when(!this.file && !this.query, el => el.child(v_flex().id("files-tree").flex_1().min_h(0).overflow_y_scrollbar().p(6).children(this.rows("", 0, [], t))
        .when(Boolean(this.error) || !this.entries.get("")?.length, el => el.child(div().p(8).text_size(12).text_color(t.dim).child(this.error || (this.loading ? "Loading files…" : this.root ? "No files" : "No folder"))))
        .when(this.limited, el => el.child(div().p(8).text_size(11).text_color(t.dim).child("File list limited to 500 entries per folder and 5,000 total")))))
      .when(Boolean(this.file), el => el.child(this.code ? Editor.new(this.code).flex_1().min_h(0) : div().p(12).text_color(t.dim).child(this.fileError || "Loading file…")));
  }
}
