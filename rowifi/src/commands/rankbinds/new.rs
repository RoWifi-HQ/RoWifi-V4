use std::collections::HashMap;

use rowifi_core::rankbinds::add::{add_rankbind, AddRankbindError, RankbindArguments};
use rowifi_framework::prelude::*;
use rowifi_models::{
    bind::Template,
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    id::RoleId,
    roblox::id::GroupId,
};
use twilight_mention::Mention;

#[derive(Arguments, Debug)]
pub struct RankbindRouteArguments {
    pub group_id: u64,
    pub rank_id: u32,
    pub template: String,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn new_rankbind(
    bot: Extension<BotContext>,
    command: Command<RankbindRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = new_rankbind_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
async fn new_rankbind_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: RankbindRouteArguments,
) -> CommandResult {
    tracing::debug!("rankbinds new invoked");
    let guild = bot
        .get_guild(
            "SELECT guild_id, rankbinds, log_channel FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;
    let server = bot.server(ctx.guild_id).await?;

    let server_roles = bot
        .cache
        .guild_roles(server.roles.iter().copied())
        .await?
        .into_iter()
        .map(|r| (r.id, r))
        .collect::<HashMap<_, _>>();

    let res = match add_rankbind(
        &bot.roblox,
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.rankbinds,
        &server_roles,
        RankbindArguments {
            group_id: GroupId(args.group_id),
            rank_id: args.rank_id,
            template: Template(args.template),
            priority: args.priority,
            discord_roles: args.discord_roles,
        },
    )
    .await
    {
        Ok(res) => res,
        Err(AddRankbindError::InvalidRank) => {
            let message = format!(
                r#"
    Oh no! There does not seem to be a rank with ID {} in the group {}.
            "#,
                args.rank_id, args.group_id
            );
            ctx.respond(&bot).content(&message).unwrap().await?;
            return Ok(());
        }
        Err(AddRankbindError::InvalidGroup) => {
            let message = format!(
                r#"
    Oh no! There does not seem to be a group with ID {}.
            "#,
                args.group_id
            );
            ctx.respond(&bot).content(&message).unwrap().await?;
            return Ok(());
        }
        Err(AddRankbindError::Generic(err)) => return Err(err),
    };

    let mut description = String::new();
    if res.modified {
        description.push_str(":warning: Bind already exists. Modified it to:\n\n");
    }
    description.push_str(&format!("**Rank Id: {}**\n", res.bind.group_rank_id));
    description.push_str(&format!(
        "Template: {}\nPriority: {}\n Roles: {}",
        res.bind.template,
        res.bind.priority,
        res.bind
            .discord_roles
            .iter()
            .map(|r| r.0.mention().to_string())
            .collect::<String>()
    ));

    if !res.ignored_roles.is_empty() {
        let ignored_roles_str = res
            .ignored_roles
            .iter()
            .map(|r| r.0.mention().to_string())
            .collect::<String>();
        description.push_str(&format!("\n\n🚫 Invalid Roles: {}", ignored_roles_str));
    }

    let embed = EmbedBuilder::new()
        .color(DARK_GREEN)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Bind Addition Successful")
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    if let Some(log_channel) = guild.log_channel {
        let embed = EmbedBuilder::new()
            .color(BLUE)
            .footer(EmbedFooterBuilder::new("RoWifi").build())
            .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
            .title(format!("Action by <@{}>", ctx.author_id))
            .description("Rankbind added")
            .field(EmbedFieldBuilder::new(format!("**Rank Id: {}**\n", res.bind.group_rank_id), format!(
                "Template: {}\nPriority: {}\n Roles: {}",
                res.bind.template,
                res.bind.priority,
                res.bind
                    .discord_roles
                    .iter()
                    .map(|r| r.0.mention().to_string())
                    .collect::<String>()
            )))
            .build();
        let _ = bot
            .http
            .create_message(log_channel.0)
            .embeds(&[embed])
            .await;
    }

    Ok(())
}
