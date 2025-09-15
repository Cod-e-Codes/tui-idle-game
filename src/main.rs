use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
};
use tokio::time::{interval, MissedTickBehavior};

#[derive(Clone)]
struct Upgrade {
    name: String,
    description: String,
    base_cost: f64,
    cost_multiplier: f64,
    base_production: f64,
    owned: u64,
    upgrade_type: UpgradeType,
}

#[derive(Clone, PartialEq)]
enum UpgradeType {
    Passive,
    Click,
}

#[derive(Clone)]
struct Achievement {
    name: String,
    description: String,
    completed: bool,
    target: f64,
    achievement_type: AchievementType,
}

#[derive(Clone)]
enum AchievementType {
    TotalGold(f64),
    GoldPerSecond(f64),
    TotalClicks(u64),
    ClickPower(f64),
    UpgradesPurchased(u64),
}

impl Upgrade {
    fn new(name: &str, description: &str, base_cost: f64, cost_multiplier: f64, base_production: f64, upgrade_type: UpgradeType) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            base_cost,
            cost_multiplier,
            base_production,
            owned: 0,
            upgrade_type,
        }
    }

    fn current_cost(&self) -> f64 {
        self.base_cost * self.cost_multiplier.powi(self.owned as i32)
    }

    fn current_production(&self) -> f64 {
        self.base_production * self.owned as f64
    }

    fn can_afford(&self, gold: f64) -> bool {
        gold >= self.current_cost()
    }

    fn purchase(&mut self) -> f64 {
        let cost = self.current_cost();
        self.owned += 1;
        cost
    }
}

impl Achievement {
    fn new(name: &str, description: &str, achievement_type: AchievementType) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            completed: false,
            target: match &achievement_type {
                AchievementType::TotalGold(t) => *t,
                AchievementType::GoldPerSecond(t) => *t,
                AchievementType::TotalClicks(t) => *t as f64,
                AchievementType::ClickPower(t) => *t,
                AchievementType::UpgradesPurchased(t) => *t as f64,
            },
            achievement_type,
        }
    }
}

#[derive(PartialEq)]
enum Tab {
    Passive,
    Click,
    Achievements,
}

struct GameState {
    gold: f64,
    gold_per_second: f64,
    click_power: f64,
    total_gold_earned: f64,
    total_upgrades_purchased: u64,
    upgrades: Vec<Upgrade>,
    achievements: Vec<Achievement>,
    selected_upgrade: usize,
    current_tab: Tab,
    last_update: Instant,
    total_clicks: u64,
    show_help: bool,
    last_click: Instant,
    click_cooldown: Duration,
}

impl Default for GameState {
    fn default() -> Self {
        let upgrades = vec![
            // Passive upgrades
            Upgrade::new("Pickaxe", "Basic mining tool (+0.1 gold/sec)", 10.0, 1.15, 0.1, UpgradeType::Passive),
            Upgrade::new("Shovel", "Dig faster (+0.5 gold/sec)", 50.0, 1.15, 0.5, UpgradeType::Passive),
            Upgrade::new("Drill", "Mechanical mining (+2.0 gold/sec)", 250.0, 1.15, 2.0, UpgradeType::Passive),
            Upgrade::new("Excavator", "Heavy machinery (+8.0 gold/sec)", 1000.0, 1.15, 8.0, UpgradeType::Passive),
            Upgrade::new("Mine Shaft", "Deep mining operation (+30.0 gold/sec)", 5000.0, 1.15, 30.0, UpgradeType::Passive),
            Upgrade::new("Gold Factory", "Automated gold production (+100.0 gold/sec)", 25000.0, 1.15, 100.0, UpgradeType::Passive),
            
            // Click upgrades
            Upgrade::new("Strong Arms", "Better swinging (+1 gold per click)", 25.0, 1.2, 1.0, UpgradeType::Click),
            Upgrade::new("Steel Tools", "Sharper equipment (+2 gold per click)", 100.0, 1.2, 2.0, UpgradeType::Click),
            Upgrade::new("Power Gloves", "Enhanced grip (+5 gold per click)", 500.0, 1.2, 5.0, UpgradeType::Click),
            Upgrade::new("Hydraulic Hammer", "Mechanized clicking (+10 gold per click)", 2500.0, 1.2, 10.0, UpgradeType::Click),
            Upgrade::new("Diamond Drill Bit", "Ultimate mining power (+25 gold per click)", 10000.0, 1.2, 25.0, UpgradeType::Click),
        ];

        let achievements = vec![
            Achievement::new("First Steps", "Earn 100 total gold", AchievementType::TotalGold(100.0)),
            Achievement::new("Getting Rich", "Earn 10,000 total gold", AchievementType::TotalGold(10000.0)),
            Achievement::new("Millionaire", "Earn 1,000,000 total gold", AchievementType::TotalGold(1000000.0)),
            Achievement::new("Passive Income", "Reach 10 gold per second", AchievementType::GoldPerSecond(10.0)),
            Achievement::new("Gold Rush", "Reach 100 gold per second", AchievementType::GoldPerSecond(100.0)),
            Achievement::new("Click Master", "Click 1,000 times", AchievementType::TotalClicks(1000)),
            Achievement::new("Power Clicker", "Reach 50 gold per click", AchievementType::ClickPower(50.0)),
            Achievement::new("Upgrade Collector", "Purchase 50 upgrades", AchievementType::UpgradesPurchased(50)),
        ];

        Self {
            gold: 0.0,
            gold_per_second: 0.0,
            click_power: 1.0,
            total_gold_earned: 0.0,
            total_upgrades_purchased: 0,
            upgrades,
            achievements,
            selected_upgrade: 0,
            current_tab: Tab::Passive,
            last_update: Instant::now(),
            total_clicks: 0,
            show_help: false,
            last_click: Instant::now() - Duration::from_secs(1),
            click_cooldown: Duration::from_millis(500),
        }
    }
}

