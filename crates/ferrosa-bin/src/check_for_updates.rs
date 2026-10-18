use reqwest::Client;
use serde_json::Value;

pub fn print_update_status() {
    tokio::spawn(async {
        let current_hash = env!("GIT_HASH");
        let client = Client::builder().user_agent("FerrosaServer").build().unwrap();

        let res = client
            .get("https://api.github.com/repos/ferrosa-game/dedicated-server/releases/latest")
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();
        let latest_tag = res.get("tag_name").unwrap().to_string();

        let compare = client
            .get(format!("https://api.github.com/repos/ferrosa-game/dedicated-server/compare/{current_hash}...{latest_tag}"))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();

        let status = compare.get("status").unwrap().to_string();

        if status == "ahead" {
            println!("!!!!!!!!!!!!!! UPDATE AVAILABLE !!!!!!!!!!!!!!");
            println!("There is an update available to Ferrosa {}!", env!("CARGO_PKG_VERSION"));
            println!("Download it at https://github.com/ferrosa-game/dedicated-server/releases/latest")
        }
    });
}
