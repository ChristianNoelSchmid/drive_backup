pub mod error;

use std::path::PathBuf;

use async_stream::stream;
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::Stream;
use lazy_static::lazy_static;
use tokio::{io::AsyncReadExt, sync::Semaphore, task::JoinSet};

use error::*;

lazy_static! {
    static ref POOL: Semaphore = Semaphore::new(num_cpus::get());
}

///
/// Generates a collection of MD5 hashes for all files provided with the given PathBufs
/// Returns mapped with the path to the file.
/// 
pub fn gen_hashes(file_paths: impl Iterator<Item = PathBuf>) -> impl Stream<Item = Result<(PathBuf, String)>> {
    // Create an async Stream
    stream! {
        // All tasks joined together at the end of the process
        let mut tasks = JoinSet::new();
        // For every PathBuf found, if that PathBuf is a file, generate
        // a new task to create an MD5 hash for it, to be returned
        for path in file_paths {
            tasks.spawn(hash_file_path(path));
        }

        // Yield each PathBuf/MD5 hash generated from the tasks spawned above
        while let Some(cx) = tasks.join_next().await {
            yield Ok(cx??);
        }
    }
}

///
/// Generates an MD5 hash for the given file, found at the given PathBuf
/// 
async fn hash_file_path(path: PathBuf) -> Result<(PathBuf, String)> {
    // Get a lock on the static semaphore
    let _permit = POOL.acquire().await.unwrap();

    // The MD5 hash, generated over time while the file is being
    // asynchronously processed
    let mut md5_ctx = md5::Context::new();
    // Buffer for the current bytes being read from the file
    let mut bytes = [0u8;1024];

    // Open the file, and create a buffered reader to read the contents
    let file = tokio::fs::File::open(&path).await?;
    let mut file_reader = tokio::io::BufReader::new(file);

    // Loop until the end of the file has been reached, adding the read bytes
    // to the MD5 hash
    loop {
        match file_reader.read(&mut bytes).await {
            Ok(0) => break,
            Ok(_) => {
                md5_ctx.consume(bytes);
            },
            // TODO - add tracing error here
            Err(e) => panic!("{:?}", e)
        }
    }
    let hash = md5::compute(bytes).0;

    Ok((path, STANDARD.encode(hash)))
}