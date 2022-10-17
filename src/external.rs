use std::fs::File;

use async_process::Command;
use serde_json::{de::from_reader, Value};
use url::Url;

use crate::hashing::{hash_url, UrlHash};
use crate::STREAM_DIR;

pub async fn download_stream(url_hash: UrlHash) {
    let path = format!["{STREAM_DIR}{}", url_hash.0];
    let data: Value = from_reader(File::open(path).unwrap()).unwrap();
    
}

pub async fn download_info_json(url: Url) -> UrlHash {
    let url_hash = hash_url(url.clone());
    let path = format!["{STREAM_DIR}{}", url_hash.0];
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&path)
        .unwrap();
    Command::new("yt-dlp")
        .arg("-P")
        .arg(path)
        .args(["--no-playlist", "--write-json", "--skip-download", "-o"])
        .arg("\"info.json\"")
        .arg(url.to_string())
        .output()
        .await
        .unwrap();
    url_hash
}
