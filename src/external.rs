use std::fs::File;

use async_process::Command;
use serde_json::{de::from_reader, Value};
use url::Url;

use crate::hashing::{hash_url, UrlHash};
use crate::STREAM_DIR;

fn get_max_res_thumbnail(data: &Value) -> Option<String> {
    if let Some(Value::String(url)) = data.get("thumbnail") {
        Some(url.into())
    } else if let Some(Value::Array(thumbnails)) = data.get("thumbnails") {
        let mut max_res_url = None;
        let mut webm_max_res_url = None;
        let mut highest_res: u64 = 0;
        let mut webm_highest_res: u64 = 0;
        for thumbnail in thumbnails {
            if let Some(Value::Number(height)) = thumbnail.get("height") {
                if let Some(Value::String(url)) = thumbnail.get("url") {
                    let height = height.as_u64().unwrap();
                    if url.contains("webm") {
                        if webm_highest_res < height {
                            webm_highest_res = height;
                            webm_max_res_url = Some(url);
                        }
                    } else if highest_res < height {
                        highest_res = height;
                        max_res_url = Some(url);
                    }
                }
            }
        }
        if highest_res >= webm_highest_res {
            max_res_url.map(|s| s.into())
        } else {
            webm_max_res_url.map(|s| s.into())
        }
    } else {
        None
    }
}

enum Format {
    Video { p: u32 },
    Audio,
    Subs,
}

fn get_format_url(data: &Value, desired_format: Format) {
    if let Some(Value::Array(formats)) = data.get("formats") {
        for format in formats {
            match desired_format {
                Format::Video { p } => {}
                Format::Audio => todo!(),
                Format::Subs => todo!(),
            }
        }
    }
}

pub async fn download_stream(url_hash: UrlHash) {
    let base_path = format!["{STREAM_DIR}{}/", url_hash.0];
    let json_path = format!["{base_path}/info.json"];
    let data: Value = from_reader(File::open(json_path).unwrap()).unwrap();
    let thumbnail_url = get_max_res_thumbnail(&data);
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
