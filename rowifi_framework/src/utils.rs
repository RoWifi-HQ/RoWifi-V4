use rowifi_core::error::RoError;
use rowifi_models::discord::{
    application::interaction::{Interaction, InteractionData, InteractionType},
    channel::message::{
        component::{ActionRow, Button, ButtonStyle},
        Component, Embed, MessageFlags, ReactionType,
    },
};
use std::{cmp::min, time::Duration};
use tokio_stream::StreamExt;
use twilight_standby::Standby;

use crate::context::{BotContext, CommandContext};

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
            .exec()
            .await?;
    } else {
        let message = ctx
            .respond(bot)
            .embeds(&[pages[0].clone()])?
            .components(&[Component::ActionRow(ActionRow {
                components: vec![
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(ReactionType::Unicode {
                            name: "⏮️".into()
                        }),
                        label: Some("First Page".into()),
                        custom_id: Some("first-page".into()),
                        url: None,
                        disabled: false,
                    }),
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(ReactionType::Unicode {
                            name: "◀️".into()
                        }),
                        label: Some("Previous Page".into()),
                        custom_id: Some("previous-page".into()),
                        url: None,
                        disabled: false,
                    }),
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(ReactionType::Unicode {
                            name: "▶️".into()
                        }),
                        label: Some("Next Page".into()),
                        custom_id: Some("next-page".into()),
                        url: None,
                        disabled: false,
                    }),
                    Component::Button(Button {
                        style: ButtonStyle::Primary,
                        emoji: Some(ReactionType::Unicode {
                            name: "⏭️".into()
                        }),
                        label: Some("Last Page".into()),
                        custom_id: Some("last-page".into()),
                        url: None,
                        disabled: false,
                    }),
                ],
            })])?
            .exec()
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
            let data = match &interaction.data {
                Some(InteractionData::MessageComponent(data)) => data,
                _ => unreachable!(),
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
