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
        .timestamp(Timestamp::from_secs(Utc::now().timestamp()).unwrap())
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
            sku_id: None,
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
