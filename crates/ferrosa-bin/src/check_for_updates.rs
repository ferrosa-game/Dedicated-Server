use std::error::Error;

use reqwest::Client;
use serde_json::Value;

pub fn print_update_status() {
    tokio::spawn(async {
        if let Err(e) = check_for_updates().await {
            eprintln!("Failed to check for updates: {}", e);
        }
    });
}

async fn check_for_updates() -> Result<(), Box<dyn Error>> {
    let current_hash = env!("GIT_HASH");
    let client = Client::builder().user_agent("FerrosaServer").build()?;

    let res = client.get("https://api.github.com/repos/ferrosa-game/dedicated-server/releases/latest").send().await?.json::<Value>().await?;
    let latest_tag = res.get("tag_name").and_then(|v| v.as_str()).ok_or("Missing tag_name in GitHub response!")?;

    let compare = client
        .get(format!("https://api.github.com/repos/ferrosa-game/dedicated-server/compare/{current_hash}...{latest_tag}"))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let status = compare.get("status").and_then(|v| v.as_str()).ok_or("Missing status in GitHub response!")?;

    if status == "ahead" {
        println!("!!!!!!!!!!!!!! UPDATE AVAILABLE !!!!!!!!!!!!!!");
        println!("There is an update available to Ferrosa {}!", env!("CARGO_PKG_VERSION"));
        println!("Download it at https://github.com/ferrosa-game/dedicated-server/releases/latest")
    }

    Ok(())
}
