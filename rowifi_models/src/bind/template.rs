use bytes::BytesMut;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    sync::LazyLock,
};
use tokio_postgres::types::{to_sql_checked, FromSql, IsNull, ToSql, Type};

use crate::{id::UserId, roblox::user::PartialUser};

static TEMPLATE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(.*?)\}").unwrap());

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Template(pub String);

impl Template {
    #[must_use]
    pub fn nickname(
        &self,
        roblox_user: &PartialUser,
        discord_id: UserId,
        discord_name: &str,
    ) -> String {
        let roblox_id = roblox_user.id.0.to_string();
        let discord_id = discord_id.to_string();
        let display_name = roblox_user.display_name.clone().unwrap_or_default();

        let template_str = &self.0;
        let mut parts = vec![];

        let mut matches = TEMPLATE_REGEX
            .find_iter(template_str)
            .map(|m| (m.start(), m.end()))
            .peekable();
        let first = match matches.peek() {
            Some((start, _)) => *start,
            None => return template_str.clone(),
        };

        if first > 0 {
            parts.push(&template_str[0..first]);
        }

        let mut previous_end = first;
        for (start, end) in matches {
            if previous_end != start {
                parts.push(&template_str[previous_end..start]);
            }

            let arg = &template_str[start..end];
            let arg_name = &arg[1..arg.len() - 1];
            match arg_name {
                "roblox-username" => parts.push(&roblox_user.name),
                "roblox-id" => parts.push(&roblox_id),
                "discord-id" => parts.push(&discord_id),
                "discord-name" => parts.push(discord_name),
                "display-name" => parts.push(&display_name),
                _ => parts.push(arg),
            }

            previous_end = end;
        }

        if previous_end < template_str.len() {
            parts.push(&template_str[previous_end..]);
        }

        parts.join("")
    }
}

impl Default for Template {
    fn default() -> Self {
        Self("{roblox-username}".to_string())
    }
}

impl ToSql for Template {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        String::to_sql(&self.0, ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        <String as ToSql>::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a> FromSql<'a> for Template {
    fn from_sql(
        ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        Ok(Template(String::from_sql(ty, raw)?))
    }

    fn accepts(ty: &Type) -> bool {
        <String as FromSql>::accepts(ty)
    }
}

impl Display for Template {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}
