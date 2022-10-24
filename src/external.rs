use std::env::Args;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::prelude::AsRawFd;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

// use async_process::Command;
use isolang::Language;
use reqwest::Client;
use serde_json::{de::from_reader, Value};
use serenity::futures::StreamExt;
use tokio::sync::mpsc::channel;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_command_fds::{CommandFdExt, FdMapping};
use url::Url;

use crate::hashing::{hash_url, UrlHash};
use crate::STREAM_DIR;

const MAX_VIDEO_SIZE: u64 = 1024 * 1024 * 100;
const MAX_VERTICAL_RESOLUTION: u64 = 720;

fn get_audio_url(data: &Value) -> Option<String> {
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
            Some(best_url.to_owned())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_video_url(data: &Value) -> Option<String> {
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
            Some(best_url.to_owned())
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
    let title = get_stream_title(&data).unwrap_or_else(|| "unkown".into());

    let (kill_switch, mut kill_signal) = channel::<()>(1);

    let threadify_dl_stream = |maybe_url: Option<String>, fallible: bool| {
        if let Some(url) = maybe_url {
            let client = req_client.clone();
            let (reader, mut writer) = tokio_pipe::pipe().unwrap();
            let kill_switch = kill_switch.clone();
            thread::spawn(move || async move {
                let mut stream = client.get(url).send().await.unwrap().bytes_stream();
                while let Some(item) = stream.next().await {
                    if let Ok(bytes) = item {
                        writer.write_all(&bytes).await.unwrap();
                    } else if !fallible {
                        kill_switch.send(()).await.unwrap();
                        break;
                    }
                }
            });
            Some(reader)
        } else {
            None
        }
    };

    let audio_pipe = threadify_dl_stream(audio_url, false);
    let video_pipe = threadify_dl_stream(video_url, false);
    let thumbnail_pipe = threadify_dl_stream(thumbnail_url, true);
    let mut subs_pipes = Vec::with_capacity(subs_urls.len());
    for (lang, name, url, extension) in subs_urls {
        if let Some(pipe) = threadify_dl_stream(Some(url), true) {
            subs_pipes.push((
                pipe,
                name,
                Language::from_639_1(&lang)
                    .unwrap_or_else(|| Language::from_639_3(&lang).unwrap_or_default()),
                extension,
            ));
        }
    }

    let mut has_video = false;
    let ext = if video_pipe.is_some() {
        has_video = true;
        "mkv"
    } else {
        if audio_pipe.is_none() {
            return;
        }
        "mka"
    };

    let output_path = format!["{base_path}/{title}.{ext}"];

    let mut cmd = Command::new("ffmpeg");

    let mut fds_to_map = Vec::with_capacity(
        subs_pipes.len()
            + if has_video {
                1 + if audio_pipe.is_some() { 1 } else { 0 }
            } else {
                1
            },
    );

    if let Some(video_pipe) = video_pipe {
        let video_fd = video_pipe.as_raw_fd();
        cmd.arg("-i")
            .arg(format!["pipe:{video_fd}"])
            .args(["-map", "0:v"]);
        fds_to_map.push(video_fd);
    }

    if let Some(audio_pipe) = audio_pipe {
        let audio_fd = audio_pipe.as_raw_fd();
        cmd.arg("-i")
            .arg(format!["pipe:{audio_fd}"])
            .args(["-map", "1:a"]);
        fds_to_map.push(audio_fd);
    }

    for (i, (pipe, name, lang, extension)) in subs_pipes.iter().enumerate() {
        let subs_fd = pipe.as_raw_fd();
        cmd.arg("-f")
            .arg(extension)
            .arg("-i")
            .arg(format!["pipe:{subs_fd}"])
            .arg("-map")
            .arg(format!["{}:s", i])
            .arg(format!["-metadata:s:s:{}", i])
            .arg(format!["language={}", lang.to_639_3()])
            .arg(format!["-metadata:s:s:{}", i])
            .arg(format!["title={}", name]);
        fds_to_map.push(subs_fd);
    }

    if let Some(thumbnail_pipe) = thumbnail_pipe {
        let thumbnail_fd = thumbnail_pipe.as_raw_fd();
        cmd.arg("-i").arg(format!["pipe:{thumbnail_fd}"]);
        fds_to_map.push(thumbnail_fd);
    }

    if has_video {
        cmd.args(["-r", "30"]);
    }

    cmd.arg(output_path);

    let fd_mappings: Vec<_> = fds_to_map
        .iter()
        .map(|n| FdMapping {
            parent_fd: *n,
            child_fd: *n,
        })
        .collect();

    cmd.fd_mappings(fd_mappings).unwrap();

    let mut child = cmd.spawn().unwrap();

    tokio::select! {
        _ = child.wait() => {}
        _ = kill_signal.recv() => child.kill().await.expect("kill failed")
    }
}

// (bv[height<=720]+ba/bv[height<=720]*+ba/b[height<=720])[filesize<100M]

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
