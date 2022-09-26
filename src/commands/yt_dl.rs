use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::prelude::*;

#[command]
async fn yt_dl(_ctx: &Context, _msg: &Message) -> CommandResult {
    Ok(())
}
