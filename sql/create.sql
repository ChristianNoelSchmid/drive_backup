CREATE TABLE dirs (
    id INTEGER PRIMARY KEY NOT NULL,
    parent_dir_id INTEGER,
    dir_name TEXT NOT NULL,

    FOREIGN KEY (parent_dir_id) REFERENCES dirs (id)
);

CREATE TABLE files (
    id INTEGER PRIMARY KEY NOT NULL,
    /* The version this file was backed up with.
       used for backwards-compatibility with new versions */
    version INTEGER NOT NULL,
    /* Foreign Key to the single dir the file belongs to */
    dir_id INTEGER NOT NULL,
    /* The file's name. Unique to the directory, but not the table */
    file_name TEXT NOT NULL,
    /* The file's time of backup */
    backup_ts DATETIME NOT NULL,
    /* The MD5 hash of the file */
    hsh TEXT,

    FOREIGN KEY(dir_id) REFERENCES dirs (id) ON DELETE CASCADE
);

CREATE INDEX idx_dirs_path_name ON dirs(dir_name);
CREATE INDEX idx_entrs_file_name ON files(file_name);