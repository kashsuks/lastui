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
enum TabKind {
    Home,
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
enum ThemeChoice {
    CatppuccinMocha,
    TokyoNight,
    TopAlbum,
}

impl ThemeChoice {
    fn from_config(value: &str) -> Self {
        match value {
            "tokyonight" => Self::TokyoNight,
            "top-album" => Self::TopAlbum,
            _ => Self::CatppuccinMocha,
        }
    }

    fn as_config_value(self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "catppuccin-mocha",
            Self::TokyoNight => "tokyonight",
            Self::TopAlbum => "top-album",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::TokyoNight => "TokyoNight",
            Self::TopAlbum => "Top Album",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::CatppuccinMocha => Self::TokyoNight,
            Self::TokyoNight => Self::TopAlbum,
            Self::TopAlbum => Self::CatppuccinMocha,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::CatppuccinMocha => Self::TopAlbum,
            Self::TokyoNight => Self::CatppuccinMocha,
            Self::TopAlbum => Self::TokyoNight,
        }
    }
}

struct UiTheme {
    background: Color,
    panel_bg: Color,
    border: Color,
    text: Color,
    muted: Color,
    accent: Color,
    selected_bg: Color,
    selected_fg: Color,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsField {
    Username,
    ApiKey,
    Theme,
}

struct App {
    active_tab: TabKind,
    open_tabs: Vec<TabKind>,
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
    settings_theme: ThemeChoice,
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
            active_tab: TabKind::Home,
            open_tabs: vec![TabKind::Home],
            settings_username: cfg.username.clone(),
            settings_api_key: cfg.api_key.clone(),
            settings_theme: ThemeChoice::from_config(&cfg.theme),
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
            status: String::from("q: quit, Tab: next tab, s: commands, Esc: close tab"),
            should_quit: false,
        }
    }

    fn open_tab(&mut self, tab: TabKind) {
        if !self.open_tabs.contains(&tab) {
            self.open_tabs.push(tab);
        }
        self.active_tab = tab;
    }

    fn cycle_tab(&mut self) {
        if self.open_tabs.is_empty() {
            return;
        }

        let current = self
            .open_tabs
            .iter()
            .position(|tab| *tab == self.active_tab)
            .unwrap_or(0);

        let next = (current + 1) % self.open_tabs.len();
        self.active_tab = self.open_tabs[next];
    }

    fn close_active_tab(&mut self) {
        if self.active_tab == TabKind::Home {
            return;
        }

        if let Some(index) = self.open_tabs.iter().position(|tab| *tab == self.active_tab) {
            self.open_tabs.remove(index);
        }

        self.active_tab = *self.open_tabs.last().unwrap_or(&TabKind::Home);
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
        let theme = self.settings_theme.as_config_value();
        config::save(&self.settings_api_key, &self.settings_username, theme)?;
        self.config.username = self.settings_username.clone();
        self.config.api_key = self.settings_api_key.clone();
        self.config.theme = theme.to_string();
        self.status = format!("Settings saved ({})", self.settings_theme.label());
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
                                self.open_tab(TabKind::RecentTracks);
                                self.status = String::from("Opened recent tracks");
                            }
                            "search song" => {
                                self.open_tab(TabKind::Search);
                                self.search_input.clear();
                                self.search_results.clear();
                                self.status = String::from("Search for a song");
                            }
                            "settings" => {
                                self.open_tab(TabKind::Settings);
                                self.settings_username = self.config.username.clone();
                                self.settings_api_key = self.config.api_key.clone();
                                self.settings_theme = ThemeChoice::from_config(&self.config.theme);
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

        if self.active_tab == TabKind::Settings {
            match key {
                KeyCode::Esc => {
                    self.close_active_tab();
                    self.status = String::from("Closed settings");
                }
                KeyCode::Tab | KeyCode::Down => {
                    self.settings_field = match self.settings_field {
                        SettingsField::Username => SettingsField::ApiKey,
                        SettingsField::ApiKey => SettingsField::Theme,
                        SettingsField::Theme => SettingsField::Username,
                    };
                }
                KeyCode::Up => {
                    self.settings_field = match self.settings_field {
                        SettingsField::Username => SettingsField::Theme,
                        SettingsField::ApiKey => SettingsField::Username,
                        SettingsField::Theme => SettingsField::ApiKey,
                    };
                }
                KeyCode::Enter => {
                    self.save_settings()?;
                    self.active_tab = TabKind::Home;
                }
                KeyCode::Left if self.settings_field == SettingsField::Theme => {
                    self.settings_theme = self.settings_theme.prev();
                }
                KeyCode::Right if self.settings_field == SettingsField::Theme => {
                    self.settings_theme = self.settings_theme.next();
                }
                KeyCode::Backspace => match self.settings_field {
                    SettingsField::Username => {
                        self.settings_username.pop();
                    }
                    SettingsField::ApiKey => {
                        self.settings_api_key.pop();
                    }
                    SettingsField::Theme => {}
                },
                KeyCode::Char(c) => match self.settings_field {
                    SettingsField::Username => self.settings_username.push(c),
                    SettingsField::ApiKey => self.settings_api_key.push(c),
                    SettingsField::Theme => match c {
                        'h' => self.settings_theme = self.settings_theme.prev(),
                        'l' => self.settings_theme = self.settings_theme.next(),
                        _ => {}
                    },
                },
                _ => {}
            }

            return Ok(());
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('s') if self.active_tab != TabKind::Search => {
                self.command_palette_open = true;
                self.command_input.clear();
                self.command_selected = 0;
            }
            KeyCode::Tab => {
                self.cycle_tab();
            }
            KeyCode::Esc => {
                self.close_active_tab();
            }
            KeyCode::Enter if self.active_tab == TabKind::Search => {
                self.search_results = commands::search::fetch(&self.config, &self.search_input)?;
                self.status = format!("{} results for '{}'", self.search_results.len(), self.search_input);
            }
            KeyCode::Backspace if self.active_tab == TabKind::Search => {
                self.search_input.pop();
            }
            KeyCode::Char(c) if self.active_tab == TabKind::Search => {
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

fn current_theme(app: &App) -> UiTheme {
    match ThemeChoice::from_config(&app.config.theme) {
        ThemeChoice::CatppuccinMocha => UiTheme {
            background: Color::Rgb(30, 30, 46),
            panel_bg: Color::Rgb(24, 24, 37),
            border: Color::Rgb(137, 180, 250),
            text: Color::Rgb(205, 214, 244),
            muted: Color::Rgb(166, 173, 200),
            accent: Color::Rgb(203, 166, 247),
            selected_bg: Color::Rgb(49, 50, 68),
            selected_fg: Color::Rgb(245, 224, 220),
        },
        ThemeChoice::TokyoNight => UiTheme {
            background: Color::Rgb(26, 27, 38),
            panel_bg: Color::Rgb(31, 35, 53),
            border: Color::Rgb(122, 162, 247),
            text: Color::Rgb(192, 202, 245),
            muted: Color::Rgb(169, 177, 214),
            accent: Color::Rgb(187, 154, 247),
            selected_bg: Color::Rgb(41, 46, 66),
            selected_fg: Color::Rgb(224, 175, 104),
        },
        ThemeChoice::TopAlbum => UiTheme {
            background: Color::Reset,
            panel_bg: Color::Reset,
            border: Color::Rgb(180, 150, 110),
            text: Color::Rgb(220, 205, 185),
            muted: Color::Rgb(170, 155, 140),
            accent: Color::Rgb(235, 190, 120),
            selected_bg: Color::Reset,
            selected_fg: Color::Rgb(235, 190, 120),
        },
    }
}

fn themed_block<'a>(title: &'a str, theme: &UiTheme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().fg(theme.text).bg(theme.panel_bg))
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

fn tab_title(tab: TabKind) -> &'static str {
    match tab {
        TabKind::Home => "Home",
        TabKind::RecentTracks => "Now Playing",
        TabKind::Search => "Search Song",
        TabKind::Settings => "Settings",
    }
}

fn ui(frame: &mut Frame, app: &App) {
    let theme = current_theme(app);
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(frame.area());

    frame.render_widget(
        Block::default().style(Style::default().bg(theme.background)),
        frame.area(),
    );

    let labels: Vec<Line> = app
        .open_tabs
        .iter()
        .map(|tab| Line::from(tab_title(*tab)))
        .collect();

    let selected = app
        .open_tabs
        .iter()
        .position(|tab| *tab == app.active_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(labels)
        .select(selected)
        .style(Style::default().fg(theme.muted).bg(theme.panel_bg))
        .highlight_style(
            Style::default()
                .fg(theme.selected_fg)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD),
        )
        .block(themed_block("lastui", &theme));

    frame.render_widget(tabs, areas[0]);

    match app.active_tab {
        TabKind::Home => {
            match &app.dashboard_state {
                DashboardState::Loading => {
                    let panel = Paragraph::new(loading_label(app.loading_frame))
                        .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                        .block(themed_block("Home", &theme));

                    frame.render_widget(panel, areas[1]);
                }
                DashboardState::Empty => {
                    let panel = Paragraph::new("No user stats")
                        .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                        .block(themed_block("Home", &theme));

                    frame.render_widget(panel, areas[1]);
                }
                DashboardState::Error(message) => {
                    let panel = Paragraph::new(format!("Error: {message}"))
                        .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                        .block(themed_block("Home", &theme));

                    frame.render_widget(panel, areas[1]);
                }
                DashboardState::Loaded(data) => {
                    let home = themed_block("Home", &theme);
                    let inner = home.inner(areas[1]);
                    let columns = Layout::horizontal([
                        Constraint::Length(34),
                        Constraint::Min(1),
                    ])
                    .split(inner);

                    let art = Paragraph::new(data.art.clone())
                        .style(Style::default().fg(theme.text).bg(theme.panel_bg));

                    let stats_items: Vec<ListItem> = dashboard_stats_line(data)
                        .into_iter()
                        .map(ListItem::new)
                        .collect();

                    let stats = List::new(stats_items)
                        .style(Style::default().fg(theme.text).bg(theme.panel_bg));

                    frame.render_widget(home, areas[1]);
                    frame.render_widget(art, columns[0]);
                    frame.render_widget(stats, columns[1]);
                }
            }
        }
        TabKind::RecentTracks => {
            let items: Vec<ListItem> = app
                .recent_tracks
                .iter()
                .map(|t| ListItem::new(t.as_str()))
                .collect();

            let list = List::new(items)
                .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                .block(themed_block("Recent Tracks", &theme));

            frame.render_widget(list, areas[1]);
        }
        TabKind::Search => {
            let inner = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(areas[1]);

            let input = Paragraph::new(app.search_input.as_str())
                .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                .block(themed_block("Search", &theme));

            let results: Vec<ListItem> = app
                .search_results
                .iter()
                .map(|t| ListItem::new(t.as_str()))
                .collect();

            let list = List::new(results)
                .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                .block(themed_block("Results", &theme));

            frame.render_widget(input, inner[0]);
            frame.render_widget(list, inner[1]);
        }
        TabKind::Settings => {
            let rows = Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
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

            let theme_title = if app.settings_field == SettingsField::Theme {
                "Theme *"
            } else {
                "Theme"
            };

            let username = Paragraph::new(app.settings_username.as_str())
                .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                .block(themed_block(username_title, &theme));

            let api_key = Paragraph::new(masked_api_key(&app.settings_api_key))
                .style(Style::default().fg(theme.text).bg(theme.panel_bg))
                .block(themed_block(api_title, &theme));

            let theme_picker = Paragraph::new(app.settings_theme.label())
                .style(Style::default().fg(theme.accent).bg(theme.panel_bg))
                .block(themed_block(theme_title, &theme));

            let config_path = Paragraph::new(config::config_path().display().to_string())
                .style(Style::default().fg(theme.muted).bg(theme.panel_bg))
                .block(themed_block("Config Path", &theme));

            let help = Paragraph::new("Tab: switch field, Left/Right: theme, Enter: save, Esc: cancel")
                .style(Style::default().fg(theme.muted).bg(theme.panel_bg))
                .block(themed_block("Help", &theme));

            frame.render_widget(username, rows[0]);
            frame.render_widget(api_key, rows[1]);
            frame.render_widget(theme_picker, rows[2]);
            frame.render_widget(config_path, rows[3]);
            frame.render_widget(help, rows[4]);
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
            .style(Style::default().fg(theme.text).bg(theme.panel_bg))
            .block(themed_block("Command", &theme));

        let items: Vec<ListItem> = matches
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let prefix = if idx == app.command_selected { "> " } else { " " };
                ListItem::new(format!("{prefix}{item}"))
            })
        .collect();

        let results = List::new(items)
            .style(Style::default().fg(theme.text).bg(theme.panel_bg))
            .block(themed_block("Matches", &theme));

        frame.render_widget(Clear, popup);
        frame.render_widget(themed_block("Commands", &theme), popup);
        frame.render_widget(input, inner[0]);
        frame.render_widget(results, inner[1]);
    }

    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(theme.muted).bg(theme.panel_bg))
        .block(themed_block("Status", &theme));

    frame.render_widget(status, areas[2]);
}

fn main() -> Result<()> {
    let cfg = config::load().unwrap_or_else(|| {
        let api_key = prompt("Enter your last.fm API key: ");
        let username = prompt("Enter your Last.fm username: ");
        config::save(&api_key, &username, "catppuccin-mocha").expect("Failed to save config");
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
            theme: cfg.theme.clone(),
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
