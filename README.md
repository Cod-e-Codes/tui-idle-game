# TUI Idle Game

A terminal-based idle game built with Rust and ratatui. Players mine gold automatically and purchase upgrades to increase production rates.

## Features

* Automatic gold generation based on owned upgrades
* Manual gold mining through keyboard input
* Six upgrade tiers with exponential cost scaling
* Real-time progress display and statistics
* Keyboard navigation interface

## Requirements

* Rust 1.70 or later
* Terminal supporting ANSI escape sequences

## Installation

```bash
git clone https://github.com/Cod-e-Codes/tui-idle-game.git
cd tui-idle-game
cargo run
```

## Controls

* `Space` - Mine gold manually (+1 gold)
* `↑/↓` - Navigate upgrade list
* `Enter` - Purchase selected upgrade
* `H` - Toggle help display
* `Q` - Quit game

## Upgrades

| Item | Base Cost | Production | Cost Multiplier |
|------|-----------|------------|-----------------|
| Pickaxe | 10 | 0.1 gold/sec | 1.15x |
| Shovel | 50 | 0.5 gold/sec | 1.15x |
| Drill | 250 | 2.0 gold/sec | 1.15x |
| Excavator | 1,000 | 8.0 gold/sec | 1.15x |
| Mine Shaft | 5,000 | 30.0 gold/sec | 1.15x |
| Gold Factory | 25,000 | 100.0 gold/sec | 1.15x |

## Dependencies

* ratatui 0.26
* crossterm 0.27
* tokio 1.0

## License

MIT License
