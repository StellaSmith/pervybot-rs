use isolang::Language;
use reqwest::Url;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Fragment {
    pub(crate) url: Option<Url>,
    pub(crate) duration: f64,
}

#[derive(Deserialize)]
pub struct Format {
    pub(crate) abr: Option<f64>,
    pub(crate) acodec: Option<String>,
    pub(crate) asr: Option<u64>,
    pub(crate) audio_ext: Option<String>,
    pub(crate) columns: Option<u64>,
    pub(crate) container: Option<String>,
    pub(crate) dynamic_range: Option<String>,
    pub(crate) ext: Option<String>,
    pub(crate) filesize: Option<u64>,
    pub(crate) format_id: Option<String>,
    pub(crate) format_note: Option<String>,
    pub(crate) format: Option<String>,
    pub(crate) fps: Option<u64>,
    pub(crate) fragments: Vec<Fragment>,
    pub(crate) has_drm: Option<bool>,
    pub(crate) height: Option<u64>,
    pub(crate) language_preference: Option<String>,
    pub(crate) language: Option<Language>,
    pub(crate) protocol: Option<String>,
    pub(crate) quality: Option<u64>,
    pub(crate) resolution: Option<String>,
    pub(crate) rows: Option<u64>,
    pub(crate) source_preference: Option<i64>,
    pub(crate) tbr: Option<f64>,
    pub(crate) url: Option<Url>,
    pub(crate) vbr: Option<f64>,
    pub(crate) vcodec: Option<String>,
    pub(crate) video_ext: Option<String>,
    pub(crate) width: Option<u64>,
}

#[derive(Deserialize)]
pub struct SubFormat {
    pub(crate) ext: Option<String>,
    pub(crate) url: Option<Url>,
    pub(crate) name: Option<String>,
}

#[derive(Deserialize)]
pub struct Subtitles {
    pub(crate) subs: Vec<SubFormat>,
}

#[derive(Deserialize)]
pub struct ThumbNailFormat {
    pub(crate) url: Option<Url>,
    pub(crate) preference: Option<i64>,
    pub(crate) id: Option<String>,
}

#[derive(Deserialize)]
pub struct StreamData {
    pub(crate) title: Option<String>,
    pub(crate) fulltitle: Option<String>,
    pub(crate) formats: Vec<Format>,
    pub(crate) subtitles: Vec<(Option<Language>, Subtitles)>,
    pub(crate) thumbnails: Vec<ThumbNailFormat>,
}
