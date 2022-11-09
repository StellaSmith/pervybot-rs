mod convert;
mod download;
mod stream_data;
mod yt_dlp;

use stream_data::*;
use tokio::sync::mpsc::Receiver;

// const MAX_VIDEO_SIZE: u64 = 1024 * 1024 * 100; // 100M
// const MAX_VERTICAL_HEIGHT: u64 = 720; // 720p

pub type KillSignal = Receiver<()>;
