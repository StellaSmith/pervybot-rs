use std::io::Read;

use async_process::Stdio;
use reqwest::Url;
use tokio::process::Command;

use crate::{
    hashing::{hash_url, UrlHash},
    STREAM_DIR,
};

use super::StreamData;

pub async fn download_info_json(url: Url) -> StreamData {
    let data = Command::new("yt-dlp")
        .args([
            // Don't download any tracks
            "--skip-download",
            // Format filtering
            "-f",
            "\"(bv[height<=720]+ba/bv[height<=720]*+ba/b[height<=720])[filesize<100M]\"",
            // Reject playlists
            "--no-playlist",
            // Write JSON to stdout
            "-j",
        ])
        .arg(url.to_string())
        .output()
        .await
        .unwrap()
        .stdout
        .bytes()
        .map(|b| b.unwrap())
        .collect::<Vec<_>>();

    serde_json::from_slice(&data).unwrap()
}
