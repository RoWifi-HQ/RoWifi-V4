use rowifi_core::groupbinds::add::{add_groupbind, AddGroupbindError, GroupbindArguments};
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
use std::collections::HashMap;
use twilight_mention::Mention;

#[derive(Arguments, Debug)]
pub struct GroupbindRouteArguments {
    pub group_id: u64,
    pub template: String,
    pub priority: Option<i32>,
    pub discord_roles: Vec<RoleId>,
}

pub async fn new_groupbind(
    bot: Extension<BotContext>,
    command: Command<GroupbindRouteArguments>,
) -> impl IntoResponse {
    spawn_command(new_groupbind_func(bot, command.ctx, command.args));

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn new_groupbind_func(
    bot: Extension<BotContext>,
    ctx: CommandContext,
    args: GroupbindRouteArguments,
) -> CommandResult {
    tracing::debug!("groupbinds new invoked");
    let guild = bot
        .get_guild(
            "SELECT guild_id, groupbinds FROM guilds WHERE guild_id = $1",
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

    let res = match add_groupbind(
        &bot.roblox,
        &bot.database,
        ctx.guild_id,
        ctx.author_id,
        &guild.groupbinds,
        &server_roles,
        GroupbindArguments {
            group_id: GroupId(args.group_id),
            template: Template(args.template),
            discord_roles: args.discord_roles,
            priority: args.priority,
        },
    )
    .await
    {
        Ok(res) => res,
        Err(AddGroupbindError::InvalidGroup) => {
            let message = format!(
                r#"
    Oh no! There does not seem to be a group with ID {}
            "#,
                args.group_id
            );
            ctx.respond(&bot).content(&message).unwrap().await?;
            return Ok(());
        }
        Err(AddGroupbindError::Generic(err)) => return Err(err),
    };

    let mut description = String::new();
    if res.modified {
        description.push_str(":warning: Bind already exists. Modified it to:\n\n");
    }
    description.push_str(&format!("**Group Id: {}**\n", res.bind.group_id));
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
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
        .title("Bind Addition Successful")
        .description(description)
        .build();
    ctx.respond(&bot).embeds(&[embed]).unwrap().await?;

    Ok(())
}
