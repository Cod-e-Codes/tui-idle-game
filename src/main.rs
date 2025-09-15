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
}

impl Upgrade {
    fn new(name: &str, description: &str, base_cost: f64, cost_multiplier: f64, base_production: f64) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            base_cost,
            cost_multiplier,
            base_production,
            owned: 0,
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

struct GameState {
    gold: f64,
    gold_per_second: f64,
    upgrades: Vec<Upgrade>,
    selected_upgrade: usize,
    last_update: Instant,
    total_clicks: u64,
    show_help: bool,
}

impl Default for GameState {
    fn default() -> Self {
        let upgrades = vec![
            Upgrade::new("Pickaxe", "Basic mining tool (+0.1 gold/sec)", 10.0, 1.15, 0.1),
            Upgrade::new("Shovel", "Dig faster (+0.5 gold/sec)", 50.0, 1.15, 0.5),
            Upgrade::new("Drill", "Mechanical mining (+2.0 gold/sec)", 250.0, 1.15, 2.0),
            Upgrade::new("Excavator", "Heavy machinery (+8.0 gold/sec)", 1000.0, 1.15, 8.0),
            Upgrade::new("Mine Shaft", "Deep mining operation (+30.0 gold/sec)", 5000.0, 1.15, 30.0),
            Upgrade::new("Gold Factory", "Automated gold production (+100.0 gold/sec)", 25000.0, 1.15, 100.0),
        ];

        Self {
            gold: 0.0,
            gold_per_second: 0.0,
            upgrades,
            selected_upgrade: 0,
            last_update: Instant::now(),
            total_clicks: 0,
            show_help: false,
        }
    }
}

impl GameState {
    fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        // Calculate total gold per second from upgrades
        self.gold_per_second = self.upgrades.iter().map(|u| u.current_production()).sum();
        
        // Add gold based on time passed
        self.gold += self.gold_per_second * delta;
    }

    fn click_for_gold(&mut self) {
        self.gold += 1.0;
        self.total_clicks += 1;
    }

    fn buy_selected(&mut self) {
        if let Some(upgrade) = self.upgrades.get_mut(self.selected_upgrade) {
            if upgrade.can_afford(self.gold) {
                let cost = upgrade.purchase();
                self.gold -= cost;
            }
        }
    }

    fn select_next(&mut self) {
        if self.selected_upgrade < self.upgrades.len() - 1 {
            self.selected_upgrade += 1;
        }
    }

    fn select_previous(&mut self) {
        if self.selected_upgrade > 0 {
            self.selected_upgrade -= 1;
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
            Span::styled("🏭 TERMINAL GOLD MINE 🏭", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from(vec![
            Span::raw("Gold: "),
            Span::styled(GameState::format_number(app.game_state.gold), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" | Rate: "),
            Span::styled(format!("{}/sec", GameState::format_number(app.game_state.gold_per_second)), Style::default().fg(Color::Green)),
            Span::raw(" | Clicks: "),
            Span::styled(app.game_state.total_clicks.to_string(), Style::default().fg(Color::Cyan)),
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
            Span::styled("⛏️  CLICK FOR GOLD!  ⛏️", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("Press "),
            Span::styled("SPACE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to mine +1 gold")
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

    // Right side - Upgrades
    let upgrade_items: Vec<ListItem> = app.game_state.upgrades
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

            let content = vec![
                Line::from(vec![
                    Span::styled(format!("{} ({})", upgrade.name, upgrade.owned), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                ]),
                Line::from(vec![
                    Span::raw(format!("Cost: ")),
                    Span::styled(GameState::format_number(upgrade.current_cost()), Style::default().fg(cost_color)),
                    Span::raw(" | +"),
                    Span::styled(format!("{}/sec", GameState::format_number(upgrade.base_production)), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled(upgrade.description.clone(), Style::default().fg(Color::Gray))
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let upgrades = List::new(upgrade_items)
        .block(Block::default().borders(Borders::ALL).title(format!("Upgrades - Gold: {} (↑/↓ select, Enter buy)", GameState::format_number(app.game_state.gold))))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");
    f.render_widget(upgrades, main_chunks[1]);

    // Footer
    let footer_text = if app.game_state.show_help {
        "SPACE: Mine gold | ↑/↓: Select upgrade | ENTER: Buy upgrade | H: Toggle help | Q: Quit"
    } else {
        "Press H for help | Q to quit"
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