impl GameState {
    fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        // Calculate total gold per second from passive upgrades
        self.gold_per_second = self.upgrades.iter()
            .filter(|u| u.upgrade_type == UpgradeType::Passive)
            .map(|u| u.current_production())
            .sum();
        
        // Calculate click power from click upgrades
        self.click_power = 1.0 + self.upgrades.iter()
            .filter(|u| u.upgrade_type == UpgradeType::Click)
            .map(|u| u.current_production())
            .sum::<f64>();

        // Add gold based on time passed
        let gold_earned = self.gold_per_second * delta;
        self.gold += gold_earned;
        self.total_gold_earned += gold_earned;

        // Check achievements
        let total_gold_earned = self.total_gold_earned;
        let gold_per_second = self.gold_per_second;
        let total_clicks = self.total_clicks;
        let click_power = self.click_power;
        let total_upgrades_purchased = self.total_upgrades_purchased;
        
        for achievement in &mut self.achievements {
            let current_value = match achievement.achievement_type {
                AchievementType::TotalGold(_) => total_gold_earned,
                AchievementType::GoldPerSecond(_) => gold_per_second,
                AchievementType::TotalClicks(_) => total_clicks as f64,
                AchievementType::ClickPower(_) => click_power,
                AchievementType::UpgradesPurchased(_) => total_upgrades_purchased as f64,
            };

            if !achievement.completed && current_value >= achievement.target {
                achievement.completed = true;
            }
        }
    }

    fn click_for_gold(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_click) >= self.click_cooldown {
            self.gold += self.click_power;
            self.total_gold_earned += self.click_power;
            self.total_clicks += 1;
            self.last_click = now;
        }
    }

    fn get_current_upgrades(&self) -> Vec<&Upgrade> {
        match self.current_tab {
            Tab::Passive => self.upgrades.iter().filter(|u| u.upgrade_type == UpgradeType::Passive).collect(),
            Tab::Click => self.upgrades.iter().filter(|u| u.upgrade_type == UpgradeType::Click).collect(),
            Tab::Achievements => Vec::new(),
        }
    }

    fn buy_selected(&mut self) {
        if self.current_tab == Tab::Achievements {
            return;
        }

        let current_upgrades = self.get_current_upgrades();
        if let Some(&upgrade) = current_upgrades.get(self.selected_upgrade) {
            if upgrade.can_afford(self.gold) {
                let upgrade_index = self.upgrades.iter().position(|u| 
                    u.name == upgrade.name && u.upgrade_type == upgrade.upgrade_type
                ).unwrap();
                
                let cost = self.upgrades[upgrade_index].purchase();
                self.gold -= cost;
                self.total_upgrades_purchased += 1;
            }
        }
    }

    fn select_next(&mut self) {
        let max_index = match self.current_tab {
            Tab::Passive => self.upgrades.iter().filter(|u| u.upgrade_type == UpgradeType::Passive).count(),
            Tab::Click => self.upgrades.iter().filter(|u| u.upgrade_type == UpgradeType::Click).count(),
            Tab::Achievements => self.achievements.len(),
        };
        
        if self.selected_upgrade < max_index.saturating_sub(1) {
            self.selected_upgrade += 1;
        }
    }

    fn select_previous(&mut self) {
        if self.selected_upgrade > 0 {
            self.selected_upgrade -= 1;
        }
    }

    fn switch_tab(&mut self, tab: Tab) {
        if self.current_tab != tab {
            self.current_tab = tab;
            self.selected_upgrade = 0;
        }
    }

    fn format_number(num: f64) -> String {
        if num >= 1_000_000.0 {
            format!("{:.2}M", num / 1_000_000.0)
        } else if num >= 1_000.0 {
            format!("{:.2}K", num / 1_000.0)
        } else {
            format!("{:.2}", num)
        }
    }
}

