pub use twilight_model::*;

pub mod cache {
    use serde::{Deserialize, Serialize};
    use std::collections::HashSet;
    use twilight_model::{
        channel::permission_overwrite::PermissionOverwrite,
        guild::{Guild, Permissions},
        util::ImageHash,
    };

    use crate::id::{ChannelId, GuildId, RoleId, UserId};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum CachedChannel {
        Text(CachedTextChannel),
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CachedTextChannel {
        pub id: ChannelId,
        pub guild_id: GuildId,
        pub name: String,
        pub permission_overwrites: Vec<PermissionOverwrite>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CachedGuild {
        pub id: GuildId,
        pub name: String,
        pub icon: Option<ImageHash>,
        pub member_count: u64,
        pub owner_id: UserId,
        pub roles: HashSet<RoleId>,
        pub channels: HashSet<ChannelId>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CachedMember {
        pub roles: Vec<RoleId>,
        pub nickname: Option<String>,
        pub id: UserId,
        pub username: String,
        pub discriminator: u16,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct CachedRole {
        pub id: RoleId,
        pub name: String,
        pub permissions: Permissions,
        pub managed: bool,
        pub position: i64,
        pub color: u32,
    }

    impl CachedChannel {
        #[must_use]
        pub fn key(id: ChannelId) -> String {
            format!("discord:channels:{id}")
        }

        #[must_use]
        pub fn name(&self) -> &str {
            match self {
                CachedChannel::Text(c) => &c.name,
            }
        }
    }

    impl CachedGuild {
        #[must_use]
        pub fn key(id: GuildId) -> String {
            format!("discord:guilds:{id}")
        }

        #[must_use]
        pub fn from_guild(guild: &Guild) -> Self {
            Self {
                id: GuildId::new(guild.id.get()),
                name: guild.name.clone(),
                owner_id: UserId::new(guild.owner_id.get()),
                icon: guild.icon,
                member_count: guild.member_count.unwrap_or_default(),
                roles: guild.roles.iter().map(|r| RoleId(r.id)).collect(),
                channels: guild.channels.iter().map(|c| ChannelId(c.id)).collect(),
            }
        }
    }

    impl CachedMember {
        #[must_use]
        pub fn key(guild_id: GuildId, user_id: UserId) -> String {
            format!("discord:m:{guild_id}:{user_id}")
        }
    }

    impl CachedRole {
        #[must_use]
        pub fn key(id: RoleId) -> String {
            format!("discord:roles:{id}")
        }
    }
}
