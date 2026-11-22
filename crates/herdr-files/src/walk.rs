use std::path::Path;

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

pub fn files(root: &Path, excludes: &[String]) -> Result<Vec<String>, String> {
    let mut overrides = OverrideBuilder::new(root);
    for pattern in std::iter::once(".git").chain(excludes.iter().map(String::as_str)) {
        overrides
            .add(&format!("!{pattern}"))
            .map_err(|e| format!("--exclude {pattern}: {e}"))?;
    }
    let overrides = overrides.build().map_err(|e| e.to_string())?;

    let mut paths = Vec::new();
    let walk = WalkBuilder::new(root)
        .standard_filters(false)
        .overrides(overrides)
        .build();
    for entry in walk {
        let Ok(entry) = entry else { continue };
        if is_folder(&entry) {
            continue;
        }
        if let Some(path) = entry.path().strip_prefix(root).ok().and_then(Path::to_str) {
            paths.push(path.to_owned());
        }
    }
    paths.sort();
    Ok(paths)
}

fn is_folder(entry: &ignore::DirEntry) -> bool {
    entry.file_type().is_some_and(|t| t.is_dir())
        || (entry.path_is_symlink() && entry.path().is_dir())
}
