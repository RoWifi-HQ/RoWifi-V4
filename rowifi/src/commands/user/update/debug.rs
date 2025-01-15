use itertools::Itertools;
use rowifi_core::custombinds::{
    evaluate::{evaluate, EvaluationContext, EvaluationResult},
    parser::parser,
};
use rowifi_framework::prelude::*;
use rowifi_models::{
    bind::{AssetType, Bind},
    deny_list::DenyListData,
    discord::http::interaction::{InteractionResponse, InteractionResponseType},
    guild::BypassRoleKind,
    id::{RoleId, UserId},
    roblox::inventory::InventoryItem,
    user::RoUser,
};
use rowifi_roblox::filter::AssetFilterBuilder;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

#[derive(Arguments, Debug)]
pub struct UpdateArguments {
    pub user_id: Option<UserId>,
}

pub async fn debug_update(
    bot: Extension<BotContext>,
    command: Command<UpdateArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = debug_update_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn debug_update_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateArguments,
) -> CommandResult {
    let server = bot.server(ctx.guild_id).await?;

    let user_id = match args.user_id {
        Some(s) => s,
        None => ctx.author_id,
    };

    let Some((discord_member, discord_user)) = bot.member(server.id, user_id).await? else {
        // Should not ever happen since slash command guarantees that the user exists.
        // But handling this nonetheless is useful.
        let message = format!(
            r#"
**Checks**:
:x: Discord User with id {}  does not exist
        "#,
            user_id
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };

    if server.owner_id == discord_member.id {
        let message = r"
**Checks**:
:white_check_mark: Discord User Exists.
:x: The command is being run on the server owner. Discord has significant limitations around server owners.
        ";
        ctx.respond(&bot).content(message).unwrap().await?;
        return Ok(());
    }

    let guild = bot
        .get_guild(
            "SELECT guild_id, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, sticky_roles FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;

    for bypass_role in &guild.bypass_roles {
        if bypass_role.kind == BypassRoleKind::All && discord_member.roles.contains(&bypass_role.role_id) {
            let message = format!(
                r"
**Checks**:
:white_check_mark: Discord User Exists.
:white_check_mark: You are not the server owner.
:x: You have a role (<@&{}>) which has been marked as a bypass role.
    ",
                bypass_role.role_id
            );
            ctx.respond(&bot).content(&message).unwrap().await?;
            return Ok(());
        }
    }

    let Some(user) = bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&discord_member.id],
        )
        .await?
    else {
        let verify_message = if args.user_id.is_some() {
            "This user has not linked any Roblox account."
        } else {
            "You have not linked any Roblox account."
        };
        let message = format!(
            r"
**Checks**:
:white_check_mark: Discord User Exists.
:white_check_mark: You are not the server owner.
:white_check_mark: You do not have a role which has been marked as bypass.
:x: {}
",
            verify_message
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    };

    let mut all_roles = guild
        .rankbinds
        .iter()
        .flat_map(|b| b.discord_roles.clone())
        .collect::<Vec<_>>();
    all_roles.extend(
        guild
            .groupbinds
            .iter()
            .flat_map(|b| b.discord_roles.clone()),
    );
    all_roles.extend(
        guild
            .custombinds
            .iter()
            .flat_map(|b| b.discord_roles.clone()),
    );
    all_roles.extend(
        guild
            .assetbinds
            .iter()
            .flat_map(|b| b.discord_roles.clone()),
    );
    all_roles.extend(&guild.unverified_roles);
    all_roles.extend(&guild.verified_roles);
    all_roles = all_roles.into_iter().unique().collect();

    let user_id = user
        .linked_accounts
        .get(&guild.guild_id)
        .unwrap_or(&user.default_account_id);

    let user_ranks = bot
        .roblox
        .get_user_roles(*user_id)
        .await?
        .into_iter()
        .map(|r| (r.group.id, r.role.rank))
        .collect::<HashMap<_, _>>();

    let roblox_user = bot.roblox.get_user(*user_id).await?;

    let mut active_deny_lists = Vec::new();
    let mut evaluation_failed = Vec::new();
    let mut parsing_failed = Vec::new();
    for denylist in &guild.deny_lists {
        let success = match &denylist.data {
            DenyListData::User(u) => *u == roblox_user.id,
            DenyListData::Group(g) => user_ranks.contains_key(g),
            DenyListData::Custom(c) => {
                // TODO: Figure out a better way to hold the expression of custom
                // denylists in memory
                match parser(&c) {
                    Ok(exp) => {
                        let res = match evaluate(
                            &exp,
                            &EvaluationContext {
                                roles: &discord_member.roles,
                                ranks: &user_ranks,
                                username: &roblox_user.name,
                            },
                        ) {
                            Ok(res) => res,
                            Err(err) => {
                                evaluation_failed.push((denylist.id, err.to_string()));
                                continue;
                            }
                        };
                        match res {
                            EvaluationResult::Bool(b) => b,
                            EvaluationResult::Number(n) => n != 0,
                        }
                    }
                    Err(err) => {
                        parsing_failed.push((denylist.id, err.to_string()));
                        continue;
                    }
                }
            }
        };
        if success {
            active_deny_lists.push(denylist);
        }
    }

    if !evaluation_failed.is_empty() || !parsing_failed.is_empty() {
        let mut message = r"
**Checks**:
:white_check_mark: Discord User Exists.
:white_check_mark: You are not the server owner.
:white_check_mark: You do not have a role which has been marked as bypass.
:white_check_mark: You have a valid linked Roblox account.
"
        .to_string();
        if !evaluation_failed.is_empty() {
            let mut eval_message =
                "\n:x: Following Custom Denylists failed to evaluate:\n".to_string();
            for (id, err) in &evaluation_failed {
                eval_message.push_str(&format!("- ID: {} -> Error: {}", id, err));
            }
            message.push_str(&eval_message);
        }
        if !parsing_failed.is_empty() {
            let mut eval_message =
                "\n:x: Following Custom Denylists failed to parse:\n".to_string();
            for (id, err) in &evaluation_failed {
                eval_message.push_str(&format!("- ID: {} -> Error: {}", id, err));
            }
            message.push_str(&eval_message);
        }
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let active_deny_list = active_deny_lists
        .iter()
        .sorted_by_key(|d| d.action_type)
        .last();
    if let Some(deny_list) = active_deny_list {
        let message = format!(
            "
**Checks**:
:white_check_mark: Discord User Exists.
:white_check_mark: You are not the server owner.
:white_check_mark: You do not have a role which has been marked as bypass.
:white_check_mark: You have a valid linked Roblox account.
:x: You are on a denylist (ID: {})
",
            deny_list.id
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let mut asset_filter = AssetFilterBuilder::new();
    for assetbind in &guild.assetbinds {
        match assetbind.asset_type {
            AssetType::Asset => asset_filter = asset_filter.asset(assetbind.asset_id),
            AssetType::Badge => asset_filter = asset_filter.badge(assetbind.asset_id),
            AssetType::Gamepass => asset_filter = asset_filter.gamepass(assetbind.asset_id),
        }
    }
    let inventory_items = bot
        .roblox
        .get_inventory_items(*user_id, asset_filter)
        .await?
        .into_iter()
        .map(|i| match i {
            InventoryItem::Asset(a) => a.asset_id,
            InventoryItem::Badge(b) => b.badge_id,
            InventoryItem::Gamepass(g) => g.gamepass_id,
        })
        .collect::<HashSet<_>>();

    let mut roles_to_add = HashSet::<RoleId>::new();
    let mut nickname_bind: Option<Bind> = None;

    for verified_role in &guild.verified_roles {
        if server.roles.contains(verified_role) {
            roles_to_add.insert(*verified_role);
        }
    }

    for rankbind in &guild.rankbinds {
        // Check if the user's rank in the group is the same as the rankbind
        // or check if the bind is for the Guest role and the user is not in
        // the group
        let to_add = match user_ranks.get(&rankbind.group_id) {
            Some(rank_id) => *rank_id == rankbind.group_rank_id,
            None => rankbind.group_rank_id == 0,
        };
        if to_add {
            if let Some(ref highest) = nickname_bind {
                if highest.priority() < rankbind.priority {
                    nickname_bind = Some(Bind::Rank(rankbind.clone()));
                }
            } else {
                nickname_bind = Some(Bind::Rank(rankbind.clone()));
            }
            roles_to_add.extend(rankbind.discord_roles.iter().copied());
        }
    }

    for groupbind in &guild.groupbinds {
        if user_ranks.contains_key(&groupbind.group_id) {
            if let Some(ref highest) = nickname_bind {
                if highest.priority() < groupbind.priority {
                    nickname_bind = Some(Bind::Group(groupbind.clone()));
                }
                roles_to_add.extend(groupbind.discord_roles.iter().copied());
            }
        }
    }

    // TODO: Have parsed custombinds stored somewhere
    evaluation_failed.clear();
    parsing_failed.clear();
    for custombind in &guild.custombinds {
        let exp = match parser(&custombind.code) {
            Ok(e) => e,
            Err(err) => {
                parsing_failed.push((custombind.custom_bind_id, err.to_string()));
                continue;
            }
        };
        let res = match evaluate(
            &exp,
            &EvaluationContext {
                roles: &discord_member.roles,
                ranks: &user_ranks,
                username: &roblox_user.name,
            },
        ) {
            Ok(res) => res,
            Err(err) => {
                evaluation_failed.push((custombind.custom_bind_id, err.to_string()));
                continue;
            }
        };
        let success = match res {
            EvaluationResult::Bool(b) => b,
            EvaluationResult::Number(n) => n != 0,
        };
        if success {
            if let Some(ref highest) = nickname_bind {
                if highest.priority() < custombind.priority {
                    nickname_bind = Some(Bind::Custom(custombind.clone()));
                }
            } else {
                nickname_bind = Some(Bind::Custom(custombind.clone()));
            }
            roles_to_add.extend(custombind.discord_roles.iter().copied());
        }
    }

    for assetbind in &guild.assetbinds {
        if inventory_items.contains(&assetbind.asset_id.0.to_string()) {
            if let Some(ref highest) = nickname_bind {
                if highest.priority() < assetbind.priority {
                    nickname_bind = Some(Bind::Asset(assetbind.clone()));
                }
            } else {
                nickname_bind = Some(Bind::Asset(assetbind.clone()));
            }
            roles_to_add.extend(assetbind.discord_roles.iter().copied());
        }
    }

    if !evaluation_failed.is_empty() || !parsing_failed.is_empty() {
        let mut message = r"
**Checks**:
:white_check_mark: Discord User Exists.
:white_check_mark: You are not the server owner.
:white_check_mark: You do not have a role which has been marked as bypass.
:white_check_mark: You have a valid linked Roblox account.
:white_check_mark: You are not on a denylist.
"
        .to_string();
        if !evaluation_failed.is_empty() {
            let mut eval_message = "\n:x: Following Custombinds failed to evaluate:\n".to_string();
            for (id, err) in &evaluation_failed {
                eval_message.push_str(&format!("- ID: {} -> Error: {}", id, err));
            }
            message.push_str(&eval_message);
        }
        if !parsing_failed.is_empty() {
            let mut eval_message = "\n:x: Following Custombinds failed to parse:\n".to_string();
            for (id, err) in &evaluation_failed {
                eval_message.push_str(&format!("- ID: {} -> Error: {}", id, err));
            }
            message.push_str(&eval_message);
        }
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let all_guild_roles = bot
        .cache
        .guild_roles(all_roles.clone().into_iter())
        .await?
        .into_iter()
        .map(|r| (r.id, r.position))
        .collect::<HashMap<_, _>>();
    let Some((bot_member, _bot_user)) = bot
        .member(ctx.guild_id, UserId::new(bot.application_id.get()))
        .await?
    else {
        return Ok(());
    };
    let highest_bot_position = bot_member
        .roles
        .iter()
        .map(|r| all_guild_roles.get(r).copied().unwrap_or_default())
        .max()
        .unwrap_or_default();

    let mut added_roles = Vec::new();
    let mut removed_roles = Vec::new();
    let mut warning_roles = Vec::new();
    for bind_role in all_roles {
        if server.roles.contains(&bind_role) {
            let role_position = all_guild_roles.get(&bind_role).copied().unwrap_or_default();
            if roles_to_add.contains(&bind_role) {
                if !discord_member.roles.contains(&bind_role) {
                    added_roles.push(bind_role);
                    if role_position > highest_bot_position {
                        warning_roles.push(bind_role);
                    }
                }
            } else {
                if discord_member.roles.contains(&bind_role) && !guild.sticky_roles.contains(&bind_role) {
                    removed_roles.push(bind_role);
                    if role_position > highest_bot_position {
                        warning_roles.push(bind_role);
                    }
                }
            }
        }
    }

    let new_nickname = if let Some(nickname_bind) = nickname_bind {
        match nickname_bind {
            Bind::Rank(r) => r
                .template
                .nickname(&roblox_user, user.user_id, &discord_user.username),
            Bind::Group(g) => g
                .template
                .nickname(&roblox_user, user.user_id, &discord_user.username),
            Bind::Asset(a) => a
                .template
                .nickname(&roblox_user, user.user_id, &discord_user.username),
            Bind::Custom(c) => c
                .template
                .nickname(&roblox_user, user.user_id, &discord_user.username),
        }
    } else {
        guild.default_template.as_ref().unwrap().nickname(
            &roblox_user,
            user.user_id,
            &discord_user.username,
        )
    };

    let mut message = r"
**Checks**:
:white_check_mark: Discord User Exists.
:white_check_mark: You are not the server owner.
:white_check_mark: You do not have a role which has been marked as bypass.
:white_check_mark: You have a valid linked Roblox account.
:white_check_mark: You are not on a denylist.
"
    .to_string();

    if new_nickname.len() > 32 {
        message.push_str("\n:exclamation: Supposed nickname is greater than 32 characters");
    }

    message.push_str(&format!("\n\nNickname: {}", new_nickname));

    let mut added_str = added_roles.iter().fold(String::new(), |mut s, a| {
        let _ = write!(s, "- <@&{}>\n", a.0);
        s
    });
    let mut removed_str = removed_roles.iter().fold(String::new(), |mut s, a| {
        let _ = write!(s, "- <@&{}>\n", a.0);
        s
    });
    if added_str.is_empty() {
        added_str = "None".into();
    }
    if removed_str.is_empty() {
        removed_str = "None".into();
    }

    message.push_str(&format!(
        "\n\n**Roles to add**:\n{}\n**Roles to remove**:\n{}",
        added_str, removed_str
    ));

    if !warning_roles.is_empty() {
        message.push_str(&format!(
            "\n\nThe bot will attempt to add/remove ({}) which will result in an error. These roles are higher than the bot's highest role. To resolve this, make sure the bot's highest role is higher than these roles or unbind these roles",
            warning_roles.iter().map(|r| format!("<@&{}>", r)).join(",")
        ));
    }

    message.push_str("\n\n⚠️ This list of checks is not exhaustive. Despite these checks, if you’re unable to resolve your issue, please contact the support server for assistance.");

    ctx.respond(&bot).content(&message).unwrap().await?;

    Ok(())
}
