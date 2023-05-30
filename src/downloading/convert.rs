use std::os::unix::prelude::AsRawFd;

use isolang::Language;
use tokio_command_fds::{CommandFdExt, FdMapping};
use tokio_pipe::PipeRead;

use crate::stream_dir;

use super::{
    stream_data::{StreamData, SubtitlesFormat},
    KillSignal,
};

pub async fn convert(
    mut kill_signal: KillSignal,
    data: StreamData,
    audio_only: bool,
    stream_pipe: Option<(PipeRead, Option<Language>)>,
    subs_pipe: Option<(PipeRead, Option<Language>, SubtitlesFormat)>,
    thumbnail_pipe: Option<PipeRead>,
) {
    let mut cmd = tokio::process::Command::new("ffmpeg");

    let mut fds_to_map =
        Vec::with_capacity(subs_pipe.is_some() as usize + stream_pipe.is_some() as usize);

    for (i, (pipe, lang)) in stream_pipe.iter().enumerate() {
        let fd = pipe.as_raw_fd();
        if audio_only {
            cmd.arg("-i")
                .arg(format!["pipe:{fd}"])
                .arg("-map")
                .arg(format!["{}:a", i]);
            if let Some(lang) = lang {
                cmd.arg(format!["-metadata:s:a:{}", i])
                    .arg(format!["language={}", lang.to_639_3()]);
            }
        } else {
            cmd.arg("-i")
                .arg(format!["pipe:{fd}"])
                .arg("-map")
                .arg(format!["{}:v", i]);
            if let Some(lang) = lang {
                cmd.arg(format!["-metadata:s:v:{}", i])
                    .arg(format!["language={}", lang.to_639_3()]);
            }
        }
        fds_to_map.push(fd);
    }

    for (i, (pipe, lang, SubtitlesFormat { ext, url: _, name })) in subs_pipe.iter().enumerate() {
        let i = i + stream_pipe.is_some() as usize;
        let subs_fd = pipe.as_raw_fd();
        if let Some(extension) = ext {
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

    if !audio_only {
        cmd.args(["-r", "30"]);
    }

    let platform = data.extractor_key.unwrap_or_else(|| "unknown".into());
    let title = data.title.unwrap_or_else(|| "unknown".into());
    let ext = if audio_only { "mka" } else { "mkv" };

    let file_name = sanitize_filename::sanitize(format!["{title} - {platform}.{ext}"]);

    cmd.arg(stream_dir().join(file_name));

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
