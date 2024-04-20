use chrono::NaiveDateTime;

pub struct CacheEntryModel {
    pub hsh: String,
    pub backup_ts: NaiveDateTime
}

#[derive(Clone, Debug)]
pub struct FileModel {
    pub version: i64,
    pub id: i64,
    pub file_name: String,
    pub backup_ts: NaiveDateTime,
    pub hsh: Option<String>
}

pub struct DirModel {
    pub id: i64,
    pub parent_dir_id: Option<i64>,
    pub dir_name: String,
}