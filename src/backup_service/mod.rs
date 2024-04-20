pub mod error;

use std::{io::{BufWriter, Write}, path::{Path, PathBuf}};

use flate2::{write::GzEncoder, Compression};
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::bytes::BytesMut;

use self::error::*;

pub trait BackupService {
    fn backup_data(&mut self, id: i64, path: &Path) -> impl std::future::Future<Output = Result<()>> + Send;
    fn delete_backup(&mut self, id: i64) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub struct FileBackupService { 
    backup_file_path: PathBuf 
}

impl FileBackupService {
    pub fn new(backup_file_path: String) -> Self { 
        Self { backup_file_path: PathBuf::from(backup_file_path) }
    }
}

impl BackupService for FileBackupService {
    async fn backup_data(&mut self, id: i64, path: &Path) -> Result<()> {
        let from_file = tokio::fs::OpenOptions::new().read(true).open(path).await?;
        let mut from_file = BufReader::new(from_file);

        let mut to_file = PathBuf::from(self.backup_file_path.clone());
        to_file.push(&format!("{}", id / 100_000));
        tokio::fs::create_dir_all(&to_file).await?;
        to_file.push(&format!("{}.gz", id));

        let to_file = BufWriter::new(std::fs::OpenOptions::new().write(true).create(true).open(to_file)?);
        let mut gz = GzEncoder::new(to_file, Compression::best());

        let mut bytes = BytesMut::with_capacity(1024);
        while from_file.read_buf(&mut bytes).await? > 0 {
            gz.write_all(&bytes[..])?;
            bytes.clear();
        }

        Ok(())
    }
    async fn delete_backup(&mut self, id: i64) -> Result<()> {
        let mut file_path = PathBuf::from(self.backup_file_path.clone());
        file_path.push(&format!("{}", id / 100_000));
        tokio::fs::create_dir_all(&file_path).await?;
        file_path.push(&format!("{}.gz", id));

        Ok(tokio::fs::remove_file(file_path).await?)
    }
}