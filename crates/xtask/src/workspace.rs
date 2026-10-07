//! Resolve the caller's Cargo workspace; a prebuilt runner must not jump to its build tree.
use std::{
    fs, io,
    path::{Path, PathBuf},
};
pub fn resolve(start: &Path) -> io::Result<PathBuf> {
    for candidate in start.ancestors() {
        let manifest = match fs::read_to_string(candidate.join("Cargo.toml")) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if manifest
            .lines()
            .any(|line| line.split('#').next().unwrap_or("").trim() == "[workspace]")
        {
            return Ok(candidate.to_path_buf());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no Cargo workspace above the caller directory",
    ))
}
