use crate::config::Config;
use anyhow::Result;
use serde_json::Value;

pub struct DashboardStats {
    pub username: String,
    pub weekly_scrobbles: u32,
    pub top_track: String,
    pub top_artist: String,
    pub top_album: String,
    pub now_playing: String,
    pub total_scrobbles: String,
    pub cover_image_url: Option<String>,
}

pub fn fetch(cfg: &Config) -> Result<Option<DashboardStats>> {
    let user_info_url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getinfo&user={}&api_key={}&format=json",
        cfg.username, cfg.api_key
    );

    let top_tracks_url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.gettoptracks&user={}&api_key={}&format=json&period=7day&limit=1",
        cfg.username, cfg.api_key
    );

    let top_arists_url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.gettopartists&user={}&api_key={}&format=json&period=7day&limit=1",
        cfg.username, cfg.api_key
    );

    let top_albums_url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.gettopalbums&user={}&api_key={}&format=json&period=7day&limit=1",
        cfg.username, cfg.api_key
    );

    let recent_url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getrecenttracks&user={}&api_key={}&format=json&limit=1",
        cfg.username, cfg.api_key
    );

    let weekly_url = format!(
        "https://ws.audioscrobbler.com/2.0/?method=user.getweeklytrackchart&user={}&api_key={}&format=json",
        cfg.username, cfg.api_key
    );

    let user_info: Value = reqwest::blocking::get(&user_info_url)?.json()?;
    let top_tracks: Value = reqwest::blocking::get(&top_tracks_url)?.json()?;
    let top_artists: Value = reqwest::blocking::get(&top_artists_url)?.json()?;
    let top_albums: Value = reqwest::blocking::get(&top_albums_url)?.json()?;
    let recent: Value = reqwest::blocking::get(&recent_url)?.json()?;
    let weekly: Value = reqwest::blocking::get(&weekly_url)?.json()?;

    let username = user_info["user"]["name"]
        .as_str()
        .unwrap_or(&cfg.username)
        .to_string();

    let total_scrobbles = user_info["user"]["playcount"]
        .as_str()
        .unwrap_or("?")
        .to_string();

    let top_track = top_tracks["toptracks"]["track"]
        .as_array()
        .and_then(|arr| arr.first())
        .map(|track| {
            let name = track["name"].as_str().unwrap_or("?");
            let artist = track["artist"]["name"].as_str().unwrap_or("?");
            format!("{name} - {artist}")
        })
        .unwrap_or_else(|| String::from("No user stats"));

    let top_artist = top_artists["topartists"]["artist"]
        .as_array()
        .and_then(|arr| arr.first())
        .map(|artist| artist["name"].as_str().unwrap_or("?").to_string())
        .unwrap_or_else(|| String::from("No user stats"));

    let top_album_value = top_albums["topalbums"]["album"]
        .as_array()
        .and_then(|arr| arr.first());

    let top_album = top_album_value
        .map(|album| {
            let name = album["name"].as_str().unwrap_or("?");
            let artist = album["artist"]["name"].as_str().unwrap_or("?");
            format!("{name} - {artist}")
        })
        .unwrap_or_else(|| String::from("No user stats"));

    let cover_image_url = top_album_value
        .and_then(|album| album["image"].as_array())
        .and_then(|images| {
            images
                .iter()
                .rev()
                .find_map(|img| img["#text"].as_str())
                .filter(|url| !url.is_empty())
        })
        .map(String::from);

    let now_playing = recent["recenttracks"]["track"]
        .as_array()
        .and_then(|arr| arr.first())
        .map(|track| {
            let name = track["name"].as_str().unwrap_or("?");
            let artist = track["artist"]["#text"].as_str().unwrap_or("?");
            let is_now_playing = track["@attr"]["nowplaying"].as_str() == Some("true");

            if is_now_playing {
                format!("{name} - {artist}")
            } else {
                String::from("Nothing playing")
            }
        })
        .unwrap_or_else(|| String::from("Nothing playing"));

    let weekly_scrobbles: u32 = weekly["weeklytrackchart"]["track"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|track| track["playcount"].as_str())
                .filter_map(|count| count.parse::<u32>().ok())
                .sum()
        })
        .unwrap_or(0);

    if weekly_scrobbles == 0 && top_track == "No user stats" && top_artist == "No user stats" && top_album == "No user stats" {
        return Ok(None);
    }

    Ok(Some(DashboardStats {
        username,
        weekly_scrobbles,
        top_track,
        top_artist,
        top_album,
        now_playing,
        total_scrobbles,
        cover_image_url,
    }))
}
