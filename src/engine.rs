use crate::chronicle::Chronicle;
use crate::llm::client::{LlmClient, SseEvent};
use crate::memory::ConversationWindow;
use crate::prompt::builder;
use crate::state::{GameState, WorldConfig, CreationChoices};
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
    /// Real token usage from all LLM API calls made during this turn
    /// (main dialog + state extraction + any compaction/retries).
    pub tokens_this_turn: u64,
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
    pub world_config: WorldConfig,
    /// Legacy NPC name — kept for backward compat with old saves
    pub npc: String,
    pub chronicle: Chronicle,
}

impl Engine {
    pub fn new(template_dir: PathBuf, client: LlmClient) -> Self {
        let world_config = WorldConfig::default();
        let state = Self::build_initial_state(&world_config, "无名", "");
        Self {
            state,
            window: ConversationWindow::new(),
            template_dir,
            client,
            world_config,
            npc: "qingxu".into(),
            chronicle: Chronicle::new(),
        }
    }

    /// Build a GameState from a WorldConfig and player identity.
    fn build_initial_state(world_config: &WorldConfig, player_name: &str, _dao_name: &str) -> GameState {
        let mut state = GameState::default();
        state.sect = world_config.sect_name.clone();
        state.current_location = world_config.starting_location_name.clone();
        state.locations = vec![
            world_config.starting_location_name.clone(),
        ];
        state.relationships.clear();
        state.character_notes.clear();

        // Set up relationship with key NPC if present
        if world_config.key_npc_name != "无" && !world_config.key_npc_name.is_empty() {
            state.relationships = vec![
                crate::state::Relationship {
                    name: world_config.key_npc_name.clone(),
                    role: world_config.key_npc_role.clone(),
                    affinity: 20,
                }
            ];
            state.character_notes.insert(
                world_config.key_npc_name.clone(),
                world_config.key_npc_description.clone(),
            );
        }

        state.flags.push(format!("player-name-{}", player_name));
        state
    }

    /// Initialize engine state from CreationChoices and a generated WorldConfig.
    /// Sets up the GameState with calculated stats, items, and world settings.
    pub fn init_from_creation(
        &mut self,
        choices: &CreationChoices,
        world_config: WorldConfig,
    ) {
        let stats = choices.calculate_initial_stats();
        let spirit_stones = choices.calculate_initial_spirit_stones();
        let items = choices.calculate_initial_items();
        let flags = choices.collect_background_flags();

        self.world_config = world_config;
        self.state = Self::build_initial_state(&self.world_config, &choices.player_name, &choices.dao_name);
        self.state.stats = stats;
        self.state.spirit_stones = spirit_stones;
        self.state.inventory = items;
        self.state.techniques.clear(); // clear default 青云吐纳术 — LLM will assign appropriate starter
        for flag in flags {
            if !self.state.flags.contains(&flag) {
                self.state.flags.push(flag);
            }
        }
        self.window = ConversationWindow::new();
        self.chronicle = Chronicle::new();
        self.npc = self.world_config.key_npc_name.clone();
    }

    /// Generate the world from character creation choices via LLM.
    /// Returns the generated WorldConfig and the token usage for this call.
    pub async fn generate_world(&self, choices: &CreationChoices) -> Result<(WorldConfig, u64), String> {
        let prompt = builder::build_world_generation_prompt(choices);
        let msgs = vec![
            crate::memory::Message { role: "system".into(), content: prompt }
        ];
        let (raw, usage) = self.client.chat_completion_sync(&msgs).await?;
        let wc = builder::parse_world_config_json(&raw, choices)
            .ok_or_else(|| "Failed to parse world config from LLM response".to_string())?;
        Ok((wc, usage.total()))
    }

