use std::collections::HashMap;

use rowifi_core::rankbinds::{add_rankbind, AddRankbindError, RankbindArguments};
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

#[derive(Arguments)]
pub struct RankbindRouteArguments {
    pub group_id: u64,
    pub rank_id: u8,
    pub template: String,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn new_rankbind(
    bot: Extension<BotContext>,
    command: Command<RankbindRouteArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = new_rankbind_func(bot, command.ctx, command.args).await {
            tracing::error!(?err);
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

async fn new_rankbind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: RankbindRouteArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, rankbinds FROM guilds WHERE guild_id = $1",
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
        &guild.rankbinds.0,
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
    Oh no! There does not seem to be a rank with ID {} in the group {}
            "#,
                args.rank_id, args.group_id
            );
            ctx.respond(&bot).content(&message).unwrap().exec().await?;
            return Ok(());
        }
        Err(AddRankbindError::InvalidGroup) => {
            let message = format!(
                r#"
    Oh no! There does not seem to be a group with ID {}
            "#,
                args.group_id
            );
            ctx.respond(&bot).content(&message).unwrap().exec().await?;
            return Ok(());
        }
        Err(AddRankbindError::Generic(err)) => return Err(err),
    };

    let mut description = String::new();
    if res.modified {
        description.push_str(":warning: Bind already exists. Modified it to:\n\n")
    }
    description.push_str(&format!("**Rank Id: {}**\n", res.bind.group_rank_id));
    description.push_str(&format!(
        "Template: {}\nPriority: {}\n Roles: {}",
        res.bind.template,
        res.bind.priority,
        res.bind
            .discord_roles
            .into_iter()
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
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Bind Addition Successful")
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().exec().await?;

    Ok(())
}
