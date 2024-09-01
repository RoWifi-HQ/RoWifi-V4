use rowifi_core::error::RoError;
use rowifi_models::discord::{
    application::interaction::{Interaction, InteractionData, InteractionType},
    channel::message::{
        component::{ActionRow, Button, ButtonStyle},
        Component, Embed, EmojiReactionType, MessageFlags,
    },
};
use std::{cmp::min, time::Duration};
use tokio_stream::StreamExt;
use twilight_standby::Standby;

use crate::context::{BotContext, CommandContext};

/// Utility method to paginate a list of embed using button components in a message.
///
/// # Errors
///
/// Returns a Discord error wrapped in a [`RoError`].
#[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
pub async fn paginate_embeds(
    ctx: &CommandContext,
    bot: &BotContext,
    standby: &Standby,
    pages: Vec<Embed>,
    page_count: usize,
) -> Result<(), RoError> {
    if page_count <= 1 {
        ctx.respond(bot)
            .embeds(&[pages[0].clone()])
            .unwrap()
            .await?;
    } else {
        let message = ctx
            .respond(bot)
            .embeds(&[pages[0].clone()])?
            .components(&[Component::ActionRow(ActionRow {
                components: vec![
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(EmojiReactionType::Unicode {
                            name: "⏮️".into()
                        }),
                        label: Some("First Page".into()),
                        custom_id: Some("first-page".into()),
                        url: None,
                        disabled: false,
                        sku_id: None,
                    }),
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(EmojiReactionType::Unicode {
                            name: "◀️".into()
                        }),
                        label: Some("Previous Page".into()),
                        custom_id: Some("previous-page".into()),
                        url: None,
                        disabled: false,
                        sku_id: None,
                    }),
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(EmojiReactionType::Unicode {
                            name: "▶️".into()
                        }),
                        label: Some("Next Page".into()),
                        custom_id: Some("next-page".into()),
                        url: None,
                        disabled: false,
                        sku_id: None,
                    }),
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(EmojiReactionType::Unicode {
                            name: "⏭️".into()
                        }),
                        label: Some("Last Page".into()),
                        custom_id: Some("last-page".into()),
                        url: None,
                        disabled: false,
                        sku_id: None,
                    }),
                ],
            })])?
            .await?
            .model()
            .await?;

        let component_interaction = standby
            .wait_for_component_stream(message.id, |interaction: &Interaction| {
                interaction.kind == InteractionType::MessageComponent
            })
            .timeout(Duration::from_secs(300));
        tokio::pin!(component_interaction);

        let mut page_pointer: usize = 0;
        while let Some(Ok(interaction)) = component_interaction.next().await {
            // There's a guarantee that it is a message component
            let Some(InteractionData::MessageComponent(data)) = &interaction.data else {
                unreachable!()
            };
            if interaction.author_id().unwrap() == ctx.author_id.0 {
                match data.custom_id.as_str() {
                    "first-page" => {
                        page_pointer = 0;
                    }
                    "previous-page" => {
                        page_pointer = page_pointer.saturating_sub(1);
                    }
                    "next-page" => {
                        page_pointer = min(page_pointer + 1, page_count - 1);
                    }
                    "last-page" => {
                        page_pointer = page_count - 1;
                    }
                    _ => {}
                }

                let res = bot
                    .http
                    .interaction(bot.application_id)
                    .update_response(&interaction.token)
                    .embeds(Some(&[pages[page_pointer].clone()]))
                    .await;
                if let Err(err) = res {
                    tracing::error!(err = ?err);
                }
            } else {
                let _ = bot
                    .http
                    .interaction(bot.application_id)
                    .create_followup(&interaction.token)
                    .flags(MessageFlags::EPHEMERAL)
                    .content("This view menu is only navigable by the original command invoker")
                    .await;
            }
        }
    }

    Ok(())
}