struct App {
    game_state: GameState,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            game_state: GameState::default(),
            should_quit: false,
        }
    }

    fn on_tick(&mut self) {
        self.game_state.update();
    }

    fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char(' ') => self.game_state.click_for_gold(),
            KeyCode::Enter => self.game_state.buy_selected(),
            KeyCode::Up => self.game_state.select_previous(),
            KeyCode::Down => self.game_state.select_next(),
            KeyCode::Char('h') => self.game_state.show_help = !self.game_state.show_help,
            KeyCode::Char('1') => self.game_state.switch_tab(Tab::Passive),
            KeyCode::Char('2') => self.game_state.switch_tab(Tab::Click),
            KeyCode::Char('3') => self.game_state.switch_tab(Tab::Achievements),
            _ => {}
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ].as_ref())
        .split(f.area());

    // Header
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("TERMINAL GOLD MINE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from(vec![
            Span::raw("Gold: "),
            Span::styled(GameState::format_number(app.game_state.gold), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" | Rate: "),
            Span::styled(format!("{}/sec", GameState::format_number(app.game_state.gold_per_second)), Style::default().fg(Color::Green)),
            Span::raw(" | Click: +"),
            Span::styled(GameState::format_number(app.game_state.click_power), Style::default().fg(Color::Cyan)),
            Span::raw(" | Total: "),
            Span::styled(GameState::format_number(app.game_state.total_gold_earned), Style::default().fg(Color::Magenta)),
        ])
    ])
    .block(Block::default().borders(Borders::ALL).title("Status"))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Main content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    // Left side - Click area and progress
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(5)].as_ref())
        .split(main_chunks[0]);

    let click_area = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("CLICK FOR GOLD!", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("SPACE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to mine +"),
            Span::styled(GameState::format_number(app.game_state.click_power), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" gold (0.5s cooldown)")
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Or just wait and earn "),
            Span::styled(format!("{} gold/sec", GameState::format_number(app.game_state.gold_per_second)), Style::default().fg(Color::Green)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Mining"))
    .alignment(Alignment::Center)
    .wrap(Wrap { trim: true });
    f.render_widget(click_area, left_chunks[0]);

    // Progress bar showing gold accumulation
    let progress = (app.game_state.gold % 100.0) / 100.0;
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Gold Progress"))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent((progress * 100.0) as u16)
        .label(format!("{:.1}%", progress * 100.0));
    f.render_widget(gauge, left_chunks[1]);

    // Tab headers
    let tab_titles = vec!["1-Passive", "2-Click", "3-Achievements"];
    let tab_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)].as_ref())
        .split(main_chunks[1]);

    for (i, title) in tab_titles.iter().enumerate() {
        let tab_type = match i {
            0 => Tab::Passive,
            1 => Tab::Click,
            _ => Tab::Achievements,
        };
        
        let style = if app.game_state.current_tab == tab_type {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let tab = Paragraph::new(*title)
            .style(style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(tab, tab_chunks[i]);
    }

    // Content area below tabs
    let content_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
        .split(main_chunks[1]);

    // Right side content based on selected tab
    match app.game_state.current_tab {
        Tab::Passive | Tab::Click => {
            let current_upgrades = app.game_state.get_current_upgrades();
            let upgrade_items: Vec<ListItem> = current_upgrades
                .iter()
                .enumerate()
                .map(|(i, upgrade)| {
                    let cost_color = if upgrade.can_afford(app.game_state.gold) {
                        Color::Green
                    } else {
                        Color::Red
                    };

                    let style = if i == app.game_state.selected_upgrade {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    let effect_text = match upgrade.upgrade_type {
                        UpgradeType::Passive => format!("+{}/sec", GameState::format_number(upgrade.base_production)),
                        UpgradeType::Click => format!("+{}/click", GameState::format_number(upgrade.base_production)),
                    };

                    let content = vec![
                        Line::from(vec![
                            Span::styled(format!("{} ({})", upgrade.name, upgrade.owned), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                        ]),
                        Line::from(vec![
                            Span::raw("Cost: "),
                            Span::styled(GameState::format_number(upgrade.current_cost()), Style::default().fg(cost_color)),
                            Span::raw(" | "),
                            Span::styled(effect_text, Style::default().fg(Color::Green)),
                        ]),
                        Line::from(vec![
                            Span::styled(upgrade.description.clone(), Style::default().fg(Color::Gray))
                        ]),
                    ];

                    ListItem::new(content).style(style)
                })
                .collect();

            let tab_name = match app.game_state.current_tab {
                Tab::Passive => "Passive Upgrades",
                Tab::Click => "Click Upgrades",
                _ => "Upgrades",
            };

            let upgrades = List::new(upgrade_items)
                .block(Block::default().borders(Borders::ALL).title(format!("{} - Gold: {} (Up/Down select, Enter buy)", tab_name, GameState::format_number(app.game_state.gold))))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            f.render_widget(upgrades, content_area[1]);
        }

        Tab::Achievements => {
            let achievement_items: Vec<ListItem> = app.game_state.achievements
                .iter()
                .enumerate()
                .map(|(i, achievement)| {
                    let style = if i == app.game_state.selected_upgrade {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };

                    let status_color = if achievement.completed {
                        Color::Green
                    } else {
                        Color::Yellow
                    };

                    let status_symbol = if achievement.completed { "[DONE]" } else { "[    ]" };

                    let current_value = match achievement.achievement_type {
                        AchievementType::TotalGold(_) => GameState::format_number(app.game_state.total_gold_earned),
                        AchievementType::GoldPerSecond(_) => GameState::format_number(app.game_state.gold_per_second),
                        AchievementType::TotalClicks(_) => app.game_state.total_clicks.to_string(),
                        AchievementType::ClickPower(_) => GameState::format_number(app.game_state.click_power),
                        AchievementType::UpgradesPurchased(_) => app.game_state.total_upgrades_purchased.to_string(),
                    };

                    let content = vec![
                        Line::from(vec![
                            Span::styled(format!("{} {}", status_symbol, achievement.name), Style::default().fg(status_color).add_modifier(Modifier::BOLD))
                        ]),
                        Line::from(vec![
                            Span::styled(achievement.description.clone(), Style::default().fg(Color::Gray))
                        ]),
                        Line::from(vec![
                            Span::raw("Progress: "),
                            Span::styled(current_value, Style::default().fg(Color::Cyan)),
                            Span::raw(" / "),
                            Span::styled(GameState::format_number(achievement.target), Style::default().fg(Color::White)),
                        ]),
                    ];

                    ListItem::new(content).style(style)
                })
                .collect();

            let completed_count = app.game_state.achievements.iter().filter(|a| a.completed).count();
            let total_count = app.game_state.achievements.len();

            let achievements = List::new(achievement_items)
                .block(Block::default().borders(Borders::ALL).title(format!("Achievements ({}/{}) - Long-term Goals", completed_count, total_count)))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            f.render_widget(achievements, content_area[1]);
        }
    }

    // Footer
    let footer_text = if app.game_state.show_help {
        "SPACE: Mine gold | Up/Down: Select | ENTER: Buy | 1: Passive | 2: Click | 3: Achievements | H: Toggle help | Q: Quit"
    } else {
        "Press H for help | 1-3: Switch tabs | Q to quit"
    };

    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut update_interval = interval(Duration::from_millis(100));
    update_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            _ = update_interval.tick() => {
                app.on_tick();
                if app.should_quit {
                    return Ok(());
                }
            }
            
            event = tokio::task::spawn_blocking(|| {
                if event::poll(Duration::from_millis(0)).unwrap_or(false) {
                    event::read().ok()
                } else {
                    None
                }
            }) => {
                if let Ok(Some(event)) = event {
                    if let Event::Key(key) = event {
                        if key.kind == KeyEventKind::Press {
                            app.on_key(key.code);
                            if app.should_quit {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
