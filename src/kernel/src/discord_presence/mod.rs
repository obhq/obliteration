use crate::{info, warn};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use param::Param;
use std::fs::File;
use std::path::Path;
use std::time::SystemTime;

pub fn rich_presence(game_path: &Path) -> Result<DiscordIpcClient, Box<dyn std::error::Error>> {
    info!("Initializing Discord rich presence.");
    let mut client = match DiscordIpcClient::new("1168617561244565584") {
        Ok(client) => client,
        Err(e) => {
            warn!(e, "Failed to create Discord IPC");
            return Err(e.into());
        }
    };

    if let Err(e) = client.connect() {
        warn!(e, "Failed to connect to Discord client");
        return Err(e.into());
    }

    let start_time: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()
        .unwrap();
    let mut param_path = game_path.join("sce_sys");
    param_path.push("param.sfo");
    let mut title = String::from("Unknown Title");
    let mut title_id = String::from("Unknown Title ID");

    match File::open(&param_path) {
        Ok(param_file) => match Param::read(param_file) {
            Ok(param) => {
                title = param.title().to_string();
                title_id = param.title_id().to_string();
            }
            Err(e) => warn!(e, "Failed to read param.sfo, using placeholders"),
        },
        Err(e) => warn!(e, "Failed to open param.sfo, using placeholders"),
    }

    let details_text = &format!("Playing {} - {}", title, title_id);
    let payload = activity::Activity::new()
        .details(details_text)
        .assets(
            activity::Assets::new()
                .large_image("obliteration-icon")
                .large_text("Obliteration"),
        )
        .timestamps(activity::Timestamps::new().start(start_time));
    if let Err(e) = client.set_activity(payload) {
        warn!(e, "Failed to update Discord presence");
        return Err(e.into());
    }
    Ok(client)
}
