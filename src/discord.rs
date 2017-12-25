use super::errors::*;
use serenity::model::*;

pub fn everyone_role(guild_id: GuildId) -> Result<RoleId> {
    let guild = guild_id.get().chain_err(|| "failed to get guild")?;
    let roles = guild.roles.iter();
    let (everyone_id, _everyone) = roles
        .filter(|&(_role_id, role)| is_everyone(role))
        .next()
        .chain_err(|| "failed to find `@everyone` role")?;

    Ok(*everyone_id)
}

fn is_everyone(role: &Role) -> bool {
    role.position <= 0 && role.name == "@everyone"
}

pub fn hide_channel(channel: &GuildChannel, to_hide_from: PermissionOverwriteType) -> Result<()> {
    channel
        .create_permission(&PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::READ_MESSAGES,
            kind: to_hide_from,
        })
        .chain_err(|| "failed to hide channel")?;

    Ok(())
}

pub fn from_role_id(role_id: RoleId) -> PermissionOverwriteType {
    PermissionOverwriteType::Role(role_id)
}

pub fn find_channel(name: &str, guild_id: GuildId) -> Result<ChannelId> {
    let channels = guild_id
        .channels()
        .chain_err(|| "failed to retrieve channels")?;
    let (channel_id, _channel) = channels
        .iter()
        .filter(|&(_channel_id, channel)| channel.name == name)
        .next()
        .chain_err(|| "failed to find channel")?;
    Ok(*channel_id)
}

pub fn create_secret_channel(name: &str, guild_id: GuildId) -> Result<GuildChannel> {
    let new_channel = guild_id
        .create_channel(name, ChannelType::Text)
        .chain_err(|| "failed to create new channel")?;

    // block the channel for everyone who hasn't clicked the checkmark. see process_checkmark().
    hide_channel(
        &new_channel,
        from_role_id(everyone_role(guild_id).chain_err(|| "failed to find @everyone RoleId")?),
    ).chain_err(|| "failed to configure new channel")?;

    Ok(new_channel)
}

pub fn guild_channel(c: Channel) -> Option<::std::sync::Arc<::std::sync::RwLock<GuildChannel>>> {
    match c {
        Channel::Guild(channel_lock) => Some(channel_lock),
        _ => None,
    }
}