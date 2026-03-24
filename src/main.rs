mod config;
mod commands;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};
use std::io;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    RecentTracks,
    Search,
}

struct App {
    screen: Screen,
    search_input: String,
    recent_tracks: Vec<String>,
    search_results: Vec<String>,
    status: String,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            screen: Screen::RecentTracks,
            search_input: String::new(),
            recent_tracks: Vec::new(),
            search_results: Vec::new(),
            status: String::from("q: quit, Tab: switch view, /: search"),
            should_quit: false,
        }
    }

    fn load_initial_dat(&mut self, cfg: &config::Config) -> Result<()> {
        self.recent_tracks = commands::recent_tracks::fetch(cfg, 10)?;
        Ok(())
    }

    fn on_key(&mut self, key: KeyCode, cfg: &config::Config) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Tab => {
                self.screen = match self.screen {
                    Screen::RecentTracks => Screen::Search,
                    Screen::Search => Screen::RecentTracks,
                };
            }
            KeyCode::Enter if self.screen == Screen::Search => {
                self.search_results = commands::search::fetch(cfg, &self.search_input)?;
                self.status = format!("{} results for '{}'", self.search_results.len(), self.search_input);
            }
            KeyCode::Backspace if self.screen == Screen::Search => {
                self.search_input.pop();
            }
            KeyCode::Char(c) if self.screen == Screen::Search => {
                self.search_input.push(c);
            }
            _ => {}
        }
        Ok(())
    }
}

fn prompt(label: &str) -> String {
    use std::io::Write;

    print!("{}", label);
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

fn ui(frame: &mut Frame, app: &App) {
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(frame.area());

    let tabs = Tabs::new(vec!["Recent", "Search"])
        .select(match app.screen {
            Screen::RecentTracks => 0,
            Screen::Search => 1,
        })
        .block(Block::default().borders(Borders::ALL).title("lastui"));

    frame.render_widget(tabs, areas[0]);

    match app.screen {
        Screen::RecentTracks => {
            let items: Vec<ListItem> = app
                .recent_tracks
                .iter()
                .map(|t| ListItem::new(t.as_str()))
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Recent Tracks"));

            frame.render_widget(list, areas[1]);
        }
        Screen::Search => {
            let inner = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(areas[1]);

            let input = Paragraph::new(app.search_input.as_str())
                .block(Block::default().borders(Borders::ALL).title("Search"));

            let results: Vec<ListItem> = app
                .search_results
                .iter()
                .map(|t| ListItem::new(t.as_str()))
                .collect();

            let list = List::new(results)
                .block(Block::default().borders(Borders::ALL).title("Results"));

            frame.render_widget(input, inner[0]);
            frame.render_widget(list, inner[1]);
        }
    }

    let status = Paragraph::new(app.status.as_str())
        .block(Block::default().borders(Borders::ALL).title("Status"));

    frame.render_widget(status, areas[2]);
}

fn main() -> Result<()> {
    let cfg = config::load().unwrap_or_else(|| {
        let api_key = prompt("Enter your last.fm API key: ");
        let username = prompt("Enter your Last.fm username: ");
        config::save(&api_key, &username).expect("Failed to save config");
        config::load().unwrap()
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> Result<()> {
        let mut app = App::new();
        app.load_initial_dat(&cfg)?;

        while !app.should_quit {
            terminal.draw(|frame| ui(frame, &app))?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key.code, &cfg)?;
                }
            }
        }

        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
