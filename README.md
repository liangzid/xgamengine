# xgamengine

An AI-text-based game engine for xianxia (cultivation) simulation, written in Rust.

## Architecture

```
                  CLI (src/main.rs, TUI via ratatui)
                  Web (server/src/main.rs, axum + WebSocket)
                        │
                  Engine (src/engine.rs)
                        │
         ┌──────────────┼──────────────┐
         │              │              │
   Prompt Builder   LLM Client    Game State
   (builder.rs)    (client.rs)    (state.rs)
         │              │              │
   Prompt Loader   SSE Stream      Memory
   (loader.rs)     (streaming)    (memory.rs)
         │
   templates/*.md  (in xgame repo, not here)
```

## Dependencies

- Rust >= 1.91
- Key crates: ratatui, crossterm, tui-textarea, reqwest, tokio, serde, regex

## Quick Start

### CLI (TUI)

```bash
export DEEPSEEK_API_KEY="sk-..."
export XGAMENGINE_TEMPLATE_DIR="../templates"
cd xgamengine
cargo run --release
```

### Web Server

See `../server/` — a separate crate that depends on xgamengine as a library.

## Engine API (Rust)

```rust
use xgamengine::engine::{Engine, EngineOutput};
use xgamengine::llm::client::LlmClient;

let client = LlmClient::from_env()?;
let mut engine = Engine::new(template_dir, client);

// Start a new game
let output: EngineOutput = engine.start_game("qingyun", "无名").await?;

// Process player input (blocking)
let output: EngineOutput = engine.process_input("我想修炼").await?;

// Process player input (streaming)
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
let output: EngineOutput = engine.process_input_streaming("我想修炼", tx).await?;

// Save/Load
engine.save_game("save.json")?;
engine.load_game("save.json")?;
```

## File Structure

```
xgamengine/
├── Cargo.toml
├── README.md
├── bin/
│   ├── build.sh            # Release build script
│   └── xgamengine          # Launcher
├── src/
│   ├── lib.rs              # Crate root
│   ├── main.rs             # CLI entry point (TUI)
│   ├── engine.rs           # Main game loop
│   ├── state.rs            # Game state (serde JSON)
│   ├── memory.rs           # Conversation window + compaction
│   ├── llm/
│   │   ├── client.rs       # DeepSeek API (sync + SSE streaming)
│   │   └── mod.rs
│   ├── prompt/
│   │   ├── loader.rs       # Template file loader
│   │   ├── builder.rs      # Prompt assembly + structured parsing
│   │   └── mod.rs
│   ├── scenes/
│   │   ├── protocol.rs     # Scene type definitions
│   │   └── mod.rs
│   └── tui/
│       ├── app.rs          # ratatui app state + rendering
│       └── mod.rs
```

## Design Decisions

- **Rust rewrite (2025-06)**: Replaced Common Lisp with Rust for TUI reliability (ratatui), type safety, and single-binary deployment.
- **Structured output**: AI responses are parsed into narrative + meta-text + 4 options + scene type.
- **LLM-powered state extraction**: After each turn, a small LLM call extracts state changes as structured JSON.
- **Infinite conversation window**: Messages are retained indefinitely with intelligent compaction at 90% context (1M tokens for DeepSeek V4 Pro).
- **Template separation**: Prompt templates live in the private `xgame` repo (`../templates/`), loaded at runtime via `XGAMENGINE_TEMPLATE_DIR`.
- **max_tokens: 4096**: Sufficient for narrative + meta-text + 4 options while leaving room for conversation history.

## Game State

```rust
pub struct GameState {
    pub realm: String,              // 练气期初期 ~ 化神期圆满
    pub realm_progress: f32,        // 0.0 ~ 1.0
    pub qi: i32, pub max_qi: i32,
    pub stats: PlayerStats,         // 6-dim: physical/magical attack/defense, divine attack/defense
    pub techniques: Vec<Technique>, // name, tier, type, proficiency
    pub inventory: Vec<InventoryItem>, // name, type, quality, quantity, effect
    pub spirit_stones: i32,
    pub locations: Vec<String>, pub current_location: String,
    pub relationships: Vec<Relationship>, // name, role, affinity
    pub quests: Vec<Quest>,
    pub character_notes: HashMap<String, String>, // dynamic NPC knowledge
    // ...
}
```

## License

TBD
