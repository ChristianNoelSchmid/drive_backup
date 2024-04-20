use async_trait::async_trait;
use chrono::NaiveDateTime;
use sqlx::SqlitePool;

#[cfg(test)]
use mockall::automock;
use tokio_stream::StreamExt; 

use super::models::{DirModel, FileModel};
use crate::data_layer_error::*;

const VERSION: i32 = 1;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait DataLayer : Send + Sync {
    /// 
    /// Gets the max ID in the file primary key column
    /// 
    async fn get_max_file_id(&self) -> Result<i64>;
    ///
    /// Retrieves the directory with the given `dir_name` from the `DataLayer`
    /// 
    async fn get_dir(&self, dir_name: &str) -> Result<Option<DirModel>>;
    ///
    /// Gets all sub-directories under the directory with the given `dir_id`
    /// 
    async fn get_sub_dirs(&self, dir_id: i64) -> Result<Vec<DirModel>>;
    ///
    /// Gets the latest updated file under the directory with the given `dir_id`, if it exists
    /// 
    async fn get_latest_file(&self, dir_id: i64, file_name: &str) -> Result<Option<FileModel>>;
    ///
    /// Gets all files with the provided `file_name` under the directory with the given `dir_id`
    /// 
    async fn get_dir_files(&self, dir_id: i64, file_name: &str) -> Result<Vec<FileModel>>;
    ///
    /// Creates a directory with the provided `dir_name`, and the given `parent_dir_id`
    /// for it's parent directory.
    /// 
    async fn create_dir(&self, dir_name: &str, parent_dir_id: Option<i64>) -> Result<i64>;
    ///
    /// Updates the file under the given `dir_id`, with the given `file_name` with a new `file_hash`,
    /// and update `ts`
    /// 
    async fn create_file_entry(&self, dir_id: i64, file_id: i64, file_name: &str, file_hsh: &str, ts: NaiveDateTime) -> Result<()>;
    ///
    /// Updates the latest file with the provided name with the provided timestamp
    /// 
    async fn update_latest_hsh_ts(&self, dir_id: i64, file_name: &str, ts: NaiveDateTime) -> Result<()>;
    ///
    /// Updates the `DataLayer` to mark all files not updated in the current process as
    /// deleted from the system.
    /// 
    async fn mark_all_deleted_files(&self, current_run_ts: NaiveDateTime) -> Result<()>;
    ///
    /// Deletes the file entry by `file_id`
    /// 
    async fn delete_file_entry(&self, file_id: i64) -> Result<()>;
}

pub struct DbDataLayer<'a> {
    db: &'a SqlitePool,
}

impl<'a> DbDataLayer<'a> {
    pub fn new(db: &'a SqlitePool) -> Self { 
        Self { db }
    }
}

#[async_trait]
impl<'a> DataLayer for DbDataLayer<'a> {
    async fn get_max_file_id(&self) -> Result<i64> {
        Ok(sqlx::query!("SELECT MAX(id) as max_id FROM files")
            .fetch_optional(self.db).await?.and_then(|r| r.max_id.and_then(|id| Some(id as i64))).unwrap_or(0))
    }
    async fn get_dir(&self, dir_name: &str) -> Result<Option<DirModel>> {
        Ok(sqlx::query_as!(DirModel,
            "SELECT id, parent_dir_id, dir_name FROM dirs WHERE dir_name = ?", dir_name
        )
            .fetch_optional(self.db).await?)
    }
    async fn get_sub_dirs(&self, dir_id: i64) -> Result<Vec<DirModel>> {
        Ok(sqlx::query_as!(DirModel,
            "SELECT id, parent_dir_id, dir_name FROM dirs WHERE parent_dir_id = ?", dir_id
        )
            .fetch_all(self.db).await?)
    }
    
    async fn get_latest_file(&self, dir_id: i64, file_name: &str) -> Result<Option<FileModel>> {
        Ok(sqlx::query_as!(FileModel, "
            SELECT version, id, file_name, backup_ts, hsh FROM files 
            WHERE dir_id = ? AND file_name = ?
            ORDER BY backup_ts DESC LIMIT 1
            ", dir_id, file_name
        )
            .fetch_optional(self.db).await?)

    } 
    async fn get_dir_files(&self, dir_id: i64, file_name: &str) -> Result<Vec<FileModel>> {
        Ok(sqlx::query_as!(FileModel, "
            SELECT version, id, file_name, backup_ts, hsh FROM files 
            WHERE dir_id = ? AND file_name = ?
            ", dir_id, file_name
        )
            .fetch_all(self.db).await?)
    }
    async fn create_dir(&self, dir_name: &str, parent_dir_id: Option<i64>) -> Result<i64> {
        Ok(sqlx::query!("INSERT INTO dirs (parent_dir_id, dir_name) VALUES (? ,?)", parent_dir_id, dir_name)
            .execute(self.db).await?.last_insert_rowid())
    }
    async fn create_file_entry(&self, dir_id: i64, file_id: i64, file_name: &str, file_hsh: &str, ts: NaiveDateTime) -> Result<()> {
        sqlx::query!(
            "INSERT INTO files (version, dir_id, id, file_name, backup_ts, hsh) VALUES (?, ?, ?, ?, ?, ?)",
            VERSION, dir_id, file_id, file_name, ts, file_hsh
        )
            .execute(self.db).await?;

        Ok(())
    }
    async fn update_latest_hsh_ts(&self, dir_id: i64, file_name: &str, ts: NaiveDateTime) -> Result<()> {
        let latest_id = sqlx::query!("SELECT id, MAX(backup_ts) as ts FROM files WHERE dir_id = ? and file_name = ?",
            dir_id, file_name
        ).fetch_one(self.db).await?.id.unwrap();

        sqlx::query!("UPDATE files SET backup_ts = ? WHERE id = ?", ts, latest_id)
            .execute(self.db).await?;

        Ok(())
    }
    async fn mark_all_deleted_files(&self, current_run_ts: NaiveDateTime) -> Result<()> {
        let mut rows = sqlx::query!(
            r#"SELECT MAX(backup_ts) as "max_ts!: NaiveDateTime", dir_id, file_name, hsh FROM files
             GROUP BY dir_id, file_name"#
        ).fetch(self.db);

        while let Some(row) = rows.next().await {
            let row = row?;
            if row.max_ts < current_run_ts {
                sqlx::query!(
                    "INSERT INTO files (version, dir_id, file_name, backup_ts, hsh)
                    VALUES (?, ?, ?, ?, NULL)",
                    VERSION, row.dir_id, row.file_name, current_run_ts
                ).execute(self.db).await.unwrap();
            }
        }

        Ok(())
    }
    async fn delete_file_entry(&self, file_id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM files WHERE id = ?", file_id).execute(self.db).await?;
        Ok(())
    }
}