//! Lists the files under a folder: the file-walking half of `files`.

use std::path::Path;

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

/// Returns every file under `root` as a path relative to it, sorted.
///
/// The list matches the file tree: ignored and hidden files are in it, and
/// only `.git` is left out. `excludes` are gitignore-style patterns such as
/// `target/` or `*.log`; files and folders that match one are left out too.
pub fn files(root: &Path, excludes: &[String]) -> Result<Vec<String>, String> {
    let mut overrides = OverrideBuilder::new(root);
    for pattern in std::iter::once(".git").chain(excludes.iter().map(String::as_str)) {
        // In an override, a leading `!` means "leave out", not "keep".
        overrides
            .add(&format!("!{pattern}"))
            .map_err(|e| format!("--exclude {pattern}: {e}"))?;
    }
    let overrides = overrides.build().map_err(|e| e.to_string())?;

    let mut paths = Vec::new();
    // Without the standard filters, .gitignore rules and hidden names don't
    // hide anything.
    let walk = WalkBuilder::new(root)
        .standard_filters(false)
        .overrides(overrides)
        .build();
    for entry in walk {
        // A folder that can't be read is skipped, as the tree skips it.
        let Ok(entry) = entry else { continue };
        if is_folder(&entry) {
            continue;
        }
        // JSON needs UTF-8, so other paths are left out.
        if let Some(path) = entry.path().strip_prefix(root).ok().and_then(Path::to_str) {
            paths.push(path.to_owned());
        }
    }
    paths.sort();
    Ok(paths)
}

/// True for folders and links to folders. The walk doesn't follow links, but
/// the tree shows a link to a folder as a folder.
fn is_folder(entry: &ignore::DirEntry) -> bool {
    entry.file_type().is_some_and(|t| t.is_dir())
        || (entry.path_is_symlink() && entry.path().is_dir())
}
