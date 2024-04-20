pub mod data_layer;
pub mod error;
pub mod models;

use std::{future::Future, path::Path};

use async_recursion::async_recursion;
use lazy_static::lazy_static;

use data_layer::*;
use error::*;

use crate::time_provider::TimeProvider;

lazy_static! {
    ///
    /// The base path for the operating system currently being used.
    /// "C:" for windows, "" for linux
    /// 
    static ref BASE_PATH: &'static str = {
        let os = std::env::consts::OS;
        match os {
            "windows" => "C:",
            "linux" => "",
            _ => panic!("Unsupported operating system")
        }
    };
}

pub enum FileStatus<'a> {
    NeedsBackup { sub_dir_id: i64, file_id: i64, file_name: &'a str },
    DoesNotNeedBackup,
}

/// 
/// Provides implementation for accessing file backup, 
/// previously generated hashes and more.
/// 
pub trait HistoryService {
    ///
    /// Retrieves backup status of a file, given a `path` and new file `hsh`.
    /// A file either needs to be backed up 
    /// (whether newly being added to the repo or already existing, but with a different hash),
    /// or has a matching `hsh` to the provided one, in which case a new 
    /// backup is not required.
    /// 
    fn get_file_status<'a>(&mut self, path: &'a Path, hsh: &str) -> impl Future<Output = Result<FileStatus<'a>>> + Send;
    ///
    /// Adds a new file and hash to the `BackupService` with the provided information.
    /// Returns the ID of the oldest entry if the # of copies surpasses the total desired backup count.
    /// 
    fn create_file_entry(&self, dir_id: i64, file_id: i64, file_name: &str, hsh: &str) -> impl Future<Output = Result<Option<i64>>> + Send;
    ///
    /// Filters all newest files by whether they have been updated since the 
    /// service has began running. If not, the files are marked as deleted
    /// 
    fn mark_all_deleted_files(&self) -> impl Future<Output = Result<()>> + Send;
}

pub struct FileHistoryService<'a> {
    data_layer: &'a dyn DataLayer,
    time_provider: &'a dyn TimeProvider,
    next_file_id: i64,
    max_copies: i32
}
impl<'a> HistoryService for FileHistoryService<'a> {
    async fn get_file_status<'b>(&mut self, path: &'b Path, hsh: &str) -> Result<FileStatus<'b>> {
        let paths = path.iter().map(|p| p.to_str().unwrap());
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let sub_dir_id = self.traverse_to_subdir(paths, true).await?.unwrap();

        let latest_hsh = self.data_layer.get_latest_file(sub_dir_id, file_name).await?
            .and_then(|f| f.hsh);

        if let Some(latest_hsh) = latest_hsh {
            if latest_hsh == hsh {
                self.data_layer.update_latest_hsh_ts(
                    sub_dir_id, file_name, self.time_provider.naive_utc_start()
                ).await?;
                return Ok(FileStatus::DoesNotNeedBackup);
            }
        }

        let file_id = self.next_file_id; 
        self.next_file_id += 1;

        Ok(FileStatus::NeedsBackup { sub_dir_id, file_id, file_name })
    }
    async fn create_file_entry(&self, dir_id: i64, file_id: i64, file_name: &str, hsh: &str) -> Result<Option<i64>> {
        self.data_layer.create_file_entry(dir_id, file_id, file_name, hsh, self.time_provider.naive_utc_start()).await?;
        let files = self.data_layer.get_dir_files(dir_id, file_name).await?;
        return if files.len() as i32 > self.max_copies {
            let file_id = files.iter().min_by_key(|f| f.backup_ts).unwrap().id;
            self.data_layer.delete_file_entry(file_id).await?;
            Ok(Some(file_id))
        } else {
            Ok(None)
        }
    }
    async fn mark_all_deleted_files(&self) -> Result<()> {
        self.data_layer.mark_all_deleted_files(self.time_provider.naive_utc_start()).await?;
        Ok(())
    }
}
impl<'a> FileHistoryService<'a> {
    pub async fn new(
        data_layer: &'a dyn DataLayer, time_provider: &'a dyn TimeProvider, max_copies: i32
    ) -> Result<Self> {
        Ok(Self { 
            data_layer, 
            time_provider,
            next_file_id: data_layer.get_max_file_id().await? + 1,
            max_copies
        })
    }
    
    #[async_recursion]
    async fn traverse_to_subdir<'b>(
        &self, 
        path: impl Iterator<Item = &'b str> + Send + 'async_recursion,
        create_dirs: bool
    ) -> Result<Option<i64>> {
        // Convert the path to a peekable iterator
        let mut path = path.peekable();
        // Attempt to retrieve the root path from the data layer.
        // If it does not exist, no rows exist in the database
        let root_dir = path.next().unwrap();
        let mut cur_dir_id = self.data_layer.get_dir(root_dir).await?.and_then(|d| Some(d.id));
        if let (None, true) = (cur_dir_id, create_dirs) {
            cur_dir_id = Some(self.data_layer.create_dir(root_dir, None).await?);
        }

        while let (Some(sub_path), Some(dir_id)) = (path.next(), cur_dir_id) {
            // If there are no more values in the iterator after popping
            // off the last element, return the sub-directory ID
            if path.peek().is_none() { return Ok(Some(dir_id)); } 
            // Otherwise, continue to traverse down the path
            cur_dir_id = self.data_layer.get_sub_dirs(dir_id).await?.into_iter()
                .filter(|d| d.dir_name == sub_path).next().and_then(|d| Some(d.id));

            if let (None, true) = (cur_dir_id, create_dirs) {
                cur_dir_id = Some(self.data_layer.create_dir(sub_path, Some(dir_id)).await?);
            }
        };

        Ok(None)
    }
}

