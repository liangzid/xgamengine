use crate::llm::client::{LlmClient, SseEvent};
use crate::memory::ConversationWindow;
use crate::prompt::builder;
use crate::state::GameState;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Output from a single engine step
#[derive(Debug, Clone)]
pub struct EngineOutput {
    pub narrative: String,
    pub meta_text: Option<String>,
    pub options: Vec<String>,
    pub scene_type: Option<String>,
    pub state_changes: Option<crate::state::StateChange>,
    pub round: i32,
    pub had_fallback: bool,  // true if options were fallback-generated
}

impl EngineOutput {
    /// Append the "custom input" option as the 5th choice
    pub fn with_custom_option(mut self) -> Self {
        self.options.push("✍ 自由输入".into());
        self
    }
}

pub struct Engine {
    pub state: GameState,
    pub window: ConversationWindow,
    pub template_dir: PathBuf,
    pub client: LlmClient,
    pub npc: String,
}

impl Engine {
    pub fn new(template_dir: PathBuf, client: LlmClient) -> Self {
        Self {
            state: GameState::default(),
            window: ConversationWindow::new(),
            template_dir,
            client,
            npc: "qingxu".into(),
        }
    }

    /// Start a new game with a custom opening prompt (for web/character creation)
    pub async fn start_game_ex(&mut self, opening_input: &str) -> Result<EngineOutput, String> {
        let messages = builder::build_messages(
            &self.template_dir, &self.state, &self.window, opening_input, &self.npc
        )?;

        let raw = self.client.chat_completion_sync(&messages).await?;
        let parsed = builder::parse_structured_response(&raw);

        let had_fallback = parsed.options.is_empty();

        self.window.append_turn(opening_input, &parsed.narrative);

        Ok(EngineOutput {
            narrative: parsed.narrative,
            meta_text: parsed.meta_text.or_else(|| Some("你准备如何开始你的修仙之旅？".into())),
            options: if had_fallback {
                vec!["开始修炼".into(), "探索周围环境".into(), "寻找师尊".into(), "检查自身状态".into()]
            } else { parsed.options },
            scene_type: parsed.scene_type,
            state_changes: None,
            round: 0,
            had_fallback,
        }.with_custom_option())
    }

    /// Start a new game — returns the opening narrative (blocking)
    pub async fn start_game(&mut self, _scenario: &str, _player_name: &str) -> Result<EngineOutput, String> {
        let opening_input = format!(
            "你睁开双眼，发现自己正盘坐在一处陌生的石洞中。灵气在四周流淌，你隐约记得自己刚刚拜入{}。请描述此刻的场景，并引入我的师尊清虚道人。",
            self.state.sect
        );

        let messages = builder::build_messages(
            &self.template_dir, &self.state, &self.window, &opening_input, &self.npc
        )?;

        let raw = self.client.chat_completion_sync(&messages).await?;
        let parsed = builder::parse_structured_response(&raw);

        let had_fallback = parsed.options.is_empty();

        self.window.append_turn(&opening_input, &parsed.narrative);

        Ok(EngineOutput {
            narrative: parsed.narrative,
            meta_text: parsed.meta_text.or_else(|| Some("你准备如何开始你的修仙之旅？".into())),
            options: if had_fallback {
                vec!["开始修炼".into(), "探索周围环境".into(), "寻找师尊".into(), "检查自身状态".into()]
            } else { parsed.options },
            scene_type: parsed.scene_type,
            state_changes: None,
            round: 0,
            had_fallback,
        }.with_custom_option())
    }

    /// Process a player input — returns output (blocking).
    /// Call LLM to extract state changes from narrative (JSON output mode)
    pub async fn extract_state_with_llm(&self, narrative: &str) -> crate::state::StateChange {
        let prompt = crate::prompt::builder::build_state_extraction_prompt(&self.state, narrative);
        let msgs = vec![
            crate::memory::Message { role: "system".into(), content: prompt }
        ];
        match self.client.chat_completion_sync(&msgs).await {
            Ok(raw) => crate::prompt::builder::parse_state_change_json(&raw, &self.state),
            Err(_) => crate::state::StateChange::default(),
        }
    }

