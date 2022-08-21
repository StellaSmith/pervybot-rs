use lazy_static::lazy_static;
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::collections::HashSet;

#[group]
#[commands(ping)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, bot_data: serenity::model::gateway::Ready) {
        log::info!("connected as {}", bot_data.user.tag());
        // at this point in time we might not be connected to a guild, so we fetch them
        let guilds = ctx.cache.current_user().guilds(&ctx.http).await.unwrap();
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

lazy_static! {
    static ref NO_INHERIT_ENV: HashSet<&'static str> = {
        let mut s = HashSet::<&'static str>::default();
        s.insert("DISCORD_TOKEN");
        s
    };
}

#[tokio::main]
async fn main() {
    let env_conf = match env_file_reader::read_file(".env") {
        Err(err) => {
            eprintln!("Couldn't parse .env file.");
            panic!("{:?}", err);
        }
        Ok(ok) => ok,
    };

    // environment variables are inherited,
    // we don't want any of the process we spawn contain these variables
    // so we don't set them
    for (k, v) in env_conf
        .iter()
        .filter(|(k, ..)| !NO_INHERIT_ENV.contains(k.as_str()))
    {
        std::env::set_var(k, v)
    }

    env_logger::init();

    let framework = StandardFramework::new()
        .configure(|c| {
            c.case_insensitivity(true)
                .no_dm_prefix(true)
                .dynamic_prefix(|ctx, msg| Box::pin(async move { dynamic_prefix(ctx, msg).await }))
        })
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env_conf.get("DISCORD_TOKEN").expect("DISCORD_TOKEN");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        log::error!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}
