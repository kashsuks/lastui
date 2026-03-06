mod config;
mod commands;

use clap::{Parser, Subcommand};
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "lastui", about = "Last.fm CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    RecentTracks {
        #[arg(short, long, default_value_t = 10)]
        limit: u32,
    },

    Search {
        query: String,
    },
}

fn prompt(label: &str) -> String {
    print!("{}", label);
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

fn main() -> anyhow::Result<()> {
    let cfg = config::load().unwrap_or_else(|| {        let api_key = prompt("Enter your last.fm API key: ");
        let username = prompt("Enter your Last.fm username: ");
        config::save(&api_key, &username).expect("Failed to save config");
        println!("Config saved");
        config::load().unwrap()
    });

   let cli = Cli::parse();

   match cli.command {
       Command::RecentTracks { limit } => commands::recent_tracks::run(&cfg, limit)?,
       Command::Search { query } => commands::search::run(&cfg, &query)?,
   }

   Ok(())
}
