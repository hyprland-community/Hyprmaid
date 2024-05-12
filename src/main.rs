use anyhow::Result;
use poise::serenity_prelude::{self as serenity, GuildId};
use dotenvy::dotenv;
struct Data {} // User data, which is stored and accessible in all command invocations

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

mod github;


/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().expect(".env file not found");

    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let github = github::Github::new(
        std::env::var("GITHUB_ORG").expect("missing GITHUB_ORG"),
        std::env::var("GITHUB_TOKEN").ok(),
        token.clone(),
        GuildId::new(std::env::var("DISCORD_SERVER_ID").expect("missing DISCORD_SERVER_ID")
            .parse::<u64>().expect("invalid DISCORD_SERVER_ID")),
    );

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                age(),
                // commands::github::update_repos()
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    println!("starting bot");

    let (git, discord) = tokio::join!(github.check_loop(), client.start());

    git.unwrap();
    discord.unwrap();

    Ok(())
}
