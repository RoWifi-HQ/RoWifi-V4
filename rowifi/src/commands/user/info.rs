use rowifi_framework::prelude::*;
use rowifi_models::{
    discord::{
        http::interaction::{InteractionResponse, InteractionResponseType},
        util::Timestamp,
    },
    id::UserId,
    user::RoUser,
};
use std::collections::HashSet;

#[derive(Arguments, Debug)]
pub struct UserInfoArguments {
    pub user: Option<UserId>,
}

pub async fn userinfo(
    bot: Extension<BotContext>,
    command: Command<UserInfoArguments>,
) -> impl IntoResponse {
    spawn_command(userinfo_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn userinfo_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: UserInfoArguments,
) -> CommandResult {
    let user_id = args.user.unwrap_or(ctx.author_id);
    let Some(discord_user) = bot.member(ctx.guild_id, user_id).await? else {
        tracing::trace!("could not find user");
        // Should not ever happen since slash command guarantees that the user exists.
        // But handling this nonetheless is useful.
        let message = format!(
            r#"
        <:rowifi:733311296732266577> **Oh no!**

        Looks like there is no member with the id {}. 
        "#,
            user_id
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };
    let guild = bot
        .get_guild(
            "SELECT guild_id, rankbinds, groupbinds FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let Some(database_user) = bot
        .database
        .query_opt::<RoUser>("SELECT * FROM roblox_users WHERE user_id = $1", &[&user_id])
        .await?
    else {
        tracing::debug!("user is not in the database");
        let message = if args.user.is_some() {
            format!(
                r#"
Oops, I did not find <@{}> in my database. They are not verified with RoWifi.
            "#,
                user_id
            )
        } else {
            format!(
                r#"
Hey there, it looks like you're not verified with us. Please run `/verify` to register with RoWifi.
            "#
            )
        };
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };

    let roblox_id = database_user
        .linked_accounts
        .get(&ctx.guild_id)
        .unwrap_or(&database_user.default_account_id);
    let roblox_user = bot.roblox.get_user(*roblox_id).await?;
    let ranks = bot.roblox.get_user_roles(*roblox_id).await?;
    let thumbnail = bot.roblox.get_user_thumbnail(*roblox_id).await?;

    let mut group_ids = HashSet::new();
    group_ids.extend(guild.rankbinds.iter().map(|r| r.group_id));
    group_ids.extend(guild.groupbinds.iter().map(|g| g.group_id));
    let mut ranks_info = String::new();
    for rank in ranks {
        if group_ids.contains(&rank.group.id) {
            ranks_info.push_str(&format!(
                "{} - {}\n",
                rank.group.name.trim(),
                rank.role.name.unwrap()
            ));
        }
    }

    let roblox_display_name = if let Some(display_name) = roblox_user.display_name {
        format!("{} (@{})", display_name, roblox_user.name)
    } else {
        roblox_user.name.clone()
    };

    let roblox_account = format!(
        r"
**Name:** {}
**Created on:** <t:{}:D>
    ",
        roblox_display_name,
        roblox_user.create_time.unwrap().timestamp()
    );

    let embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title(discord_user.nickname.unwrap_or(discord_user.username))
        .field(EmbedFieldBuilder::new("Roblox Account", roblox_account).build())
        .field(EmbedFieldBuilder::new("Ranks", ranks_info))
        .thumbnail(ImageSource::url(thumbnail).unwrap())
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
