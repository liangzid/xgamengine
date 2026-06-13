# xgamengine

An AI-text-based game engine for xianxia (cultivation) simulation, written in Common Lisp (SBCL).

## Architecture

```
                  CLI (bin/cli.lisp)
                        │
                  Engine (src/engine.lisp)
                        │
         ┌──────────────┼──────────────┐
         │              │              │
   Prompt Builder   LLM Client    Game State
   (builder.lisp)  (client.lisp)  (state.lisp)
         │              │              │
   Prompt Loader   SSE Stream      Memory
   (loader.lisp)   (stream.lisp)  (short-term.lisp)
         │
   templates/*.md  (in xgame repo, not here)
```

## Dependencies

- SBCL (>= 2.6)
- Quicklisp libraries: dexador, shasht, cl-ppcre, str, fiveam

## Quick Start

```bash
# Ensure SBCL + Quicklisp are installed
# Set your DeepSeek API key
export DEEPSEEK_API_KEY="sk-..."

# Run the CLI
cd xgamengine
sbcl --script bin/cli.lisp
```

## Running Tests

```bash
cd xgamengine
sbcl --noinform --load ~/quicklisp/setup.lisp \
  --eval '(push (truename ".") asdf:*central-registry*)' \
  --eval '(asdf:test-system :xgamengine)' \
  --eval '(quit)'
```

## Engine API

```lisp
;; Start a new game
(start-game :scenario "qingyun" :player-name "无名")
;; => engine-output with opening narrative

;; Process player input
(process-input "我想修炼")
;; => engine-output with narrative, state-changes, suggestions

;; Save/Load
(save-game "save.json")
(load-game "save.json")
```

## File Structure

```
xgamengine/
├── xgamengine.asd        # ASDF system definition
├── README.md
├── bin/
│   └── cli.lisp          # CLI entry point
├── src/
│   ├── package.lisp      # Package & exports
│   ├── json.lisp         # JSON utility wrapper
│   ├── engine.lisp       # Main game loop
│   ├── llm/
│   │   ├── client.lisp   # DeepSeek API client
│   │   └── stream.lisp   # SSE stream parser
│   ├── prompt/
│   │   ├── loader.lisp   # Template file loader
│   │   └── builder.lisp  # Prompt assembly
│   ├── state/
│   │   └── state.lisp    # Game state management
│   └── memory/
│       └── short-term.lisp # Conversation window
└── tests/
    ├── suite.lisp         # Unit tests
    └── integration-test.lisp # Integration tests
```

## Design Decisions

- **Non-streaming MVP**: Streaming SSE is parsed but not yet integrated into the main loop. The game uses synchronous API calls for simplicity.
- **Heuristic state extraction**: State changes are extracted from AI responses via pattern matching, not a separate LLM call. This keeps costs low.
- **Template separation**: Prompt templates live in the private `xgame` repo (`../templates/`), loaded at runtime. Change templates without recompiling.
- **JSON via shasht**: Thin wrapper (`json.lisp`) provides string-in/string-out JSON with plist-based data, using shasht's streaming parser underneath.

## License

TBD
