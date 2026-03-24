use crate::config::Config;
use anyhow::Result;
use serde_json::Value;

pub fn fetch(cfg: &Config, limit: u32) -> Result<Vec<String>> {
    let url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit={}",
        cfg.username, cfg.api_key, limit
    );

    let res: Value = reqwest::blocking::get(&url)?.json()?;
    let tracks = &res["recenttracks"]["track"];

   let mut items = Vec::new();

   if let Some(arr) = tracks.as_array() {
       for track in arr {
           let name = track["name"].as_str().unwrap_or("?");
           let artist = track["artist"]["#text"].as_str().unwrap_or("?");
           let now_playing = track["@attr"]["nowplaying"].as_str() == Some("true");

           let line = if now_playing {
               format!("> {} - {} (now playing)", name, artist)
           } else {
               format!("    {} - {}", name, artist)
           };

           items.push(line);
       }
   }

   Ok(items)
}
