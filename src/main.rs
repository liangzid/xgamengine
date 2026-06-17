use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::{CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;

use xgamengine::engine::Engine;
use xgamengine::llm::client::{LlmClient, SseEvent};
use xgamengine::prompt::builder;
use xgamengine::tui::app::{AppMode, AppState, render_input, render_prompt, render_spinner, tick_spinner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY not set");
    let client = LlmClient::new(api_key);
    let template_dir = std::env::var("XGAMENGINE_TEMPLATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("../templates"));

    let mut engine = Engine::new(template_dir, client);

    let save_dir = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&save_dir).ok();
    let autosave_path = save_dir.join("xgamengine_autosave.json");

    let start_output = if autosave_path.exists() && engine.load_game(&autosave_path.to_string_lossy()).is_ok() {
        // Resume: generate a simple return message
        xgamengine::engine::EngineOutput {
            narrative: format!("你回到了修仙世界。当前境界：{}，灵力：{}/{}，位于{}。",
                engine.state.realm, engine.state.qi, engine.state.max_qi,
                engine.state.current_location),
            meta_text: Some("天道玉简轻震，一道熟悉的意念传入神识：你的仙途仍在继续。要做什么？".into()),
            options: vec![
                "继续修炼".into(), "探索周围".into(),
                "查看状态".into(), "与师尊交谈".into(),
            ],
            scene_type: None,
            state_changes: None,
            round: engine.state.round,
            had_fallback: false,
        }.with_custom_option()
    } else {
        engine.start_game("qingyun", "无名").await
            .map_err(|e| format!("Start failed: {}", e))?
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new(engine.state.clone());
    app.set_output(start_output);

    // Streaming channel
    let (stream_tx, mut stream_rx) = mpsc::unbounded_channel::<SseEvent>();
    let mut stream_active = false;
    let mut stream_buffer = String::new();

    loop {
        // ---- Poll streaming channel ----
        if stream_active {
            let mut got_content = false;
            while let Ok(event) = stream_rx.try_recv() {
                match event {
                    SseEvent::Content(chunk) => {
                        stream_buffer.push_str(&chunk);
                        app.narrative = Some(stream_buffer.clone());
                        got_content = true;
                    }
                    SseEvent::Done => {
                        stream_active = false;
                        // Parse the accumulated text
                        let parsed = builder::parse_structured_response(&stream_buffer);
                        let input_text = app.status_message.clone(); // stored input
                        app.narrative = Some(parsed.narrative.clone());
                        app.meta_text = parsed.meta_text;
                        app.options = if parsed.options.is_empty() {
                            engine.generate_fallback_options()
                        } else { parsed.options };
                        app.scene_type = parsed.scene_type;
                        app.selected = 0;
                        app.mode = AppMode::Selecting;

                        let narrative = parsed.narrative.clone();
                        let changes = engine.extract_state_with_llm(&narrative).await;
                        engine.state.apply_state_change(&changes);
                        engine.state.last_narrative = narrative.clone();
                        engine.window.append_turn(&input_text, &narrative);
                        app.state = engine.state.clone();

                        if app.state.round % 5 == 0 {
                            let _ = engine.save_game(&autosave_path.to_string_lossy());
                        }

                        // Append custom option
                        app.options.push("✍ 自由输入".into());

                        stream_buffer.clear();
                        got_content = true;
                    }
                    SseEvent::Error(e) => {
                        stream_active = false;
                        app.status_message = format!("推演受阻: {}", e);
                        app.mode = AppMode::Selecting;
                        stream_buffer.clear();
                    }
                }
            }
            if got_content {
                terminal.draw(|f| render_frame(f, &app, stream_active))?;
            }
        }

        // ---- Handle quit ----
        if app.mode == AppMode::Quit {
            let _ = engine.save_game(&autosave_path.to_string_lossy());
            break;
        }

        // ---- Spinner tick ----
        if app.mode == AppMode::Loading && !stream_active {
            tick_spinner(&mut app);
        }

        // ---- Render ----
        if !stream_active || app.mode != AppMode::Loading {
            terminal.draw(|f| render_frame(f, &app, stream_active))?;
        }

        // ---- Handle input ----
        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }

                // Handle textarea input
                if app.mode == AppMode::CustomInput {
                    match key.code {
                        KeyCode::Esc => { app.mode = AppMode::Selecting; }
                        KeyCode::Enter => {
                            let input = app.textarea.lines().join("\n").trim().to_string();
                            if !input.is_empty() {
                                app.textarea = make_textarea();
                                stream_buffer.clear();
                                app.narrative = Some(String::new());
                                app.status_message = input.clone();
                                app.mode = AppMode::Loading;

                                // Start streaming
                                let msgs = builder::build_messages(
                                    &engine.template_dir, &engine.state,
                                    &engine.window, &input, &engine.npc,
                                ).unwrap_or_default();
                                let tx = stream_tx.clone();
                                let client = engine.client.clone();
                                tokio::spawn(async move {
                                    let _ = client.chat_completion_streaming(&msgs, tx).await;
                                });
                                stream_active = true;
                                engine.state.round += 1;
                            }
                        }
                        _ => { app.textarea.input(Event::Key(key)); }
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        app.mode = AppMode::Quit;
                    }
                    KeyCode::Char('j') | KeyCode::Down => { app.select_next(); }
                    KeyCode::Char('k') | KeyCode::Up => { app.select_prev(); }
                    KeyCode::Char(':') | KeyCode::Tab => {
                        app.mode = AppMode::CustomInput;
                        app.textarea = make_textarea();
                    }
                    KeyCode::Enter => {
                        if let Some(choice) = app.selected_option() {
                            if choice.contains("自由输入") {
                                app.mode = AppMode::CustomInput;
                                app.textarea = make_textarea();
                            } else {
                                stream_buffer.clear();
                                app.narrative = Some(String::new());
                                app.status_message = choice.clone();
                                app.mode = AppMode::Loading;

                                let msgs = builder::build_messages(
                                    &engine.template_dir, &engine.state,
                                    &engine.window, &choice, &engine.npc,
                                ).unwrap_or_default();
                                let tx = stream_tx.clone();
                                let client_clone = engine.client.clone();
                                tokio::spawn(async move {
                                    let _ = client_clone.chat_completion_streaming(&msgs, tx).await;
                                });
                                stream_active = true;
                                engine.state.round += 1;
                            }
                        }
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let n = c.to_digit(10).unwrap_or(0) as usize;
                        if n >= 1 && n <= 5 && n <= app.options.len() {
                            let is_custom = n == 5 || app.options.get(n - 1)
                                .map_or(false, |o| o.contains("自由输入"));
                            if is_custom {
                                app.mode = AppMode::CustomInput;
                                app.textarea = make_textarea();
                            } else if let Some(opt) = app.options.get(n - 1) {
                                let choice = opt.clone();
                                stream_buffer.clear();
                                app.narrative = Some(String::new());
                                app.status_message = choice.clone();
                                app.mode = AppMode::Loading;

                                let msgs = builder::build_messages(
                                    &engine.template_dir, &engine.state,
                                    &engine.window, &choice, &engine.npc,
                                ).unwrap_or_default();
                                let tx = stream_tx.clone();
                                let client_clone = engine.client.clone();
                                tokio::spawn(async move {
                                    let _ = client_clone.chat_completion_streaming(&msgs, tx).await;
                                });
                                stream_active = true;
                                engine.state.round += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    let _ = engine.save_game(&autosave_path.to_string_lossy());
    println!("道心不灭，仙途再续。告辞！");
    Ok(())
}

fn render_frame(f: &mut ratatui::Frame, app: &AppState, streaming: bool) {
    let main_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(6),
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(f.area());

    xgamengine::tui::app::render_ui(f, app);

    let bottom = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Min(5),
        ])
        .split(main_layout[1]);

    match app.mode {
        AppMode::CustomInput => render_input(f, main_layout[1], app),
        AppMode::Loading if streaming => {
            render_spinner(f, bottom[0], app);
            xgamengine::tui::app::render_options(f, bottom[1], &None, &[], 0);
        }
        AppMode::Loading => {
            render_spinner(f, main_layout[1], app);
        }
        _ => {
            xgamengine::tui::app::render_options(
                f, bottom[1], &app.meta_text, &app.options, app.selected
            );
        }
    }

    render_prompt(f, main_layout[2], app);
}

fn make_textarea() -> tui_textarea::TextArea<'static> {
    let mut ta = tui_textarea::TextArea::default();
    ta.set_block(ratatui::widgets::Block::default().title(" 输入行动 ").borders(ratatui::widgets::Borders::ALL));
    ta.set_placeholder_text("输入你的行动，Enter 提交，Esc 取消");
    ta
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".xgamengine"))
}
