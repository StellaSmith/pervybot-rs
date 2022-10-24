mod external;
mod stream_data;
mod yt_dlp;

use stream_data::*;

const MAX_VIDEO_SIZE: u64 = 1024 * 1024 * 100;
const MAX_VERTICAL_RESOLUTION: u64 = 720;

// let url_hash = hash_url(url.clone());
// let path = format!["{STREAM_DIR}{}", url_hash.0];
// std::fs::DirBuilder::new()
//     .recursive(true)
//     .create(&path)
//     .unwrap();
