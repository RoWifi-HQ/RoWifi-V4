use rowifi_framework::prelude::*;
use rowifi_models::discord::{
    channel::message::{
        component::{ActionRow, Button, ButtonStyle},
        Component,
    },
    http::interaction::{InteractionResponse, InteractionResponseType},
    util::Timestamp,
};

pub async fn verify_route() -> impl IntoResponse {
    let embed = EmbedBuilder::new()
        .color(BLUE)
        .footer(EmbedFooterBuilder::new("RoWifi").build())
        .timestamp(Timestamp::from_secs(OffsetDateTime::now_utc().unix_timestamp()).unwrap())
        .title("Verification Process")
        .description("To link your account, click the button below")
        .build();

    let component = Component::ActionRow(ActionRow {
        components: vec![Component::Button(Button {
            custom_id: None,
            disabled: false,
            emoji: None,
            label: Some("Link Account".into()),
            url: Some("https://dashboard.rowifi.xyz/auth/roblox".into()),
            style: ButtonStyle::Link,
        })],
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(
            InteractionResponseDataBuilder::new()
                .embeds([embed])
                .components([component])
                .build(),
        ),
    })
}
