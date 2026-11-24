use std::path::Path;

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;

pub fn files(root: &Path, excludes: &[String]) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    each_file(root, excludes, |path| {
        paths.push(path);
        true
    })?;
    paths.sort();
    Ok(paths)
}

pub fn each_file(
    root: &Path,
    excludes: &[String],
    mut visit: impl FnMut(String) -> bool,
) -> Result<(), String> {
    let mut overrides = OverrideBuilder::new(root);
    for pattern in std::iter::once(".git").chain(excludes.iter().map(String::as_str)) {
        overrides
            .add(&format!("!{pattern}"))
            .map_err(|e| format!("--exclude {pattern}: {e}"))?;
    }
    let overrides = overrides.build().map_err(|e| e.to_string())?;

    let walk = WalkBuilder::new(root)
        .standard_filters(false)
        .overrides(overrides)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();
    for entry in walk {
        let Ok(entry) = entry else { continue };
        if is_folder(&entry) {
            continue;
        }
        if let Some(path) = entry.path().strip_prefix(root).ok().and_then(Path::to_str)
            && !visit(path.to_owned())
        {
            break;
        }
    }
    Ok(())
}

fn is_folder(entry: &ignore::DirEntry) -> bool {
    entry.file_type().is_some_and(|t| t.is_dir())
        || (entry.path_is_symlink() && entry.path().is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_file_stops_when_visit_says_so() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut seen = 0;
        each_file(root, &[], |_| {
            seen += 1;
            false
        })
        .unwrap();
        assert_eq!(seen, 1);
    }
}
