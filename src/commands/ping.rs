use serenity::framework::standard::macros::command;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::ops::Deref;

use crate::database;

#[command]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let response = {
        let lock = ctx.data.read().await;
        let db = &lock
            .get::<database::Database>()
            .expect("database in context data")
            .0;

        let author_id: i64 = msg.author.id.0.try_into().unwrap();
        let channel_id: i64 = msg.channel_id.0.try_into().unwrap();
        let message_id: i64 = msg.id.0.try_into().unwrap();
        let timestamp = msg.timestamp.deref();

        match sqlx::query("INSERT INTO pings (author_id, channel_id, message_id, timestamp) VALUES ($1, $2, $3, $4)")
            .bind(author_id)
            .bind(channel_id)
            .bind(message_id)
            .bind(timestamp)
            .execute(db)
            .await
        {
            Ok(_) => match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) AS count FROM pings;")
                .fetch_one(db)
                .await
            {
                Ok(result) => format!("Pong {result}!"),
                Err(err) => {
                    log::error!("failed to pong: {:?}", err);
                    "Failed to pong :(".to_owned()
                }
            },
            Err(err) => {
                log::error!("failed to ping: {:?}", err);
                "Failed to ping :(".to_owned()
            }
        }
    };

    msg.reply(ctx, response).await?;
    Ok(())
}
