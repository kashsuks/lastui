use crate::config::Config;
use anyhow::Result;
use image::{DynamicImage, imageops::FilterType};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde_json::Value;

const ASCII_CHARS: &[char] = &['@', '%', '#', '*', '+', '=', '-', ':', '.'];

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

    let top_artists_url = format!(
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

pub fn cover_to_ascii(url: &str, width: u32) -> Result<Vec<Line<'static>>> {
    let bytes = reqwest::blocking::get(url)?.bytes()?;
    let image = image::load_from_memory(&bytes)?;
    Ok(image_to_ascii(image, width))
}

fn image_to_ascii(image: DynamicImage, width: u32) -> Vec<Line<'static>> {
    let rgb = image.to_rgb8();

    let (src_w, src_h) = rgb.dimensions();
    if src_w == 0 || src_h == 0 {
        return vec![Line::from("No user stats")];
    }

    let aspect = src_h as f32 / src_w as f32;
    let height = ((width as f32 * aspect) * 0.55).max(1.0) as u32;
    let resized = image.resize_exact(width, height, FilterType::Triangle).to_rgb8();

    let mut lines = Vec::new();

    for y in 0..resized.height() {
        let mut spans = Vec::new();

        for x in 0..resized.width() {
            let pixel = resized.get_pixel(x, y);
            let [r, g, b] = pixel.0;
            let [fg_r, fg_g, fg_b] = boost_color([r, g, b]);
            let [bg_r, bg_g, bg_b] = darken_color([r, g, b]);

            let brightness =
                (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0;
            let normalized = brightness.powf(0.82);
            let index = ((ASCII_CHARS.len() - 1) as f32 * normalized).round() as usize;
            let ch = ASCII_CHARS[index];

            spans.push(Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(Color::Rgb(fg_r, fg_g, fg_b))
                    .bg(Color::Rgb(bg_r, bg_g, bg_b)),
            ));
        }

        lines.push(Line::from(spans));
    }

    lines
}

fn boost_color([r, g, b]: [u8; 3]) -> [u8; 3] {
    let [r, g, b] = saturate([r, g, b], 1.22);
    let [r, g, b] = brighten([r, g, b], 1.08);
    [r, g, b]
}

fn darken_color([r, g, b]: [u8; 3]) -> [u8; 3] {
    [
        scale_channel(r, 0.45),
        scale_channel(g, 0.45),
        scale_channel(b, 0.45),
    ]
}

fn saturate([r, g, b]: [u8; 3], amount: f32) -> [u8; 3] {
    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;

    [
        clamp_channel(luma + (r as f32 - luma) * amount),
        clamp_channel(luma + (g as f32 - luma) * amount),
        clamp_channel(luma + (b as f32 - luma) * amount),
    ]
}

fn brighten([r, g, b]: [u8; 3], amount: f32) -> [u8; 3] {
    [
        scale_channel(r, amount),
        scale_channel(g, amount),
        scale_channel(b, amount),
    ]
}

fn scale_channel(value: u8, amount: f32) -> u8 {
    clamp_channel(value as f32 * amount)
}

fn clamp_channel(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}