    /// Retries once with a format reminder if the AI doesn't produce options.
    pub async fn process_input(&mut self, user_input: &str) -> Result<EngineOutput, String> {
        self.state.round += 1;

        // ---- Compaction: compress old history if approaching context limit ----
        if self.window.needs_compaction() {
            if let Some(compact_prompt) = self.window.prepare_compaction() {
                // Call LLM to summarize old history
                let compact_msgs = vec![
                    crate::memory::Message { role: "system".into(), content: compact_prompt }
                ];
                if let Ok(summary_raw) = self.client.chat_completion_sync(&compact_msgs).await {
                    self.window.apply_compaction(&summary_raw);
                }
            }
        }

        // Append format reminder to user input
        let input_with_hint = format!("{}\n\n（必须严格包含：[叙事正文] + --- + [元文本] + [选项] + 恰好4个选项，否则视为无效回复）", user_input);

        let messages = builder::build_messages(
            &self.template_dir, &self.state, &self.window, &input_with_hint, &self.npc
        )?;

        let raw = self.client.chat_completion_sync(&messages).await?;
        let mut parsed = builder::parse_structured_response(&raw);

        if parsed.narrative.is_empty() {
            return Err("No narrative in API response".into());
        }

        // Retry once if no options
        let options_empty = parsed.options.is_empty();
        if options_empty {
            let retry_input = format!(
                "{}\n\n【重要：你上次回复缺少[选项]部分。请严格按照以下格式重新回复：\n[叙事正文]\n...\n---\n[元文本]\n...\n[选项]\n1. ...\n2. ...\n3. ...\n4. ...】",
                user_input
            );
            let retry_messages = builder::build_messages(
                &self.template_dir, &self.state, &self.window, &retry_input, &self.npc
            )?;
            if let Ok(retry_raw) = self.client.chat_completion_sync(&retry_messages).await {
                let retry_parsed = builder::parse_structured_response(&retry_raw);
                if !retry_parsed.options.is_empty() {
                    parsed = retry_parsed;
                }
            }
        }

        let had_fallback = parsed.options.is_empty();
        let narrative = parsed.narrative.clone();
        let meta_text = parsed.meta_text.clone();
        let scene_type = parsed.scene_type.clone();
        let final_options = if parsed.options.is_empty() {
            self.generate_fallback_options()
        } else {
            parsed.options
        };

        // ---- LLM-powered state extraction ----
        let changes = self.extract_state_with_llm(&narrative).await;
        self.state.apply_state_change(&changes);
        self.state.last_narrative = narrative.clone();

        self.window.append_turn(user_input, &narrative);

        Ok(EngineOutput {
            narrative,
            meta_text,
            options: final_options,
            scene_type,
            state_changes: Some(changes),
            round: self.state.round,
            had_fallback,
        }.with_custom_option())
    }

    /// Generate sensible fallback options based on current state
    pub fn generate_fallback_options(&self) -> Vec<String> {
        let mut opts = vec!["继续修炼".into()];
        if self.state.qi < 50 {
            opts.push("恢复灵力".into());
        } else {
            opts.push("研习功法".into());
        }
        if self.state.realm_progress > 0.7 {
            opts.push("准备突破".into());
        } else {
            opts.push("查看状态".into());
        }
        opts.push("探索周围".into());
        opts.push("与师尊交谈".into());
        opts.truncate(4);
        opts
    }

    /// Process input with streaming — sends chunks via tx channel
    /// Returns the final EngineOutput after stream completes
    pub async fn process_input_streaming(
        &mut self,
        user_input: &str,
        tx: mpsc::UnboundedSender<SseEvent>,
    ) -> Result<EngineOutput, String> {
        self.state.round += 1;

        let messages = builder::build_messages(
            &self.template_dir, &self.state, &self.window, user_input, &self.npc
        )?;

        // Start streaming
        self.client.chat_completion_streaming(&messages, tx.clone()).await?;

        // Stream is done — we still need the full text for parsing
        // For now, just call sync again for parsing
        // TODO: accumulate from stream for real streaming + parsing
        let raw = self.client.chat_completion_sync(&messages).await?;
        let parsed = builder::parse_structured_response(&raw);

        let changes = crate::state::StateChange::default(); // streaming path: skip extraction for now
        self.state.apply_state_change(&changes);
        self.state.last_narrative = parsed.narrative.clone();
        self.window.append_turn(user_input, &parsed.narrative);

        Ok(EngineOutput {
            narrative: parsed.narrative,
            meta_text: parsed.meta_text,
            options: parsed.options,
            scene_type: parsed.scene_type,
            state_changes: Some(changes),
            round: self.state.round,
            had_fallback: false,
        }.with_custom_option())
    }

    pub fn save_game(&self, path: &str) -> Result<(), String> {
        let data = serde_json::json!({
            "state": self.state,
            "window": self.window,
            "npc": self.npc,
        });
        std::fs::write(path, data.to_string())
            .map_err(|e| format!("Save failed: {}", e))
    }

    pub fn load_game(&mut self, path: &str) -> Result<(), String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("Load failed: {}", e))?;
        let data: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| format!("Invalid save format: {}", e))?;

        // Load state
        if let Some(state_val) = data.get("state") {
            self.state = serde_json::from_value(state_val.clone())
                .map_err(|e| format!("State parse error: {}", e))?;
        }

        // Load window (conversation history)
        if let Some(win_val) = data.get("window") {
            self.window = serde_json::from_value(win_val.clone())
                .unwrap_or_else(|_| ConversationWindow::new());
        }

        // Load npc
        if let Some(npc_val) = data.get("npc") {
            self.npc = serde_json::from_value(npc_val.clone())
                .unwrap_or_else(|_| "qingxu".into());
        }

        Ok(())
    }
}
