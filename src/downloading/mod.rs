mod download;
mod stream_data;
mod yt_dlp;
mod convert;

use isolang::Language;
use reqwest::Url;
use stream_data::*;

const MAX_VIDEO_SIZE: u64 = 1024 * 1024 * 100;
const MAX_VERTICAL_HEIGHT: u64 = 720;

// let url_hash = hash_url(url.clone());
// let path = format!["{STREAM_DIR}{}", url_hash.0];
// std::fs::DirBuilder::new()
//     .recursive(true)
//     .create(&path)
//     .unwrap();

pub async fn get_streams(
    data: StreamData,
) -> (
    (Option<Url>, bool),
    Option<Language>,
    Option<(Option<Language>, SubtitlesFormat)>,
    Option<Url>,
    Option<String>,
) {
    // let mut audio_formats: Vec<(Url, Option<Language>)> = Vec::with_capacity(data.formats.len());
    // let mut video_formats: Vec<(Url, Option<Language>)> = Vec::with_capacity(data.formats.len());
    // for format in data.formats {
    //     if format.video_ext.is_some() {
    //         if let Some(height) = format.height {
    //             if height <= MAX_VERTICAL_HEIGHT {
    //                 if let Some(url) = format.url {
    //                     video_formats.push((url, format.language));
    //                 }
    //                 continue;
    //             }
    //         }
    //     }
    //     if format.audio_ext.is_some() {
    //         if let Some(url) = format.url {
    //             audio_formats.push((url, format.language));
    //         }
    //         continue;
    //     }
    // }

    let mut the_subs: Option<(Option<Language>, SubtitlesFormat)> = None;
    for (lang, subs_group) in data.subtitles {
        'inner: for subs in subs_group.subs {
            if subs.ext.is_some() {
                the_subs = Some((lang, subs));
                break 'inner;
            }
        }
    }

    (
        (
            data.url,
            data.audio_ext.is_some() && data.video_ext.is_none(),
        ),
        data.language,
        the_subs,
        data.thumbnail,
        data.fulltitle.map_or_else(|| data.title, |f| f.into()),
    )
}
