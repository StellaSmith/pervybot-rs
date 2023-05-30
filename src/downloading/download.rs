use std::thread;

// use async_process::Command;
use isolang::Language;
use reqwest::Client;
use serenity::futures::StreamExt;
use tokio::{io::AsyncWriteExt, sync::mpsc::channel};
use tokio_pipe::PipeRead;
use url::Url;

use super::{stream_data::{StreamData, SubtitlesFormat}, KillSignal};

pub async fn download_stream(
    data: StreamData,
    req_client: Client,
) -> (
    KillSignal,
    bool,
    Option<(PipeRead, Option<Language>)>,
    Option<(PipeRead, Option<Language>, SubtitlesFormat)>,
    Option<PipeRead>,
) {
    let (kill_switch, kill_signal) = channel::<()>(1);

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

    let stream_pipe: Option<_> = data
        .url
        .into_iter()
        .map(|url| (threadify_dl_stream(url, false), data.language))
        .next();

    let subs_pipe: Option<_> = data
        .subtitles
        .into_iter()
        .flat_map(|(lang, formats)| {
            formats.subs.get(0).map(|subs| {
                (
                    threadify_dl_stream(subs.url.clone(), true),
                    lang,
                    (*subs).clone(),
                )
            })
        })
        .next();

    let thumbnail_pipe = data.thumbnail.map(|url| threadify_dl_stream(url, true));

    (
        kill_signal,
        data.video_ext.is_none() && data.audio_ext.is_some(),
        stream_pipe,
        subs_pipe,
        thumbnail_pipe,
    )
}
