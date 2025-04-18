use itertools::Itertools;
use rowifi_core::custombinds::{
    evaluate::{evaluate, EvaluationContext, EvaluationResult},
    parser::parser,
};
use rowifi_framework::prelude::*;
use rowifi_models::{
    bind::{AssetType, Assetbind, Bind, Custombind, Groupbind, Rankbind},
    deny_list::DenyListData,
    discord::http::interaction::{InteractionResponse, InteractionResponseType},
    guild::BypassRoleKind,
    id::{RoleId, UserId},
    roblox::inventory::InventoryItem,
    user::RoUser,
};
use rowifi_roblox::{error::ErrorKind, filter::AssetFilterBuilder};
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

enum Checks {
    ServerOwner,
    BypassRole,
    Denylist,
}

enum BindRef<'b> {
    Rank(&'b Rankbind),
    Group(&'b Groupbind),
    Custom(&'b Custombind),
    Asset(&'b Assetbind),
    Verified,
}

#[tracing::instrument(skip_all, fields(args = ?args))]
pub async fn debug_update_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: UpdateArguments,
) -> CommandResult {
    let server = bot.server(ctx.guild_id).await?;
    let mut success_checks = Vec::new();
    let mut failed_checks = Vec::new();

    let user_id = match args.user_id {
        Some(s) => s,
        None => ctx.author_id,
    };

    let Some((discord_member, discord_user)) = bot.member(server.id, user_id).await? else {
        // Should not ever happen since slash command guarantees that the user exists.
        // But handling this nonetheless is useful.
        let message = format!("Discord User with id {user_id}  does not exist");
        ctx.respond(bot).content(&message).unwrap().await?;
        return Ok(());
    };

    let Some(user) = bot
        .database
        .query_opt::<RoUser>(
            "SELECT * FROM roblox_users WHERE user_id = $1",
            &[&discord_member.id],
        )
        .await?
    else {
        let message = if args.user_id.is_some() {
            "This user is not verified with RoWifi."
        } else {
            "You are not verified with RoWifi."
        };
        ctx.respond(bot).content(message).unwrap().await?;
        return Ok(());
    };

    if server.owner_id == discord_member.id {
        failed_checks.push(Checks::ServerOwner);
    } else {
        success_checks.push(Checks::ServerOwner);
    }

    let guild = bot
        .get_guild(
            "SELECT guild_id, bypass_roles, unverified_roles, verified_roles, rankbinds, groupbinds, custombinds, assetbinds, deny_lists, default_template, sticky_roles FROM guilds WHERE guild_id = $1",
            server.id,
        )
        .await?;

    let mut active_bypass_role = None;
    for bypass_role in &guild.bypass_roles {
        if bypass_role.kind == BypassRoleKind::All
            && discord_member.roles.contains(&bypass_role.role_id)
        {
            active_bypass_role = Some(bypass_role.role_id);
            failed_checks.push(Checks::BypassRole);
            break;
        }
    }
    if active_bypass_role.is_none() {
        success_checks.push(Checks::BypassRole);
    }

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

    let roblox_user = match bot.roblox.get_user(*user_id).await {
        Ok(u) => u,
        Err(err) => {
            if let ErrorKind::Response {
                route: _,
                status,
                bytes: _,
            } = err.kind()
            {
                if status.as_u16() == 404 {
                    let message = format!("Your selected Roblox account for this server is [this](https://www.roblox.com/users/{user_id}/profile). It seems that Roblox has banned or suspended this account. If this is not the case, please contact the RoWifi support server.");
                    ctx.respond(bot).content(&message).unwrap().await?;
                    return Ok(());
                }
            }
            return Err(err.into());
        }
    };

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
                match parser(c) {
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

    let active_deny_list = active_deny_lists
        .iter()
        .sorted_by_key(|d| d.action_type)
        .next_back();
    if active_deny_list.is_none() {
        success_checks.push(Checks::Denylist);
    } else {
        failed_checks.push(Checks::Denylist);
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
    let mut role_addition_tracking = HashMap::new();

    for verified_role in &guild.verified_roles {
        if server.roles.contains(verified_role) {
            roles_to_add.insert(*verified_role);
            role_addition_tracking.insert(*verified_role, BindRef::Verified);
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
            for role in &rankbind.discord_roles {
                if !role_addition_tracking.contains_key(role) {
                    role_addition_tracking.insert(*role, BindRef::Rank(rankbind));
                }
            }
        }
    }

    for groupbind in &guild.groupbinds {
        if user_ranks.contains_key(&groupbind.group_id) {
            if let Some(ref highest) = nickname_bind {
                if highest.priority() < groupbind.priority {
                    nickname_bind = Some(Bind::Group(groupbind.clone()));
                }
                roles_to_add.extend(groupbind.discord_roles.iter().copied());
                for role in &groupbind.discord_roles {
                    if !role_addition_tracking.contains_key(role) {
                        role_addition_tracking.insert(*role, BindRef::Group(groupbind));
                    }
                }
            }
        }
    }

    let mut custombind_evaluation_failed = Vec::new();
    let mut custombind_parsing_failed = Vec::new();
    for custombind in &guild.custombinds {
        let exp = match parser(&custombind.code) {
            Ok(e) => e,
            Err(err) => {
                custombind_parsing_failed.push((custombind.custom_bind_id, err.to_string()));
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
                custombind_evaluation_failed.push((custombind.custom_bind_id, err.to_string()));
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
            for role in &custombind.discord_roles {
                if !role_addition_tracking.contains_key(role) {
                    role_addition_tracking.insert(*role, BindRef::Custom(custombind));
                }
            }
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
            for role in &assetbind.discord_roles {
                if !role_addition_tracking.contains_key(role) {
                    role_addition_tracking.insert(*role, BindRef::Asset(assetbind));
                }
            }
        }
    }

    let all_guild_roles = bot
        .cache
        .guild_roles(server.roles.clone().into_iter())
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
    let mut warning_roles = HashSet::new();
    for bind_role in all_roles {
        if server.roles.contains(&bind_role) {
            let role_position = all_guild_roles.get(&bind_role).copied().unwrap_or_default();
            if roles_to_add.contains(&bind_role) {
                if !discord_member.roles.contains(&bind_role) {
                    added_roles.push(bind_role);
                    if role_position > highest_bot_position {
                        warning_roles.insert(bind_role);
                    }
                }
            } else if discord_member.roles.contains(&bind_role)
                && !guild.sticky_roles.contains(&bind_role)
            {
                removed_roles.push(bind_role);
                if role_position > highest_bot_position {
                    warning_roles.insert(bind_role);
                }
            }
        }
    }

    let new_nickname = if let Some(nickname_bind) = &nickname_bind {
        match nickname_bind {
            Bind::Rank(r) => {
                r.template
                    .nickname(&roblox_user, user.user_id, &discord_user.username)
            }
            Bind::Group(g) => {
                g.template
                    .nickname(&roblox_user, user.user_id, &discord_user.username)
            }
            Bind::Asset(a) => {
                a.template
                    .nickname(&roblox_user, user.user_id, &discord_user.username)
            }
            Bind::Custom(c) => {
                c.template
                    .nickname(&roblox_user, user.user_id, &discord_user.username)
            }
        }
    } else {
        guild.default_template.as_ref().unwrap().nickname(
            &roblox_user,
            user.user_id,
            &discord_user.username,
        )
    };

    let mut message = String::new();

    message.push_str("**Checks**:\n");
    for check in success_checks {
        match check {
            Checks::BypassRole => message.push_str(
                ":white_check_mark: You do not have a role which has been marked as bypass.\n",
            ),
            Checks::Denylist => message.push_str(":white_check_mark: You are not on a denylist.\n"),
            Checks::ServerOwner => {
                message.push_str(":white_check_mark: You are not the server owner.\n");
            }
        }
    }

    for check in failed_checks {
        match check {
            Checks::BypassRole => {
                let _ = writeln!(
                    message,
                    ":x: You have a role <@&{}> marked as bypass",
                    active_bypass_role.unwrap()
                );
            }
            Checks::Denylist => {
                let active_denylist = active_deny_list.unwrap();
                let _ = writeln!(
                    message,
                    ":x: You are on a denylist: Id {}, Action: {}",
                    active_denylist.id, active_denylist.action_type
                );
            }
            Checks::ServerOwner => message.push_str(":x: You are the server owner\n"),
        }
    }

    message.push_str(&format!("\nNickname: {new_nickname}"));
    if let Some(nickname_bind) = nickname_bind {
        let bind = match nickname_bind {
            Bind::Rank(rank) => format!(
                "Rankbind (Group Id: {}, Rank Id: {})",
                rank.group_id, rank.group_rank_id
            ),
            Bind::Group(group) => format!("Groupbind (Group Id: {})", group.group_id),
            Bind::Custom(custom) => format!("Custombind (Id: {})", custom.custom_bind_id),
            Bind::Asset(asset) => format!("Assetbind (Asset Id: {})", asset.asset_id),
        };
        let _ = write!(message, " - Decided by {bind}");

        if new_nickname.is_empty() {
            message.push_str("- :warning: Nickname has no characters and is considered invalid. This will cause an error.");
        } else if new_nickname.len() > 32 {
            message.push_str(
                "- :warning: Nickname is more than 32 characters. This will cause an error.",
            );
        }
    } else {
        message.push_str("- Decided by Default Template");
    }
    message.push('\n');

    message.push_str("\nAdded Roles:\n");
    for role in &added_roles {
        let _ = write!(message, "- <@&{role}> ");
        if warning_roles.contains(role) {
            message.push_str(" :warning: ");
        }
        let bind = role_addition_tracking.get(role).unwrap();
        let bind_string = match bind {
            BindRef::Rank(rank) => format!(
                "Rankbind (Group Id: {}, Rank Id: {})",
                rank.group_id, rank.group_rank_id
            ),
            BindRef::Group(group) => format!("Groupbind (Group Id: {})", group.group_id),
            BindRef::Custom(custom) => format!("Custombind (Id: {})", custom.custom_bind_id),
            BindRef::Asset(asset) => format!("Assetbind (Asset Id: {})", asset.asset_id),
            BindRef::Verified => "Verified Roles".into(),
        };
        let _ = writeln!(message, "[Added by {bind_string}]");
    }
    if added_roles.is_empty() {
        message.push_str("None\n");
    }

    message.push_str("\nRemoved Roles:\n");
    let mut removed_str = String::new();
    for role in removed_roles {
        let _ = writeln!(removed_str, "- <@&{role}>");
        if warning_roles.contains(&role) {
            message.push_str(" :warning:");
        }
        message.push('\n');
    }
    if removed_str.is_empty() {
        removed_str = "None\n".into();
    }
    message.push_str(&removed_str);

    message.push_str(
        "\nRoles marked :warning: are above the bot and may cause issues while updating\n",
    );

    message.push_str("\nThis list of checks is not exhaustive. Despite these checks, if you’re unable to resolve your issue, please contact the support server for assistance.");

    ctx.respond(bot).content(&message).unwrap().await?;

    Ok(())
}
