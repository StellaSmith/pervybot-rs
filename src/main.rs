mod commands;
mod database;
mod downloading;

use std::path::PathBuf;

use serenity::async_trait;

use serenity::framework::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;

fn stream_dir() -> PathBuf {
    std::env::var("STREAM_DOWNLOAD_DIR").unwrap().into()
}

fn init() {
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(stream_dir())
        .unwrap();
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, bot_data: serenity::model::gateway::Ready) {
        tokio::spawn({
            let ctx = ctx.clone();
            async move {
                if let Ok(Ok(logging_channel)) = std::env::var("LOGGING_CHANNEL").map(|s| s.parse())
                {
                    ChannelId(logging_channel)
                        .send_message(&ctx, |builder| builder.content("Bot started"))
                        .await
                        .ok();
                }
            }
        });

        log::info!("connected as {}", bot_data.user.tag());
        // at this point in time we might not be connected to a guild, so we fetch them
        let guilds = ctx.cache.current_user().guilds(&ctx).await.unwrap();
        let guild_names = guilds
            .iter()
            .map(|guild| guild.name.as_str())
            .collect::<Vec<_>>();

        log::info!("joined to:\n\t{}", guild_names.join("\n\t"));
    }
}

async fn dynamic_prefix(ctx: &Context, msg: &Message) -> Option<String> {
    let bot_id = ctx.cache.current_user_id();

    // don't like having to rebuild a regex each time
    let re = regex::Regex::new(&format!(r"(\s*<@{bot_id}>\s*)")).unwrap();
    re.captures(&msg.content)
        .and_then(|captures| captures.get(1))
        .map(|m| m.as_str().to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();
    init();

    let framework = StandardFramework::new()
        .configure(|c| {
            c.case_insensitivity(true)
                .no_dm_prefix(true)
                .dynamic_prefix(|ctx, msg| Box::pin(async move { dynamic_prefix(ctx, msg).await }))
        })
        .group(&commands::GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    log::info!("connecting to database");
    let db = {
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        sqlx::any::AnyPoolOptions::new().connect(&db_url).await?
    };
    {
        let mut lock = client.data.write().await;
        lock.insert::<database::Database>(database::Database(db));
    };

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        log::error!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
