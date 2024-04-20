use glob::glob;
use std::path::PathBuf;

pub fn get_glob_files(glob_iter: impl Iterator<Item = String>) -> impl Iterator<Item = PathBuf> {
    // For every glob pattern given, generate iterators finding
    // each file that matches the pattern
    // TODO - add tracing for each unwrap
    glob_iter.flat_map(|glob_ptn| glob(&glob_ptn).unwrap()) 
        .map(|path| std::fs::canonicalize(path.unwrap()).unwrap())
        .filter(|path| !path.is_dir())
}