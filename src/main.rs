mod config;

use std::io::{self, Write};

fn main() -> anyhow::Result<()> {
    let cfg = match config::load() {
        Some(c) => c,
        None => {
            print!("Enter your last.fm API key: ");
            io::stdout().flush()?;

            let mut key = String::new();
            io::stdin().read_line(&mut key)?;
            let key = key.trim();

            config::save(key)?;
            println!("API key saved.");
            config::load().unwrap()
        }
    };

    println!("Loaded API key: {}", &cfg.api_key[..8]); // only show the preview
    //TODO: add commands soon
    Ok(())
}