    /// Start a new game with a custom opening prompt (for web/character creation)
    pub async fn start_game_ex(&mut self, opening_input: &str) -> Result<EngineOutput, String> {
        let messages = builder::build_messages(
            &self.template_dir, &self.state, &self.window, opening_input, &self.world_config
        )?;

        let (raw, usage) = self.client.chat_completion_sync(&messages).await?;
        let parsed = builder::parse_structured_response(&raw);

        let had_fallback = parsed.options.is_empty();

        self.window.append_turn(opening_input, &raw);

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
            tokens_this_turn: usage.total(),
        }.with_custom_option())
    }

    /// Start a new game — returns the opening narrative (blocking)
    pub async fn start_game(&mut self, _scenario: &str, _player_name: &str) -> Result<EngineOutput, String> {
        let opening_input = format!(
            "你睁开双眼，发现自己正盘坐在一处陌生的石洞中。灵气在四周流淌，你隐约记得自己刚刚拜入{}。请描述此刻的场景，并引入我的师尊清虚道人。",
            self.state.sect
        );

        let messages = builder::build_messages_legacy(
            &self.template_dir, &self.state, &self.window, &opening_input, &self.npc
        )?;

        let (raw, usage) = self.client.chat_completion_sync(&messages).await?;
        let parsed = builder::parse_structured_response(&raw);

        let had_fallback = parsed.options.is_empty();

        self.window.append_turn(&opening_input, &raw);

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
            tokens_this_turn: usage.total(),
        }.with_custom_option())
    }

    /// Process a player input — returns output (blocking).
    /// Call LLM to extract state changes from narrative (JSON output mode).
    /// Returns (state_changes, token_usage).
    pub async fn extract_state_with_llm(&self, narrative: &str) -> (crate::state::StateChange, u64) {
        // Build structured turn log for entity tracking
        let context_msgs = self.window.get_context_messages();
        let recent: Vec<_> = context_msgs.iter()
            .filter(|m| m.role != "system")
            .rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
        
        let mut recent_context = String::new();
        let mut round_num = self.state.round.saturating_sub(recent.len() as i32 / 2).max(1);
        for chunk in recent.chunks(2) {
            let user_msg = chunk.iter().find(|m| m.role == "user");
            let asst_msg = chunk.iter().find(|m| m.role == "assistant");
            if let (Some(u), Some(a)) = (user_msg, asst_msg) {
                let narrative_summary = {
                    let p = crate::prompt::builder::parse_structured_response(&a.content);
                    let n = if p.narrative.is_empty() { &a.content } else { &p.narrative };
                    n.chars().take(150).collect::<String>()
                };
                recent_context.push_str(&format!(
                    "- 回合{}: 玩家选择 \"{}\" → 天道叙事: \"{}\"\n",
                    round_num,
                    &u.content.chars().take(80).collect::<String>(),
                    narrative_summary,
                ));
                round_num += 1;
            }
        }
        
        let prompt = crate::prompt::builder::build_state_extraction_prompt(&self.state, narrative, &recent_context);
        let msgs = vec![
            crate::memory::Message { role: "system".into(), content: prompt }
        ];
        match self.client.chat_completion_sync(&msgs).await {
            Ok((raw, usage)) => {
                let changes = crate::prompt::builder::parse_state_change_json(&raw, &self.state);
                (changes, usage.total())
            }
            Err(_) => (crate::state::StateChange::default(), 0),
        }
    }

    /// Retries once with a format reminder if the AI doesn't produce options.
    pub async fn process_input(&mut self, user_input: &str) -> Result<EngineOutput, String> {
        self.state.round += 1;
        let mut tokens_this_turn = 0u64;

        // ---- Compaction: compress old history if approaching context limit ----
        if self.window.needs_compaction() {
            if let Some(compact_prompt) = self.window.prepare_compaction() {
                // Call LLM to summarize old history
                let compact_msgs = vec![
                    crate::memory::Message { role: "system".into(), content: compact_prompt }
                ];
                if let Ok((summary_raw, usage)) = self.client.chat_completion_sync(&compact_msgs).await {
                    tokens_this_turn += usage.total();
                    self.window.apply_compaction(&summary_raw);
                }
            }
        }

        // Append format reminder to user input
        let input_with_hint = format!("{}\n\n（每次回复必须输出纯 JSON，格式：{{\"narrative\":\"...\",\"meta_text\":\"...\",\"options\":[\"...\",\"...\",\"...\",\"...\"]}}）", user_input);

        let messages = builder::build_messages(
            &self.template_dir, &self.state, &self.window, &input_with_hint, &self.world_config
        )?;

        let (raw, usage) = self.client.chat_completion_sync(&messages).await?;
        tokens_this_turn += usage.total();
        let mut parsed = builder::parse_structured_response(&raw);

        if parsed.narrative.is_empty() {
            return Err("No narrative in API response".into());
        }

        // Retry once if no options
        let options_empty = parsed.options.is_empty();
        if options_empty {
            let retry_input = format!(
                "{}\n\n【重要：你上次回复格式错误。请重新输出一个严格的 JSON 对象：{{\"narrative\":\"...\",\"meta_text\":\"...\",\"options\":[\"...\",\"...\",\"...\",\"...\"]}}】",
                user_input
            );
            let retry_messages = builder::build_messages(
                &self.template_dir, &self.state, &self.window, &retry_input, &self.world_config
            )?;
            if let Ok((retry_raw, retry_usage)) = self.client.chat_completion_sync(&retry_messages).await {
                tokens_this_turn += retry_usage.total();
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
            eprintln!("[engine] AI returned no options — empty, free input only (round {})", self.state.round);
            vec![]
        } else {
            parsed.options
        };

        // ---- LLM-powered state extraction ----
        let (changes, extraction_tokens) = self.extract_state_with_llm(&narrative).await;
        tokens_this_turn += extraction_tokens;
        self.state.apply_state_change(&changes);
        self.state.last_narrative = narrative.clone();

        // Record this round in the chronicle
        self.chronicle.record_round(
            self.state.round, &self.state.realm, &changes, &self.state.flags,
        );

        self.window.append_turn(user_input, &raw);

        Ok(EngineOutput {
            narrative,
            meta_text,
            options: final_options,
            scene_type,
            state_changes: Some(changes),
            round: self.state.round,
            had_fallback,
            tokens_this_turn,
        }.with_custom_option())
    }

    /// Generate context-aware fallback options based on current game state.
    /// These are used only when the AI fails to produce options — a last resort.
    pub fn generate_fallback_options(&self) -> Vec<String> {
        let mut opts: Vec<String> = Vec::new();

        // Cultivation options — always relevant
        if self.state.qi < 50 {
            opts.push("打坐恢复灵力".into());
        } else {
            opts.push("潜心修炼功法".into());
        }

        // Location-based options
        if self.state.current_location.contains("洞府") || self.state.current_location.contains("闭关") {
            opts.push("出关探索外界".into());
        } else if self.state.current_location.contains("坊市") {
            opts.push("浏览坊市摊位".into());
        } else if self.state.current_location.contains("殿") {
            opts.push("向师尊请教".into());
        } else {
            opts.push("继续探索此处".into());
        }

        // Inventory / resource options
        if !self.state.inventory.is_empty() {
            let item = &self.state.inventory[0];
            if item.item_type == "丹药" {
                opts.push(format!("服用{}", item.name));
            } else if item.item_type == "法器" {
                opts.push(format!("祭炼{}", item.name));
            } else if item.item_type == "功法" {
                opts.push(format!("研读{}", item.name));
            } else {
                opts.push("整理随身物品".into());
            }
        } else {
            opts.push("寻找可用资源".into());
        }

        // Relationship-driven options
        if let Some(rel) = self.state.relationships.first() {
            if rel.affinity > 50 {
                opts.push(format!("与{}深谈", rel.name));
            } else if rel.affinity < 0 {
                opts.push(format!("尝试缓和与{}的关系", rel.name));
            } else {
                opts.push(format!("拜访{}", rel.name));
            }
        } else {
            opts.push("结识新的道友".into());
        }

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
            &self.template_dir, &self.state, &self.window, user_input, &self.world_config
        )?;

        // Start streaming
        self.client.chat_completion_streaming(&messages, tx.clone()).await?;

        // Stream is done — we still need the full text for parsing
        // For now, just call sync again for parsing
        // TODO: accumulate from stream for real streaming + parsing
        let (raw, usage) = self.client.chat_completion_sync(&messages).await?;
        let parsed = builder::parse_structured_response(&raw);

        let changes = crate::state::StateChange::default(); // streaming path: skip extraction for now
        self.state.apply_state_change(&changes);
        self.state.last_narrative = parsed.narrative.clone();
        self.window.append_turn(user_input, &raw);

        Ok(EngineOutput {
            narrative: parsed.narrative,
            meta_text: parsed.meta_text,
            options: parsed.options,
            scene_type: parsed.scene_type,
            state_changes: Some(changes),
            round: self.state.round,
            had_fallback: false,
            tokens_this_turn: usage.total(),
        }.with_custom_option())
    }

    pub fn save_game(&self, path: &str) -> Result<(), String> {
        let data = serde_json::json!({
            "state": self.state,
            "window": self.window,
            "world_config": self.world_config,
            "npc": self.npc,
            "chronicle": self.chronicle,
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

        // Load world_config (may not exist in older saves)
        if let Some(wc_val) = data.get("world_config") {
            self.world_config = serde_json::from_value(wc_val.clone())
                .unwrap_or_else(|_| WorldConfig::default());
        }

        // Load npc
        if let Some(npc_val) = data.get("npc") {
            self.npc = serde_json::from_value(npc_val.clone())
                .unwrap_or_else(|_| "qingxu".into());
        }

        // Load chronicle (may not exist in older saves)
        if let Some(chronicle_val) = data.get("chronicle") {
            self.chronicle = serde_json::from_value(chronicle_val.clone())
                .unwrap_or_else(|_| Chronicle::new());
        }

        Ok(())
    }

    /// Summarize a chronicle volume via LLM. Returns the summary or error.
    pub async fn summarize_chronicle_volume(&self, volume_index: usize) -> Result<String, String> {
        let prompt = self.chronicle.build_summary_prompt(volume_index)
            .ok_or_else(|| "Volume index out of range".to_string())?;
        let msgs = vec![
            crate::memory::Message { role: "system".into(), content: prompt }
        ];
        self.client.chat_completion_sync(&msgs).await.map(|(s,_)| s)
    }

    /// Export the current game as a clean Markdown document.
    /// Contains only: header, chronicle, clean turn-by-turn log, and state snapshot.
    /// No system prompts, format markers, or internal artifacts.
    pub fn export_full_document(&self) -> String {
        let mut doc = String::new();
        doc.push_str("# 修仙录\n\n");
        let player_name = self.state.flags.iter()
            .find(|f| f.starts_with("player-name-"))
            .map(|f| f.replace("player-name-", ""))
            .unwrap_or_else(|| "无名".into());
        doc.push_str(&format!("**修士**: {}  |  **境界**: {}  |  **回合**: {}  |  **灵力**: {}/{}\n\n",
            player_name, self.state.realm, self.state.round, self.state.qi, self.state.max_qi));

        // Chronicle
        doc.push_str(&self.chronicle.to_markdown());
        doc.push_str("\n---\n\n");

        // Clean turn-by-turn log: skip system messages, strip format markers from assistant
        doc.push_str("## 修仙录\n\n");
        let context = self.window.get_context_messages();
        let mut user_action: Option<&str> = None;
        for msg in &context {
            match msg.role.as_str() {
                "user" => {
                    user_action = Some(&msg.content);
                }
                "assistant" => {
                    if let Some(action) = user_action.take() {
                        // Extract clean narrative only — no [元文本], no [选项]
                        let parsed = crate::prompt::builder::parse_structured_response(&msg.content);
                        let narrative = if parsed.narrative.is_empty() {
                            msg.content.lines().take(3).collect::<Vec<_>>().join("\n")
                        } else {
                            parsed.narrative
                        };
                        doc.push_str(&format!("> {}\n\n", action));
                        doc.push_str(&format!("{}\n\n", narrative));
                        doc.push_str("---\n\n");
                    }
                }
                _ => {} // skip system messages (compaction summaries, etc.)
            }
        }

        // State snapshot
        doc.push_str("## 当前状态\n\n");
        doc.push_str(&self.state.to_narrative());

        doc
    }
}
