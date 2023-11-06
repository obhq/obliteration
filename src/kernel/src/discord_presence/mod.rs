use crate::log::print;
use crate::{info, warn};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use param::Param;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

pub fn rich_presence(game_path: &Path) -> Result<DiscordIpcClient, Box<dyn std::error::Error>> {
    // Fetch Title and TitleID from Param.sfo if possible
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

    // Print out Param information for easier reporting.
    let mut initstatus = info!();

    writeln!(initstatus, "Initializing Discord rich presence.").unwrap();
    writeln!(initstatus, "Application Title  : {}", title).unwrap();
    writeln!(initstatus, "Application TitleID: {}", title_id).unwrap();

    print(initstatus);

    // Initialize new Discord IPC with our ID, should never fail.
    let mut client = match DiscordIpcClient::new("1168617561244565584") {
        Ok(client) => client,
        Err(e) => {
            warn!(e, "Failed to create Discord IPC");
            return Err(e.into());
        }
    };

    // Attempt to have IPC connect to user's Discord, will fail if user doesn't have Discord running.
    if let Err(e) = client.connect() {
        warn!(e, "Failed to connect to Discord client");
        return Err(e.into());
    }

    // Get Timestamp of Kernel starting for Discord's "elapsed" counter.
    let start_time: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        .try_into()
        .unwrap();

    // Create details about game and send activity to Discord.
    let details_text = &format!("Playing {} - {}", title, title_id);
    let payload = activity::Activity::new()
        .details(details_text)
        .assets(
            activity::Assets::new()
                .large_image("obliteration-icon")
                .large_text("Obliteration"),
        )
        .timestamps(activity::Timestamps::new().start(start_time));

    // If failing here, user's Discord most likely crashed or is offline.
    if let Err(e) = client.set_activity(payload) {
        warn!(e, "Failed to update Discord presence");
        return Err(e.into());
    }
    Ok(client)
}
