use anyhow::Result;
use octocrab::{
    models::{
        hooks::{Config as HookConfig, ContentType, Hook},
        webhook_events::WebhookEventType,
        Repository,
    },
    Octocrab, Page,
};
use poise::serenity_prelude::{
    self as serenity, json::json, ChannelType, CreateChannel, GuildChannel, GuildId, Http,
};

const DELAY_MS: u64 = 10000;
const BLACKLISTED_REPOS: [&str; 3] = [".GITHUB", "SUBMISSIONS", "COMMUNITY"];

pub async fn fetch_repos(octo: Octocrab, org: &String) -> Result<Page<Repository>> {
    let response = octo.orgs(org).list_repos().send().await?;

    Ok(response)
}

pub async fn fetch_categories(guild_id: GuildId, http: &Http) -> Result<Vec<GuildChannel>> {
    let guild = http.get_guild(guild_id).await.unwrap();

    let channels = guild.channels(http).await.unwrap();

    let categories = channels
        .iter()
        .filter(|c| c.1.kind == serenity::model::channel::ChannelType::Category)
        .map(|c| c.1.to_owned())
        .collect::<Vec<GuildChannel>>();

    Ok(categories)
}

pub struct Github {
    pub github_token: Option<String>,
    pub bot_token: String,
    pub guild_id: GuildId,
    pub org: String,
    pub webhook_events: Vec<WebhookEventType>,
}

impl Github {
    pub fn new(org: String,github_token: Option<String>, bot_token: String, guild_id: GuildId) -> Self {
        Self {
            github_token,
            bot_token,
            guild_id,
            org,
            webhook_events: vec![
                WebhookEventType::BranchProtectionRule,
                WebhookEventType::BranchProtectionRule,
                WebhookEventType::CheckRun,
                WebhookEventType::CheckSuite,
                WebhookEventType::CodeScanningAlert,
                WebhookEventType::CommitComment,
                WebhookEventType::Create,
                WebhookEventType::Delete,
                WebhookEventType::DependabotAlert,
                WebhookEventType::DeployKey,
                WebhookEventType::Deployment,
                WebhookEventType::DeploymentStatus,
                WebhookEventType::Discussion,
                WebhookEventType::DiscussionComment,
                WebhookEventType::Fork,
                WebhookEventType::Gollum,
                WebhookEventType::IssueComment,
                WebhookEventType::Issues,
                WebhookEventType::Label,
                WebhookEventType::Member,
                WebhookEventType::MergeGroup,
                WebhookEventType::Meta,
                WebhookEventType::Milestone,
                WebhookEventType::Package,
                WebhookEventType::PageBuild,
                WebhookEventType::Ping,
                WebhookEventType::ProjectCard,
                WebhookEventType::Project,
                WebhookEventType::ProjectColumn,
                WebhookEventType::Public,
                WebhookEventType::PullRequest,
                WebhookEventType::PullRequestReviewComment,
                WebhookEventType::PullRequestReview,
                WebhookEventType::PullRequestReviewThread,
                WebhookEventType::Push,
                WebhookEventType::RegistryPackage,
                WebhookEventType::Release,
                WebhookEventType::RepositoryAdvisory,
                WebhookEventType::Repository,
                WebhookEventType::RepositoryImport,
                WebhookEventType::RepositoryVulnerabilityAlert,
                WebhookEventType::SecretScanningAlert,
                WebhookEventType::SecretScanningAlertLocation,
                WebhookEventType::SecurityAndAnalysis,
                WebhookEventType::Star,
                WebhookEventType::Status,
                WebhookEventType::TeamAdd,
                WebhookEventType::Watch,
            ],
        }
    }

    pub async fn check_loop(&self) -> Result<()> {
        let http = Http::new(&self.bot_token);

        let octo = match self.github_token.clone() {
            Some(t) => Octocrab::builder().personal_token(t).build()?,
            None => Octocrab::builder().build()?,
        };

        loop {
            let repos = fetch_repos(octo.clone(), &self.org).await?;
            let categories = fetch_categories(self.guild_id, &http).await?;

            println!("checking repos");
            self.update(repos, categories, &http, &octo).await?;
            println!("done checking repos");

            tokio::time::sleep(tokio::time::Duration::from_millis(DELAY_MS)).await;
        }
    }

    pub async fn update(
        &self,
        repos: Page<Repository>,
        categories: Vec<GuildChannel>,
        http: &Http,
        octo: &Octocrab,
    ) -> Result<()> {
        let category_names = categories
            .iter()
            .map(|c| c.name().to_uppercase())
            .collect::<Vec<String>>();

        for repo in repos {
            println!("checking repo: {}", repo.name);

            if category_names.contains(&repo.name.to_uppercase()) {
                println!("repo already exists");
                continue;
            }

            if BLACKLISTED_REPOS.contains(&repo.name.to_uppercase().as_str()) {
                println!("repo is blacklisted");
                continue;
            }

            // create discord category
            let guild = http.get_guild(self.guild_id).await?;

            // category
            println!("creating category: {}", repo.name);
            let cat = guild
                .create_channel(
                    http,
                    CreateChannel::new(repo.name.clone()).kind(ChannelType::Category),
                )
                .await
                .expect("failed to create category");

            // announcements
            println!("creating announcement channel");
            guild
                .create_channel(
                    http,
                    CreateChannel::new("announcement")
                        .kind(ChannelType::News)
                        .category(cat.id),
                )
                .await
                .expect("failed to create announcement channel");

            // general
            println!("creating general channel");
            guild
                .create_channel(
                    http,
                    CreateChannel::new(format!("{}-general", repo.name))
                        .kind(ChannelType::Text)
                        .category(cat.id),
                )
                .await
                .expect("failed to create general channel");

            // git webhook
            println!("creating git channel");
            let git_channel = guild
                .create_channel(
                    http,
                    CreateChannel::new("git")
                        .kind(ChannelType::Text)
                        .category(cat.id),
                )
                .await
                .expect("failed to create git channel");

            let map = json!({"name":"GitHub"});

            println!("creating discord webhook");
            let discord_webhook = http
                .create_webhook(git_channel.id, &map, None)
                .await
                .expect("failed to create discord webhook");

            let repo_handler = octo.repos(&self.org, &repo.name);

            let hook_config = HookConfig {
                url: discord_webhook.url()? + "/github",
                content_type: Some(ContentType::Json),
                secret: None,
                insecure_ssl: None,
            };

            let hook = Hook {
                name: "web".to_string(),
                active: true,
                config: hook_config,
                events: self.webhook_events.clone(),
                ..Hook::default()
            };

            println!("creating git webhook");
            repo_handler
                .create_hook(hook)
                .await
                .expect("failed to create git webhook");
        }

        Ok(())
    }
}