/*#[cfg(test)] 
mod tests {
    use std::{path::PathBuf, str::FromStr};

    use chrono::{TimeZone, Utc};
    use mockall::predicate::eq;

    use crate::{history_service::{models::DirModel, HistoryService, FileHistoryService, MockDataLayer, BASE_PATH}, time_provider::MockTimeProvider};

    use super::models::FileModel;

    fn build_mock_time_provider() -> MockTimeProvider {
        let mut mock_time_provider = MockTimeProvider::new();
        mock_time_provider.expect_naive_utc_now()
            .returning(|| Utc.with_ymd_and_hms(2023, 5, 10, 0, 0, 0).unwrap().naive_utc());

        mock_time_provider
    }
    fn build_mock_data_layer() -> MockDataLayer {
        let mut mock_dl = MockDataLayer::new();    
        mock_dl.expect_get_dir().with(eq(BASE_PATH.to_string()))
            .returning(|_| Ok(Some(DirModel { id: 1, dir_name: BASE_PATH.to_string(), parent_dir_id: None })));

        mock_dl.expect_get_sub_dirs().with(eq(1))
            .returning(|_| Ok(vec![DirModel { id: 2, dir_name: "path".to_string(), parent_dir_id: Some(1) },]));

        mock_dl.expect_get_sub_dirs().with(eq(2))
            .returning(|_| Ok(vec![
                DirModel { id: 3, dir_name: "path2".to_string(), parent_dir_id: Some(2) },
                DirModel { id: 4, dir_name: "path3".to_string(), parent_dir_id: Some(2) },
            ]));

        mock_dl.expect_get_dir_files().with(eq(3))
            .returning(|_| Ok(vec![
                FileModel { file_name: "entry1".to_string(), backup_ts: Utc.with_ymd_and_hms(2023, 5, 11, 14, 0, 8).unwrap().naive_local(), update_ts: Utc.with_ymd_and_hms(2023, 5, 11, 14, 0, 8).unwrap().naive_local(),  hsh: Some("hash1".to_string()) },
                FileModel { file_name: "entry1".to_string(), backup_ts: Utc.with_ymd_and_hms(2023, 5, 10, 14, 0, 8).unwrap().naive_local(), update_ts: Utc.with_ymd_and_hms(2023, 5, 10, 14, 0, 8).unwrap().naive_local(), hsh: Some("hash2".to_string()) },
                FileModel { file_name: "entry2".to_string(), backup_ts: Utc.with_ymd_and_hms(2023, 5, 10, 14, 0, 12).unwrap().naive_local(), update_ts: Utc.with_ymd_and_hms(2023, 5, 10, 14, 0, 12).unwrap().naive_local(), hsh: Some("hash3".to_string()) }
            ]));

        mock_dl
    }

    #[tokio::test] 
    async fn test_get_file_hsh_with_one_subdir_and_entry() {
        let mock_dl = build_mock_data_layer();
        let mock_tp = build_mock_time_provider();
        let svc = FileHistoryService::new(&mock_dl, &mock_tp).await.unwrap();

        let path = PathBuf::from_str(&format!("{}/path/entry1", BASE_PATH.to_string())).unwrap();
        let hsh = svc.get_file_status(&path).await.unwrap();

        assert_eq!(hsh, Some(Some("hash".to_string())));
    }

    #[tokio::test]
    async fn test_get_file_hsh_with_multi_subdirs() {
        let mock_dl = build_mock_data_layer();
        let mock_tp = build_mock_time_provider();
        let svc = FileHistoryService::new(&mock_dl, &mock_tp).await.unwrap();

        let path = PathBuf::from_str(&format!("{}/path/path3/entry2", BASE_PATH.to_string())).unwrap();
        let hsh = svc.get_file_status(&path).await.unwrap();
        assert_eq!(hsh, Some(Some("hash3".to_string())));
    }

    #[tokio::test]
    async fn test_cache_selects_most_recent_file() {
        let mock_dl = build_mock_data_layer();
        let mock_tp = build_mock_time_provider();
        let svc = FileHistoryService::new(&mock_dl, &mock_tp).await.unwrap();

        let path = PathBuf::from_str(&format!("{}/path/path2/entry2", BASE_PATH.to_string())).unwrap();
        let hsh = svc.get_file_status(&path).await.unwrap();
        assert_eq!(hsh, Some(Some("hash3".to_string())));

        let path = PathBuf::from_str(&format!("{}/path/path2/entry1", BASE_PATH.to_string())).unwrap();
        let hsh = svc.get_file_status(&path).await.unwrap();
        assert_eq!(hsh, Some(Some("hash1".to_string())));
    }
}*/