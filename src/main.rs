use std::env;

use drive_backup::{backup_service::{BackupService, FileBackupService}, config::Config, file_svc::get_glob_files, hash_svc::gen_hashes, history_service::{self, FileHistoryService, FileStatus, HistoryService}, time_provider::CoreTimeProvider};
use futures_util::{pin_mut, StreamExt};
use lazy_static::lazy_static;
use sqlx::sqlite::SqlitePoolOptions;

lazy_static! {
    static ref CONFIG: Config = 
        serde_json::from_str(&std::fs::read_to_string("config.json").unwrap())
            .unwrap();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let paths = get_glob_files(CONFIG.backup_globs.clone().into_iter());
    let hashes = gen_hashes(paths); 

    let db = SqlitePoolOptions::new().connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();
    let time_provider = CoreTimeProvider::new();

    let data_layer = history_service::data_layer::DbDataLayer::new(&db);
    let mut cache_svc = FileHistoryService::new(&data_layer, &time_provider, CONFIG.max_copies).await.unwrap();

    let mut backup_service = FileBackupService::new(CONFIG.backup_path.to_string());

    pin_mut!(hashes);
    while let Some(Ok((path, hsh))) = hashes.next().await {
        let status = cache_svc.get_file_status(&path, &hsh).await.unwrap();
        if let FileStatus::NeedsBackup { sub_dir_id, file_id, file_name } = status {
            backup_service.backup_data(file_id, &path).await.unwrap();
            if let Some(id) = cache_svc.create_file_entry(sub_dir_id, file_id, &file_name, &hsh).await.unwrap() {
                backup_service.delete_backup(id).await.unwrap();
            }
        }
    }

    cache_svc.mark_all_deleted_files().await.unwrap();
}
