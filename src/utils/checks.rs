use serenity::{
    framework::standard::{macros::check, Reason},
    model::channel::Message,
    prelude::Context,
};

#[check]
#[name = "bot_has_manage_messages"]
pub async fn bot_has_manage_messages_check(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let bot_id = ctx.cache.current_user().await.id.0;
    let err = Reason::User(
        "I'm unable to run this command due to missing the `Manage Messages` permission."
            .to_string(),
    );
    if let Some(guild) = msg.channel(ctx).await.unwrap().guild() {
        if !guild
            .permissions_for_user(ctx, bot_id)
            .await
            .expect("what even")
            .manage_messages()
        {
            Err(err)
        } else {
            Ok(())
        }
    } else {
        Err(err)
    }
}

#[check]
#[name = "bot_has_manage_roles"]
pub async fn bot_has_manage_roles_check(ctx: &Context, msg: &Message) -> Result<(), Reason> {
    let bot_id = ctx.cache.current_user().await.id.0;
    if !ctx
        .http
        .get_member(msg.guild_id.unwrap().0, bot_id)
        .await
        .expect("What even")
        .permissions(ctx)
        .await
        .expect("What even 2")
        .manage_roles()
    {
        Err(Reason::User(
            "I'm unable to run this command due to missing the `Manage Roles` permission."
                .to_string(),
        ))
    } else {
        Ok(())
    }
}
