# WoT Battle Analyzer

A World of Tanks statistics tracker for Windows, created by **Friend of Misery**. Analyzes local replays to show which maps and tanks you perform best with.

## Features

- Overview of win rate, damage, assistance, and battle history.
- Statistics by tank, class, tier, map, and date range.
- Battle details, credit balance, shot statistics, and player search.
- Automatic replay import, local archive, and custom SQL queries.
- Interface available in 13 languages.

## Getting started

With Rust installed:

```powershell
cargo run --release
```

Select your game and replay folders under **Settings**, then start an import. Enable recording of all battles in the game settings.

Statistics cover random battles from `.wotreplay` files. Missing final results are treated as unknown. All data stays on your computer; no login is required.

## Development

```powershell
cargo test
cargo build --release
```

Technical details: [Architecture](docs/architecture.md) (in German).

Independent project. World of Tanks is a trademark of Wargaming.
