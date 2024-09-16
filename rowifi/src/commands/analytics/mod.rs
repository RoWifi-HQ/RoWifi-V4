use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use plotters::{
    chart::{ChartBuilder, LabelAreaPosition},
    prelude::{BitMapBackend, IntoDrawingArea},
    series::LineSeries,
    style::{IntoFont, WHITE},
};
use rowifi_framework::{arguments::ArgumentError, prelude::*};
use rowifi_models::{
    analytics::AnalyticsGroup,
    discord::{
        application::interaction::application_command::{CommandDataOption, CommandOptionValue},
        http::{
            attachment::Attachment,
            interaction::{InteractionResponse, InteractionResponseType},
        },
    },
    guild::GuildType,
    roblox::id::GroupId,
};
use std::{io::Cursor, time::Duration};

#[derive(Debug)]
pub struct ViewDuration(pub Duration);

#[derive(Arguments, Debug)]
pub struct ViewArguments {
    pub group_id: GroupId,
    pub duration: Option<ViewDuration>,
    pub rank_id: Option<u32>,
}

pub async fn analytics_view(
    bot: Extension<BotContext>,
    command: Command<ViewArguments>,
) -> impl IntoResponse {
    tokio::spawn(async move {
        if let Err(err) = analytics_view_func(&bot, &command.ctx, command.args).await {
            handle_error(bot.0, command.ctx, err).await;
        }
    });

    Json(InteractionResponse {
        kind: InteractionResponseType::DeferredChannelMessageWithSource,
        data: None,
    })
}

pub async fn analytics_view_func(
    bot: &BotContext,
    ctx: &CommandContext,
    args: ViewArguments,
) -> CommandResult {
    let guild = bot
        .get_guild(
            "SELECT guild_id, kind, registered_groups FROM guilds WHERE guild_id = $1",
            ctx.guild_id,
        )
        .await?;

    let kind = guild.kind.unwrap_or_default();
    if kind != GuildType::Gamma {
        let message = "The `analytics` module is only available for Gamma Tier servers";
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    if !guild.registered_groups.contains(&args.group_id) {
        let message = format!(
            "Group with ID {} is not registered for analytics.",
            args.group_id
        );
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let server = bot.server(ctx.guild_id).await?;

    let duration = args
        .duration
        .unwrap_or_else(|| ViewDuration(Duration::from_secs(60 * 60 * 24 * 7)))
        .0;
    let start_time = Utc::now() - duration;
    let mut group_data = bot
        .database
        .query::<AnalyticsGroup>(
            "SELECT * FROM group_analytics WHERE group_id = $1 and timestamp > $2",
            &[&args.group_id, &start_time],
        )
        .await?;
    group_data.sort_unstable_by_key(|k| k.timestamp);

    if group_data.len() <= 2 {
        let message = "There is not enough usable data for the given timeframe. Please give the bot 24 hours to collect enough data or use another timeframe";
        ctx.respond(&bot).content(&message).unwrap().await?;
        return Ok(());
    }

    let min_timestamp = group_data.iter().map(|g| g.timestamp).min().unwrap();
    let max_timestamp = group_data.iter().map(|g| g.timestamp).max().unwrap();

    #[allow(clippy::option_if_let_else)]
    let (min_members, max_members, iterator) = if let Some(rank_id) = args.rank_id {
        let mut min_members = group_data
            .iter()
            .map(|g| {
                g.roles
                    .iter()
                    .find(|r| r.rank == rank_id)
                    .map(|r| r.member_count)
                    .unwrap_or_default()
            })
            .min()
            .unwrap_or_default();
        let mut max_members = group_data
            .iter()
            .map(|g| {
                g.roles
                    .iter()
                    .find(|r| r.rank == rank_id)
                    .map(|r| r.member_count)
                    .unwrap_or_default()
            })
            .max()
            .unwrap_or_default();
        let diff = max_members - min_members;
        min_members -= diff / 10;
        max_members += diff / 10;
        let iterator = group_data
            .iter()
            .map(|g| {
                (
                    g.timestamp,
                    g.roles
                        .iter()
                        .find(|r| r.rank == rank_id)
                        .map(|r| r.member_count)
                        .unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();
        (min_members, max_members, iterator)
    } else {
        let mut min_members = group_data.iter().map(|g| g.member_count).min().unwrap();
        let mut max_members = group_data.iter().map(|g| g.member_count).max().unwrap();
        let diff = max_members - min_members;
        min_members -= diff / 10;
        max_members += diff / 10;
        let iterator = group_data
            .iter()
            .map(|g| (g.timestamp, g.member_count))
            .collect::<Vec<_>>();
        (min_members, max_members, iterator)
    };

    let mut buffer = vec![0_u8; 1024 * 768 * 3];
    {
        let root_drawing_area =
            BitMapBackend::with_buffer(&mut buffer, (1024, 768)).into_drawing_area();
        root_drawing_area.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root_drawing_area)
            .caption(server.name.clone(), ("sans-serif", 30).into_font())
            .margin(10)
            .set_label_area_size(LabelAreaPosition::Left, 40)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_cartesian_2d(min_timestamp..max_timestamp, min_members..max_members)
            .unwrap();

        chart
            .configure_mesh()
            .x_label_formatter(&|x: &DateTime<Utc>| x.date_naive().to_string())
            .draw()
            .unwrap();

        chart
            .draw_series(LineSeries::new(iterator, plotters::prelude::RED))
            .unwrap();
    }

    let mut bytes = Vec::new();
    let img = PngEncoder::new(Cursor::new(&mut bytes));
    img.write_image(&buffer, 1024, 768, ExtendedColorType::Rgb8)
        .unwrap();

    ctx.respond(&bot)
        .files(&[Attachment::from_bytes(
            "analytics.png".to_string(),
            bytes,
            1,
        )])
        .await?;

    Ok(())
}

impl Argument for ViewDuration {
    fn from_interaction(option: &CommandDataOption) -> Result<Self, ArgumentError> {
        match &option.value {
            CommandOptionValue::String(value) => {
                let mut value = value.clone();
                if let Some(dur) = value.pop() {
                    if let Ok(num) = value.parse::<u64>() {
                        match dur {
                            'h' => return Ok(ViewDuration(Duration::from_secs(num * 60 * 60))),
                            'd' => {
                                return Ok(ViewDuration(Duration::from_secs(num * 24 * 60 * 60)))
                            }
                            'm' => {
                                return Ok(ViewDuration(Duration::from_secs(
                                    num * 30 * 24 * 60 * 60,
                                )))
                            }
                            'y' => {
                                return Ok(ViewDuration(Duration::from_secs(
                                    num * 365 * 24 * 60 * 60,
                                )))
                            }
                            _ => {}
                        }
                    }
                }
                Err(ArgumentError::BadArgument)
            }
            _ => unreachable!("ViewDuration unreached"),
        }
    }
}
