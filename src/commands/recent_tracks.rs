use crate::config::Config;
use anyhow::Result;
use serde_json::Value;

pub fn run(cfg: &Config, limit: u32) -> Result<()> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit={}",
        cfg.username, cfg.api_key, limit
    );

    let res: Value = reqwest::blocking::get(&url)?.json()?;
    let tracks = &res["recenttracks"]["track"];

    if let Some(arr) = tracks.as_array() {
        for track in arr {
            let name = track["name"].as_str().unwrap_or("?");
            let artist = track["artist"]["#text"].as_str().unwrap_or("?");
            let now_playing = track["@attr"]["nowplaying"].as_str() == Some("true");
            if now_playing {
                println!("> {} - {} (now playing)", name, artist);
            } else {
                println!("    {} - {}", name, artist);
            }
        }
    }

    Ok(())
}
