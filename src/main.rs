mod config;
mod commands;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{
        disable_raw_mode, 
        enable_raw_mode, 
        EnterAlternateScreen, 
        LeaveAlternateScreen
    },
};
use ratatui::{
    prelude::*,
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs},
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
    Settings,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsField {
    Username,
    ApiKey,
}

struct App {
    screen: Screen,
    config: config::Config,
    search_input: String,
    recent_tracks: Vec<String>,
    search_results: Vec<String>,
    dashboard_state: DashboardState,

    command_palette_open: bool,
    command_input: String,
    command_selected: usize,

    settings_username: String,
    settings_api_key: String,
    settings_field: SettingsField,

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
    fn new(cfg: config::Config) -> Self {
        Self {
            screen: Screen::Dashboard,
            settings_username: cfg.username.clone(),
            settings_api_key: cfg.api_key.clone(),
            config: cfg,
            search_input: String::new(),
            recent_tracks: Vec::new(),
            search_results: Vec::new(),
            dashboard_state: DashboardState::Loading,

            command_palette_open: false,
            command_input: String::new(),
            command_selected: 0,
            settings_field: SettingsField::Username,
            loading_frame: 0,
            last_tick: Instant::now(),
            status: String::from("q: quit, Tab: switch view, s: commands"),
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

    fn save_settings(&mut self) -> Result<()> {
        config::save(&self.settings_api_key, &self.settings_username)?;
        self.config.username = self.settings_username.clone();
        self.config.api_key = self.settings_api_key.clone();
        self.status = String::from("Settings saved");
        Ok(())
    }

    fn on_key(&mut self, key: KeyCode) -> Result<()> {
        if self.command_palette_open {
            match key {
                KeyCode::Esc => {
                    self.command_palette_open = false;
                    self.command_input.clear();
                    self.command_selected = 0;
                }
                KeyCode::Backspace => {
                    self.command_input.pop();
                    self.command_selected = 0;
                }
                KeyCode::Up => {
                    self.command_selected = self.command_selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    let count = filtered_commands(&self.command_input).len();
                    if count > 0 {
                        self.command_selected = (self.command_selected + 1).min(count - 1);
                    }
                }
                KeyCode::Enter => {
                    let matches = filtered_commands(&self.command_input);
                    if let Some(selected) = matches.get(self.command_selected) {
                        match *selected {
                            "now playing" => {
                                self.screen = Screen::RecentTracks;
                                self.status = String::from("Opened recent tracks");
                            }
                            "search song" => {
                                self.screen = Screen::Search;
                                self.search_input.clear();
                                self.search_results.clear();
                                self.status = String::from("Search for a song");
                            }
                            "settings" => {
                                self.screen = Screen::Settings;
                                self.settings_username = self.config.username.clone();
                                self.settings_api_key = self.config.api_key.clone();
                                self.settings_field = SettingsField::Username;
                                self.status = String::from("Editing settings");
                            }
                            _ => {}
                        }
                    }

                    self.command_palette_open = false;
                    self.command_input.clear();
                    self.command_selected = 0;
                }
                KeyCode::Char(c) => {
                    self.command_input.push(c);
                    self.command_selected = 0;
                }
                _ => {}
            }

            return Ok(());
        }

        if self.screen == Screen::Settings {
            match key {
                KeyCode::Esc => {
                    self.screen = Screen::Dashboard;
                    self.status = String::from("Closed settings");
                }
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                    self.settings_field = match self.settings_field {
                        SettingsField::Username => SettingsField::ApiKey,
                        SettingsField::ApiKey => SettingsField::Username,
                    };
                }
                KeyCode::Enter => {
                    self.save_settings()?;
                    self.screen = Screen::Dashboard;
                }
                KeyCode::Backspace => match self.settings_field {
                    SettingsField::Username => {
                        self.settings_username.pop();
                    }
                    SettingsField::ApiKey => {
                        self.settings_api_key.pop();
                    }
                },
                KeyCode::Char(c) => match self.settings_field {
                    SettingsField::Username => self.settings_username.push(c),
                    SettingsField::ApiKey => self.settings_api_key.push(c),
                },
                _ => {}
            }

            return Ok(());
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('s') => {
                self.command_palette_open = true;
                self.command_input.clear();
                self.command_selected = 0;
            }
            KeyCode::Tab => {
                self.screen = match self.screen {
                    Screen::Dashboard => Screen::RecentTracks,
                    Screen::RecentTracks => Screen::Search,
                    Screen::Search => Screen::Dashboard,
                    Screen::Settings => Screen::Dashboard,
                };
            }
            KeyCode::Enter if self.screen == Screen::Search => {
                self.search_results = commands::search::fetch(&self.config, &self.search_input)?;
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

fn command_items() -> Vec<&'static str> {
    vec!["now playing", "search song", "settings"]
}

fn fuzzy_matches(query: &str, candidate: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let mut query_chars = query.chars().map(|c| c.to_ascii_lowercase());
    let mut current = query_chars.next();

    for ch in candidate.chars().map(|c| c.to_ascii_lowercase()) {
        if Some(ch) == current {
            current = query_chars.next();
            if current.is_none() {
                return true;
            }
        }
    }

    false
}

fn filtered_commands(query: &str) -> Vec<&'static str> {
    command_items()
        .into_iter()
        .filter(|item| fuzzy_matches(query, item))
        .collect()
}

fn masked_api_key(value: &str) -> String {
    let char_count = value.chars().count();
    if char_count <= 4 {
        return value.to_string();
    }

    let visible: String = value
        .chars()
        .skip(char_count.saturating_sub(4))
        .collect();

    format!("{}{}", "*".repeat(char_count - 4), visible)
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let horizontal = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .split(area);

    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .split(horizontal[1]);

    vertical[1]
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
            Screen::Settings => 0,
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
                    let home = Block::default().borders(Borders::ALL).title("Home");
                    let inner = home.inner(areas[1]);
                    let columns = Layout::horizontal([
                        Constraint::Length(34),
                        Constraint::Min(1),
                    ])
                    .split(inner);

                    let art = Paragraph::new(data.art.clone());

                    let stats_items: Vec<ListItem> = dashboard_stats_line(data)
                        .into_iter()
                        .map(ListItem::new)
                        .collect();

                    let stats = List::new(stats_items);

                    frame.render_widget(home, areas[1]);
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
        Screen::Settings => {
            let rows = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(areas[1]);

            let username_title = if app.settings_field == SettingsField::Username {
                "Username *"
            } else {
                "Username"
            };

            let api_title = if app.settings_field == SettingsField::ApiKey {
                "API Key *"
            } else {
                "API Key"
            };

            let username = Paragraph::new(app.settings_username.as_str())
                .block(Block::default().borders(Borders::ALL).title(username_title));

            let api_key = Paragraph::new(masked_api_key(&app.settings_api_key))
                .block(Block::default().borders(Borders::ALL).title(api_title));

            let help = Paragraph::new("Tab: switch field, Enter: save, Esc: cancel")
                .block(Block::default().borders(Borders::ALL).title("Help"));

            frame.render_widget(username, rows[0]);
            frame.render_widget(api_key, rows[1]);
            frame.render_widget(help, rows[2]);
        }
    }

    if app.command_palette_open {
        let popup = centered_rect(frame.area(), 50, 10);
        let inner = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .margin(1)
        .split(popup);

        let matches = filtered_commands(&app.command_input);

        let input = Paragraph::new(app.command_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Command"));

        let items: Vec<ListItem> = matches
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let prefix = if idx == app.command_selected { "> " } else { " " };
                ListItem::new(format!("{prefix}{item}"))
            })
        .collect();

        let results = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Matches"));

        frame.render_widget(Clear, popup);
        frame.render_widget(
            Block::default().borders(Borders::ALL).title("Commands"), 
            popup,
        );
        frame.render_widget(input, inner[0]);
        frame.render_widget(results, inner[1]);
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
        let mut app = App::new(cfg.clone());
        let app_cfg = app.config.clone();
        app.load_initial_dat(&app_cfg)?;

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
                        app.on_key(key.code)?;
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
