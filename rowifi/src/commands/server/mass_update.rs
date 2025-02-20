use rowifi_database::postgres::Row;
use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::http::interaction::{
        InteractionResponse, InteractionResponseData, InteractionResponseType,
    },
    guild::{GuildType, PartialRoGuild},
    id::{GuildId, RoleId},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MassUpdateQueueArguments {
    pub guild_id: GuildId,
    pub role_id: Option<RoleId>,
}

#[derive(Debug)]
pub struct MassUpdateGuild {
    pub updates: i32,
    pub errored: i32,
}

#[derive(Debug)]
pub struct MassUpdateGuildCount {
    pub count: i64,
}

pub async fn update_all(bot: Extension<BotContext>, command: Command<()>) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_all_func(&bot, &command.ctx).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-all` queue started".into()),
            ..Default::default()
        }),
    })
}

#[tracing::instrument(skip_all)]
pub async fn update_all_func(bot: &BotContext, ctx: &CommandContext) -> CommandResult {
    let guild = bot.database.query_opt::<PartialRoGuild>("SELECT guild_id, kind, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, log_channel FROM guilds WHERE guild_id = $1", &[&ctx.guild_id]).await?.unwrap_or_else(|| PartialRoGuild::new(ctx.guild_id));
    if guild.kind.unwrap() == GuildType::Free {
        let message = "Mass Update commands are only available to Premium servers";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    let mass_update_guild = bot
        .database
        .query_opt::<MassUpdateGuild>(
            "SELECT * FROM mass_update_guilds WHERE guild_id = $1",
            &[&ctx.guild_id],
        )
        .await?;
    if let Some(mass_update_guild) = mass_update_guild {
        let count = bot.database.query::<MassUpdateGuildCount>("
            WITH mass_update_counts AS ( SELECT guild_id, user_id, ROW_NUMBER() OVER (ORDER BY timestamp ASC) AS row_num FROM mass_update_users )
            SELECT row_num AS count FROM mass_update_counts WHERE guild_id = $1 LIMIT 1
        ", &[&ctx.guild_id]).await?;
        let message = format!("This server is currently present in the mass update queue.\nRemaining users: {}\nErrors: {}\nThere are {} user(s) ahead in the queue.", mass_update_guild.updates, mass_update_guild.errored, count[0].count - 1);
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let count = bot
        .database
        .query::<MassUpdateGuildCount>("SELECT COUNT(*) AS count FROM mass_update_users", &[])
        .await?;
    bot.database
        .execute(
            "INSERT INTO mass_update_guilds(guild_id) VALUES($1)",
            &[&ctx.guild_id],
        )
        .await?;
    ctx.respond(bot)
        .content(&format!(
            "`update-all` queue started. There are {} users ahead in the queue",
            count[0].count
        ))
        .unwrap()
        .await?;

    Ok(())
}

#[derive(Arguments, Debug)]
pub struct UpdateRoleArguments {
    pub role: u64,
}

pub async fn update_role(
    bot: Extension<BotContext>,
    command: Command<UpdateRoleArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = update_role_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some("`update-role` queue started".into()),
            ..Default::default()
        }),
    })
}

#[tracing::instrument(skip_all)]
pub async fn update_role_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateRoleArguments,
) -> CommandResult {
    let guild = bot.database.query_opt::<PartialRoGuild>("SELECT guild_id, kind, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, log_channel FROM guilds WHERE guild_id = $1", &[&ctx.guild_id]).await?.unwrap_or_else(|| PartialRoGuild::new(ctx.guild_id));
    if guild.kind.unwrap() == GuildType::Free {
        let message = "Mass Update commands are only available to Premium servers";
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    }

    let mass_update_guild = bot
        .database
        .query_opt::<MassUpdateGuild>(
            "SELECT guild_id FROM mass_update_guilds WHERE guild_id = $1",
            &[&ctx.guild_id],
        )
        .await?;
    if let Some(mass_update_guild) = mass_update_guild {
        let count = bot.database.query::<MassUpdateGuildCount>("
            WITH mass_update_counts AS ( SELECT guild_id, user_id, ROW_NUMBER() OVER (ORDER BY timestamp ASC) AS row_num FROM mass_update_users )
            SELECT row_num AS count FROM mass_update_counts WHERE guild_id = $1 LIMIT 1
        ", &[&ctx.guild_id]).await?;
        let message = format!("This server is currently present in the mass update queue.\nRemaining users: {}\nErrors: {}\nThere are {} users ahead in the queue.", mass_update_guild.updates, mass_update_guild.errored, count[0].count);
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let count = bot
        .database
        .query::<MassUpdateGuildCount>("SELECT COUNT(*) AS count FROM mass_update_users", &[])
        .await?;
    bot.database
        .execute(
            "INSERT INTO mass_update_guilds(guild_id, role_id) VALUES($1, $2)",
            &[&ctx.guild_id, &RoleId::new(args.role)],
        )
        .await?;
    ctx.respond(bot)
        .content(&format!(
            "`update-role` queue started. There are {} users ahead in the queue",
            count[0].count
        ))
        .unwrap()
        .await?;

    Ok(())
}

impl TryFrom<Row> for MassUpdateGuild {
    type Error = rowifi_database::postgres::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let updates = row.try_get("updates")?;
        let errored = row.try_get("errored")?;

        Ok(Self { updates, errored })
    }
}

impl TryFrom<Row> for MassUpdateGuildCount {
    type Error = rowifi_database::postgres::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let count = row.try_get("count")?;

        Ok(Self { count })
    }
}
