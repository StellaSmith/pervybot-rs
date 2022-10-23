use std::fs::File;
use std::io::Write;

use async_process::Command;
use reqwest::Client;
use serde_json::{de::from_reader, Value};
use url::Url;

use crate::hashing::{hash_url, UrlHash};
use crate::STREAM_DIR;

const MAX_VIDEO_SIZE: u64 = 1024 * 1024 * 100;
const MAX_VERTICAL_RESOLUTION: u64 = 720;

fn get_audio_url(data: &Value) -> Option<(String, String)> {
    if let Some(Value::Array(formats)) = data.get("formats") {
        let mut best = 0;
        let mut best_url = "";
        let mut best_ext = "";
        for format in formats {
            if let Some(Value::String(audio_ext)) = format.get("audio-ext") {
                if audio_ext != "none" {
                    if let (Some(Value::String(url)), Some(Value::Number(sample_rate))) =
                        (format.get("url"), format.get("asr"))
                    {
                        if sample_rate.as_u64().unwrap() > best
                            && sample_rate.as_u64().unwrap() <= 48000
                        {
                            best_ext = audio_ext;
                            best = sample_rate.as_u64().unwrap();
                            best_url = url;
                        }
                    }
                }
            }
        }
        if !best_url.is_empty() {
            Some((best_url.to_owned(), best_ext.to_owned()))
        } else {
            None
        }
    } else {
        None
    }
}

fn get_video_url(data: &Value) -> Option<(String, String)> {
    if let Some(Value::Array(formats)) = data.get("formats") {
        let mut best = 0;
        let mut best_url = "";
        let mut best_ext = "";
        for format in formats {
            if let Some(Value::String(video_ext)) = format.get("video-ext") {
                if video_ext != "none" {
                    if let (
                        Some(Value::String(url)),
                        Some(Value::Number(height)),
                        Some(Value::Number(size)),
                    ) = (
                        format.get("url"),
                        format.get("height"),
                        format.get("filesize"),
                    ) {
                        if height.as_u64().unwrap() > best
                            && height.as_u64().unwrap() <= MAX_VERTICAL_RESOLUTION
                            && size.as_u64().unwrap() <= MAX_VIDEO_SIZE
                        {
                            best_ext = video_ext;
                            best = height.as_u64().unwrap();
                            best_url = url;
                        }
                    }
                }
            }
        }
        if !best_url.is_empty() {
            Some((best_url.to_owned(), best_ext.to_owned()))
        } else {
            None
        }
    } else {
        None
    }
}

fn get_subs_urls(data: &Value) -> Vec<(String, String, String, String)> {
    let mut output = vec![];
    if let Some(Value::Object(subs)) = data.get("subtitles") {
        for (lang_code, subs) in subs {
            if let Value::Array(subs) = subs {
                if let Some(Value::Object(subs)) = subs.get(0) {
                    if let (
                        Some(Value::String(name)),
                        Some(Value::String(url)),
                        Some(Value::String(extension)),
                    ) = (subs.get("name"), subs.get("url"), subs.get("ext"))
                    {
                        output.push((
                            lang_code.to_owned(),
                            name.to_owned(),
                            url.to_owned(),
                            extension.to_owned(),
                        ))
                    }
                }
            }
        }
    }
    output
}

fn get_max_res_thumbnail_url(data: &Value) -> Option<String> {
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

fn get_stream_title(data: &Value) -> Option<String> {
    if let Some(Value::String(nym)) = data.get("fulltitle") {
        Some(nym.to_owned())
    } else if let Some(Value::String(nym)) = data.get("title") {
        Some(nym.to_owned())
    } else {
        None
    }
}

pub async fn download_stream(url_hash: UrlHash, req_client: Client) {
    let base_path = format!["{STREAM_DIR}/{}", url_hash.0];
    let json_path = format!["{base_path}/info.json"];
    let data: Value = from_reader(File::open(json_path).unwrap()).unwrap();
    let audio_url = get_audio_url(&data);
    let video_url = get_video_url(&data);
    let subs_urls = get_subs_urls(&data);
    let thumbnail_url = get_max_res_thumbnail_url(&data);
    let title = get_stream_title(&data);

    if let Some((url, extension)) = audio_url {
        if let Ok(res) = req_client.post(url).send().await {
            let mut file = File::create(format!["{base_path}/audio.{extension}"]).unwrap();
            file.write_all(&res.bytes().await.unwrap()).unwrap();
        }
    }
    if let Some((url, extension)) = video_url {
        if let Ok(res) = req_client.post(url).send().await {
            let mut file = File::create(format!["{base_path}/video.{extension}"]).unwrap();
            file.write_all(&res.bytes().await.unwrap()).unwrap();
        }
    }
    for (lang, name, url, extension) in subs_urls {
        
    }
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
        .args([
            "--no-playlist",
            "--write-info-json",
            "--skip-download",
            "-o",
        ])
        .arg("\"info.json\"")
        .arg(url.to_string())
        .output()
        .await
        .unwrap();
    url_hash
}
