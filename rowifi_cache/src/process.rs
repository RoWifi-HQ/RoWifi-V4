use redis::Pipeline;
use rowifi_models::{
    discord::{
        cache::{CachedChannel, CachedGuild, CachedMember, CachedRole, CachedTextChannel, CachedUser},
        channel::{Channel, ChannelType},
        guild::{Guild, Member, PartialMember, Role},
        user::User,
    },
    id::{ChannelId, GuildId, RoleId, UserId},
};

use crate::error::CacheError;

pub(crate) fn cache_guild(
    pipeline: &mut Pipeline,
    guild: &Guild,
) -> Result<CachedGuild, CacheError> {
    for channel in &guild.channels {
        cache_guild_channel(pipeline, channel)?;
    }

    for role in &guild.roles {
        cache_role(pipeline, role)?;
    }

    let cached = CachedGuild::from_guild(guild);

    pipeline.set(CachedGuild::key(cached.id), rmp_serde::to_vec(&cached)?);

    Ok(cached)
}

pub(crate) fn cache_guild_channel(
    pipeline: &mut Pipeline,
    channel: &Channel,
) -> Result<(), CacheError> {
    #[allow(clippy::single_match)]
    match channel.kind {
        ChannelType::GuildText => cache_text_channel(pipeline, channel)?,
        _ => {}
    }

    Ok(())
}

pub(crate) fn cache_text_channel(
    pipeline: &mut Pipeline,
    channel: &Channel,
) -> Result<(), CacheError> {
    if let Some(guild_id) = channel.guild_id {
        let cached = CachedTextChannel {
            id: ChannelId::new(channel.id.get()),
            guild_id: GuildId::new(guild_id.get()),
            name: channel.name.clone().unwrap_or_default(),
            permission_overwrites: channel.permission_overwrites.as_ref().unwrap().clone(),
        };

        pipeline.set(CachedChannel::key(cached.id), rmp_serde::to_vec(&cached)?);
    }

    Ok(())
}

pub(crate) fn cache_role(pipeline: &mut Pipeline, role: &Role) -> Result<(), CacheError> {
    let cached = CachedRole {
        id: RoleId::new(role.id.get()),
        name: role.name.clone(),
        permissions: role.permissions,
        managed: role.managed,
        position: role.position,
        color: role.color,
    };

    pipeline.set(CachedRole::key(cached.id), rmp_serde::to_vec(&cached)?);

    Ok(())
}

pub(crate) fn cache_member(
    pipeline: &mut Pipeline,
    guild_id: GuildId,
    member: &Member,
) -> Result<CachedMember, CacheError> {
    let cached = CachedMember {
        id: UserId(member.user.id),
        roles: member.roles.iter().map(|r| RoleId(*r)).collect(),
        nickname: member.nick.clone(),
        avatar: member.avatar.map(|a| a.to_string()),
    };

    pipeline.set(
        CachedMember::key(guild_id, cached.id),
        rmp_serde::to_vec(&cached)?,
    );

    Ok(cached)
}

pub(crate) fn cache_partial_member(
    pipeline: &mut Pipeline,
    guild_id: GuildId,
    member: &PartialMember,
    user: &User,
) -> Result<(), CacheError> {
    let cached = CachedMember {
        id: UserId(user.id),
        roles: member.roles.iter().map(|r| RoleId(*r)).collect(),
        nickname: member.nick.clone(),
        avatar: member.avatar.map(|a| a.to_string()),
    };

    pipeline.set(
        CachedMember::key(guild_id, cached.id),
        rmp_serde::to_vec(&cached)?,
    );

    Ok(())
}

pub(crate) fn cache_user(
    pipeline: &mut Pipeline,
    user: &User,
) -> Result<CachedUser, CacheError> {
    let cached = CachedUser {
        id: UserId(user.id),
        username: user.name.clone(),
        avatar: user.avatar.map(|a| a.to_string()),
    };

    pipeline.set(
        CachedUser::key(cached.id),
        rmp_serde::to_vec(&cached)?,
    );

    Ok(cached)
}
