use std::env::Args;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::prelude::AsRawFd;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

// use async_process::Command;
use isolang::Language;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{de::from_reader, Value};
use serenity::futures::StreamExt;
use tokio::sync::mpsc::channel;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_command_fds::{CommandFdExt, FdMapping};
use url::Url;

use crate::hashing::{hash_url, UrlHash};
use crate::STREAM_DIR;

// fn get_audio_url(data: &Value) -> Option<String> {
//     if let Some(Value::Array(formats)) = data.get("formats") {
//         let mut best = 0;
//         let mut best_url = "";
//         let mut best_ext = "";
//         for format in formats {
//             if let Some(Value::String(audio_ext)) = format.get("audio-ext") {
//                 if audio_ext != "none" {
//                     if let (Some(Value::String(url)), Some(Value::Number(sample_rate))) =
//                         (format.get("url"), format.get("asr"))
//                     {
//                         if sample_rate.as_u64().unwrap() > best
//                             && sample_rate.as_u64().unwrap() <= 48000
//                         {
//                             best_ext = audio_ext;
//                             best = sample_rate.as_u64().unwrap();
//                             best_url = url;
//                         }
//                     }
//                 }
//             }
//         }
//         if !best_url.is_empty() {
//             Some(best_url.to_owned())
//         } else {
//             None
//         }
//     } else {
//         None
//     }
// }

// fn get_video_urls(data: &Value) -> Vec<(Url, Option<Language>)> {
//     let mut output = Vec::new();

//     output
// }

// fn get_subs_urls(data: &Value) -> Vec<(Url, Option<String>, Option<Language>, Option<String>)> {
//     let mut output = vec![];
//     if let Some(Value::Object(subs)) = data.get("subtitles") {
//         for (lang_code, subs) in subs {
//             if let Value::Array(subs) = subs {
//                 if let Some(Value::Object(subs)) = subs.get(0) {
//                     if let (
//                         Some(Value::String(name)),
//                         Some(Value::String(url)),
//                         Some(Value::String(extension)),
//                     ) = (subs.get("name"), subs.get("url"), subs.get("ext"))
//                     {
//                         output.push((
//                             Url::from_str(url).unwrap(),
//                             name.to_owned().into(),
//                             lang_str_to_lang(lang_code),
//                             extension.to_owned().into(),
//                         ))
//                     }
//                 }
//             }
//         }
//     }
//     output
// }

// fn get_max_res_thumbnail_url(data: &Value) -> Option<Url> {
//     if let Some(Value::String(url)) = data.get("thumbnail") {
//         Some(Url::from_str(url).unwrap())
//     } else if let Some(Value::Array(thumbnails)) = data.get("thumbnails") {
//         let mut max_res_url = None;
//         let mut webm_max_res_url = None;
//         let mut highest_res: u64 = 0;
//         let mut webm_highest_res: u64 = 0;
//         for thumbnail in thumbnails {
//             if let Some(Value::Number(height)) = thumbnail.get("height") {
//                 if let Some(Value::String(url)) = thumbnail.get("url") {
//                     let height = height.as_u64().unwrap();
//                     if url.contains("webm") {
//                         if webm_highest_res < height {
//                             webm_highest_res = height;
//                             webm_max_res_url = Some(url);
//                         }
//                     } else if highest_res < height {
//                         highest_res = height;
//                         max_res_url = Some(url);
//                     }
//                 }
//             }
//         }
//         if highest_res >= webm_highest_res {
//             max_res_url.map(|s| Url::from_str(s).unwrap())
//         } else {
//             webm_max_res_url.map(|s| Url::from_str(s).unwrap())
//         }
//     } else {
//         None
//     }
// }

// fn get_stream_title(data: &Value) -> Option<String> {
//     if let Some(Value::String(nym)) = data.get("fulltitle") {
//         Some(nym.to_owned())
//     } else if let Some(Value::String(nym)) = data.get("title") {
//         Some(nym.to_owned())
//     } else {
//         None
//     }
// }

pub async fn download_stream(
    video_urls: Vec<(Url, Option<Language>)>,
    audio_urls: Vec<(Url, Option<Language>)>,
    subs_urls: Vec<(Url, Option<String>, Option<Language>, Option<String>)>,
    thumbnail_url: Option<Url>,
    title: Option<String>,
    url_hash: UrlHash,
    req_client: Client,
) {
    let base_path = format!["{STREAM_DIR}/{}", url_hash.0];

    let (kill_switch, mut kill_signal) = channel::<()>(1);

    let threadify_dl_stream = |url: Url, fallible: bool| {
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
        reader
    };

    let audio_pipes: Vec<_> = audio_urls
        .into_iter()
        .map(|(url, lang)| (threadify_dl_stream(url, false), lang))
        .collect();
    let video_pipes: Vec<_> = video_urls
        .into_iter()
        .map(|(url, lang)| (threadify_dl_stream(url, false), lang))
        .collect();
    let subs_pipes: Vec<_> = subs_urls
        .into_iter()
        .map(|(url, name, lang, extension)| (threadify_dl_stream(url, true), name, lang, extension))
        .collect();

    let thumbnail_pipe = thumbnail_url.map(|url| threadify_dl_stream(url, true));

    let mut has_video = false;
    let ext = if video_pipes.is_empty() {
        if audio_pipes.is_empty() {
            return;
        }
        "mka"
    } else {
        has_video = true;
        "mkv"
    };

    let title = title.unwrap_or_else(|| "unknown".into());
    let output_path = format!["{base_path}/{title}.{ext}"];

    let mut cmd = Command::new("ffmpeg");

    let mut fds_to_map =
        Vec::with_capacity(subs_pipes.len() + video_pipes.len() + audio_pipes.len());

    for (i, (pipe, lang)) in video_pipes.iter().enumerate() {
        let fd = pipe.as_raw_fd();
        cmd.arg("-i")
            .arg(format!["pipe:{fd}"])
            .arg("-map")
            .arg(format!["{}:v", i]);
        if let Some(lang) = lang {
            cmd.arg(format!["-metadata:s:v:{}", i])
                .arg(format!["language={}", lang.to_639_3()]);
        }
        fds_to_map.push(fd);
    }

    for (i, (pipe, lang)) in audio_pipes.iter().enumerate() {
        let i = i + video_pipes.len();
        let fd = pipe.as_raw_fd();
        cmd.arg("-i")
            .arg(format!["pipe:{fd}"])
            .arg("-map")
            .arg(format!["{}:a", i]);
        if let Some(lang) = lang {
            cmd.arg(format!["-metadata:s:a:{}", i])
                .arg(format!["language={}", lang.to_639_3()]);
        }
        fds_to_map.push(fd);
    }

    for (i, (pipe, name, lang, extension)) in subs_pipes.iter().enumerate() {
        let i = i + video_pipes.len() + audio_pipes.len();
        let subs_fd = pipe.as_raw_fd();
        if let Some(extension) = extension {
            cmd.arg("-f").arg(extension);
        }
        cmd.arg("-i")
            .arg(format!["pipe:{subs_fd}"])
            .arg("-map")
            .arg(format!["{}:s", i]);
        if let Some(lang) = lang {
            cmd.arg(format!["-metadata:s:s:{}", i])
                .arg(format!["language={}", lang.to_639_3()]);
        }
        if let Some(name) = name {
            cmd.arg(format!["-metadata:s:s:{}", i])
                .arg(format!["title={}", name]);
        }
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
