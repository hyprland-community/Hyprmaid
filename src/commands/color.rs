use anyhow::Result;
use poise::{serenity_prelude::RoleId,CreateReply};
use strum::VariantArray;
use crate::{Error,Context};

#[derive(poise::ChoiceParameter,Clone,Copy,Debug,strum::VariantArray)]
enum Colors {
    Grey=1234590815431430204,
    Orange=1234591006217994271,
    Piss=1234603957146484757,
    Red=1234591217069854731,
    Lavender=1234591535622914099,
    Purple=1234595002806567135,
    DarkTeal=1234591688991703073,
    Teal=1234595515707162707,
    Green=1234591811033497660,
    Blue=1234622330144161893,
}

#[poise::command(slash_command, prefix_command)]
pub async fn color(
    ctx: Context<'_>,
    #[description = "Choose Color"] color: Colors,
) -> Result<(), Error> {
    let author = ctx.author_member().await.unwrap();

    let color_role_id = RoleId::new(color as u64);

    for role in Colors::VARIANTS {
        let role_id = RoleId::new(*role as u64);
        if author.roles.contains(&role_id) {
            if role_id == color_role_id {
                ctx.send(
                    CreateReply::default()
                        .content("You already have this color dum dum")
                        .ephemeral(true)
                ).await?;
                return Ok(());
            }
            author.remove_role(ctx.http(), role_id).await?;
        }
    }

    author.add_role(ctx.http(), color_role_id).await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Woa you are now {:?}",color))
            .ephemeral(true)
    ).await?;

    Ok(())
}
