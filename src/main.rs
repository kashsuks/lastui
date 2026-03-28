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
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};
use std::{
    io,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Screen {
    Dashboard,
    RecentTracks,
    Search,
}

struct DashboardData {
    art: Vec<Line<'static>>,
    stats: Vec<String>,
}

enum DashboardState {
    Loading,
    Loaded(DashboardData),
    Empty,
    Error(String),
}

struct App {
    screen: Screen,
    search_input: String,
    recent_tracks: Vec<String>,
    search_results: Vec<String>,
    dashboard_state: DashboardState,
    loading_frame: usize,
    last_tick: Instant,
    status: String,
    should_quit: bool,
}

enum DashboardMessage {
    Loaded(DashboardData),
    Empty,
    Error(String),
}

impl App {
    fn new() -> Self {
        Self {
            screen: Screen::Dashboard,
            search_input: String::new(),
            recent_tracks: Vec::new(),
            search_results: Vec::new(),
            dashboard_state: DashboardState::Loading,
            loading_frame: 0,
            last_tick: Instant::now(),
            status: String::from("q: quit, Tab: switch view"),
            should_quit: false,
        }
    }

    fn tick(&mut self) {
        if self.last_tick.elapsed() >= Duration::from_millis(300) {
            self.loading_frame = (self.loading_frame + 1) % 3;
            self.last_tick = Instant::now();
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
                    Screen::Dashboard => Screen::RecentTracks,
                    Screen::RecentTracks => Screen::Search,
                    Screen::Search => Screen::Dashboard,
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

fn loading_label(frame: usize) -> String {
    let dots = match frame % 3 {
        0 => ".",
        1 => "..",
        _ => "...",
    };

    format!("Loading{dots}")
}

fn dashboard_stats_line(data: &DashboardData) -> Vec<String> {
    data.stats.clone()
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

    let tabs = Tabs::new(vec!["Home", "Recent", "Search"])
        .select(match app.screen {
            Screen::Dashboard => 0,
            Screen::RecentTracks => 1,
            Screen::Search => 2,
        })
        .block(Block::default().borders(Borders::ALL).title("lastui"));

    frame.render_widget(tabs, areas[0]);

    match app.screen {
        Screen::Dashboard => {
            match &app.dashboard_state {
                DashboardState::Loading => {
                    let panel = Paragraph::new(loading_label(app.loading_frame))
                        .block(Block::default().borders(Borders::ALL).title("Home"));

                    frame.render_widget(panel, areas[1]);
                }
                DashboardState::Empty => {
                    let panel = Paragraph::new("No user stats")
                        .block(Block::default().borders(Borders::ALL).title("Home"));

                    frame.render_widget(panel, areas[1]);
                }
                DashboardState::Error(message) => {
                    let panel = Paragraph::new(format!("Error: {message}"))
                        .block(Block::default().borders(Borders::ALL).title("Home"));

                    frame.render_widget(panel, areas[1]);
                }
                DashboardState::Loaded(data) => {
                    let columns = Layout::horizontal([
                        Constraint::Length(34),
                        Constraint::Min(1),
                    ])
                    .split(areas[1]);

                    let art = Paragraph::new(data.art.clone())
                        .block(Block::default().borders(Borders::ALL).title("Art"));

                    let stats_items: Vec<ListItem> = dashboard_stats_line(data)
                        .into_iter()
                        .map(ListItem::new)
                        .collect();

                    let stats = List::new(stats_items)
                        .block(Block::default().borders(Borders::ALL).title("Stats"));

                    frame.render_widget(art, columns[0]);
                    frame.render_widget(stats, columns[1]);
                }
            }
        }
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

        let (tx, rx) = mpsc::channel::<DashboardMessage>();
        let dashboard_cfg = config::Config {
            api_key: cfg.api_key.clone(),
            username: cfg.username.clone(),
        };

        std::thread::spawn(move || {
            let message = match commands::dashboard::fetch(&dashboard_cfg) {
                Ok(Some(stats)) => {
                    let art = stats
                        .cover_image_url
                        .as_deref()
                        .and_then(|url| commands::dashboard::cover_to_ascii(url, 30).ok())
                        .unwrap_or_else(|| vec![Line::from("No user stats")]);

                    let stats_lines = vec![
                        format!("user: {}", stats.username),
                        format!("scrobbles this week: {}", stats.weekly_scrobbles),
                        format!("top track: {}", stats.top_track),
                        format!("top artist: {}", stats.top_artist),
                        format!("top album: {}", stats.top_album),
                        format!("now playing: {}", stats.now_playing),
                        format!("total scrobbles: {}", stats.total_scrobbles),
                    ];

                    DashboardMessage::Loaded(DashboardData {
                        art,
                        stats: stats_lines,
                    })
                }
                Ok(None) => DashboardMessage::Empty,
                Err(err) => DashboardMessage::Error(err.to_string()),
            };

            let _ = tx.send(message);
        });

        while !app.should_quit {
            app.tick();

            if let Ok(message) = rx.try_recv() {
                app.dashboard_state = match message {
                    DashboardMessage::Loaded(data) => DashboardState::Loaded(data),
                    DashboardMessage::Empty => DashboardState::Empty,
                    DashboardMessage::Error(message) => DashboardState::Error(message),
                };
            }

            terminal.draw(|frame| ui(frame, &app))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        app.on_key(key.code, &cfg)?;
                    }
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
