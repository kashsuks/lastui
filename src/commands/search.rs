use crate::config::Config;
use anyhow::Result;
use serde_json::Value;

pub fn fetch(cfg: &Config, query: &str) -> Result<Vec<String>> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=track.search&track={}&api_key={}&format=json&limit=10",
        urlencoding::encode(query), cfg.api_key
    );

    let res: Value = reqwest::blocking::get(&url)?.json()?;
    let tracks = &res["results"]["trackmatches"]["track"];

    let mut items = Vec::new();

    if let Some(arr) = tracks.as_array() {
        for track in arr {
            let name = track["name"].as_str().unwrap_or("?");
            let artist = track["artist"].as_str().unwrap_or("?");
            let listeners = track["listeners"].as_str().unwrap_or("?");

            items.push(format!("{} - {} ({} listeners)", name, artist, listeners));
        }
    }

    Ok(items)
}
