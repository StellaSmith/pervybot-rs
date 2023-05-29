use serenity::framework::standard::macros::group;

mod yt_dl;
pub use yt_dl::*;

mod ping;
pub use ping::*;

#[group]
#[commands(ping, yt_dl)]
pub struct General;
