use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub backup_globs: Vec<String>,
    pub backup_path: String,
    pub max_copies: i32
}