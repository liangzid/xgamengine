use super::loader;
use crate::state::GameState;
use crate::memory::{ConversationWindow, Message};
use std::path::PathBuf;

const FORMAT_INSTRUCTION: &str = r#"每次回复必须严格遵循以下格式，必须以 --- 作为分隔符（独占一行），违反则无效：

[叙事正文]
（150-300字的修仙叙事）

---
[元文本]
（以天道玉简或相似隐喻开头，简短询问玩家下一步行动）

[选项]
1. 选项一（简短）
2. 选项二（简短）
3. 选项三（简短）
4. 选项四（简短）

必须恰好4个选项。"#;

/// Build the full system prompt
pub fn build_system_prompt(
    template_dir: &PathBuf,
    state: &GameState,
    npc: &str,
) -> Result<String, String> {
    let world_rules_raw = loader::load_template(template_dir, "world-rules")?;
    let guardrails_raw = loader::load_template(template_dir, "guardrails")?;
    let npc_card = loader::load_template(template_dir, &format!("npc-{}", npc))?;
    let narrative = state.to_narrative();

    let world_rules = loader::render_template(&world_rules_raw, &[
        ("state-narrative", &narrative)
    ]);

    Ok(format!("{}\n\n{}\n\n{}\n\n{}",
        world_rules, guardrails_raw, npc_card, FORMAT_INSTRUCTION))
}

/// Build the full messages array
pub fn build_messages(
    template_dir: &PathBuf,
    state: &GameState,
    window: &ConversationWindow,
    user_input: &str,
    npc: &str,
) -> Result<Vec<Message>, String> {
    let system_prompt = build_system_prompt(template_dir, state, npc)?;
    let mut messages = vec![
        Message { role: "system".into(), content: system_prompt }
    ];
    messages.extend(window.get_context_messages());
    messages.push(Message { role: "user".into(), content: user_input.into() });
    Ok(messages)
}

/// Parsed structured AI response
#[derive(Debug, Clone)]
pub struct StructuredResponse {
    pub narrative: String,
    pub meta_text: Option<String>,
    pub options: Vec<String>,
    pub scene_type: Option<String>,
}

/// Parse the structured AI response
pub fn parse_structured_response(raw: &str) -> StructuredResponse {
    // Try "\n---\n" first (exact separator), then bare "---" as fallback
    let sep = if raw.contains("\n---\n") { "\n---\n" } else { "---" };
    let segments: Vec<&str> = raw.splitn(2, sep).collect();
    let narrative = segments.first().map(|s| s.trim().to_string()).unwrap_or_default();
    let trailing = segments.get(1).map(|s| s.trim()).unwrap_or("");

    if trailing.is_empty() {
        return StructuredResponse {
            narrative,
            meta_text: None,
            options: vec![],
            scene_type: None,
        };
    }

    let meta_text = extract_section(trailing, "[元文本]", "[选项]");
    let options = extract_options(trailing);
    let scene_type = detect_scene_type(&narrative);

    StructuredResponse { narrative, meta_text, options, scene_type }
}

fn extract_section(text: &str, start_marker: &str, end_marker: &str) -> Option<String> {
    let start = text.find(start_marker)?;
    let content_start = start + start_marker.len();
    let content_end = text[content_start..].find(end_marker)
        .map(|i| content_start + i)
        .unwrap_or(text.len());
    let raw = &text[content_start..content_end];
    let trimmed = raw.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn extract_options(text: &str) -> Vec<String> {
    let opt_start = match text.find("[选项]") {
        Some(i) => i + "[选项]".len(),
        None => return vec![],
    };
    let opt_text = &text[opt_start..];
    let mut options = Vec::new();

    for line in opt_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // Try multiple patterns:
        // "1. xxx" "1、xxx" "1) xxx" "1）xxx" "1 xxx"
        // "一、xxx" "一. xxx"
        // "- xxx" (bullet)
        // "**1.** xxx" (markdown bold)
        let mut after_prefix: Option<&str> = None;

        if let Some(rest) = trimmed.strip_prefix("- ") {
            after_prefix = Some(rest);
        }

        // Try markdown bold "**1.**" or "**1**" patterns
        if after_prefix.is_none() {
            if let Some(rest) = trimmed.strip_prefix("**") {
                // Find the closing "**" or ".**" and strip everything up to that
                if let Some(content_start) = rest.find("**") {
                    let after_bold = &rest[content_start + 2..];
                    let clean = after_bold.strip_prefix('.').unwrap_or(after_bold).trim();
                    if !clean.is_empty() {
                        after_prefix = Some(clean);
                    }
                }
            }
        }

        // Try numbered patterns: strip "1. " or "1." or "1、" etc
        if after_prefix.is_none() {
            let chars: Vec<char> = trimmed.chars().collect();
            if chars.len() >= 2 {
                let c0 = chars[0];
                let c1 = chars[1];
                let is_num = c0.is_ascii_digit() || matches!(c0, '一' | '二' | '三' | '四' | '五');
                let is_sep = matches!(c1, '.' | '、' | '）' | ')' | ' ');
                if is_num && is_sep {
                    after_prefix = Some(&trimmed[2..]);
                }
            }
        }

        // Try single-char prefix: "1 xxx" where space follows digit
        if after_prefix.is_none() {
            let chars: Vec<char> = trimmed.chars().collect();
            if chars.len() >= 1 && (chars[0].is_ascii_digit()
                || matches!(chars[0], '一' | '二' | '三' | '四' | '五')) {
                after_prefix = Some(&trimmed[1..]);
            }
        }

        if let Some(rest) = after_prefix {
            let clean = rest.trim();
            if !clean.is_empty() && clean.len() >= 1 {
                options.push(clean.to_string());
            }
        }
    }

    // Fallback: if structured parsing yielded < 2 options, treat each non-empty
    // line in the options section as a raw option (the AI may have used bare text)
    if options.len() < 2 {
        options.clear();
        for line in opt_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            // Skip lines that are clearly meta-text or separators
            if trimmed.starts_with('[') || trimmed == "---" { continue; }
            if trimmed.len() > 1 {
                options.push(trimmed.to_string());
            }
        }
    }

    options.truncate(4);
    options
}

fn detect_scene_type(text: &str) -> Option<String> {
    if text.contains("闭关") || text.contains("修炼") || text.contains("吐纳")
        || text.contains("突破") || text.contains("丹田") {
        Some("cultivation".into())
    } else if text.contains("出剑") || text.contains("斩") || text.contains("战斗")
        || text.contains("交锋") || text.contains("迎战") {
        Some("combat".into())
    } else if text.contains("辩论") || text.contains("论道") || text.contains("说服") {
        Some("debate".into())
    } else if text.contains("探索") || text.contains("秘境") || text.contains("遗迹")
        || text.contains("陌生") {
        Some("exploration".into())
    } else if text.contains("坊市") || text.contains("交易") || text.contains("丹药")
        || text.contains("买卖") {
        Some("trade".into())
    } else {
        None
    }
}

/// Format inventory with quantities for the state extraction prompt
fn format_items_with_qty(state: &crate::state::GameState) -> String {
    state.inventory.iter()
        .map(|i| format!("{} x{}", i.name, i.quantity))
        .collect::<Vec<_>>()
        .join("、")
}

/// Build the prompt for LLM-powered state extraction.
/// Includes dedup instructions to prevent duplicate items/techniques/flags.
/// `recent_context` should be the last 2-3 rounds of user→assistant conversation
/// to help the LLM track entity identity across turns.
pub fn build_state_extraction_prompt(state: &crate::state::GameState, narrative: &str, recent_context: &str) -> String {
    let existing_techs: Vec<String> = state.techniques.iter().map(|t| t.name.clone()).collect();
    let existing_flags: Vec<String> = state.flags.clone();
    let existing_locs: Vec<String> = state.locations.clone();
    let existing_quests: Vec<String> = state.quests.iter().map(|q| q.name.clone()).collect();
    let existing_rels: Vec<String> = state.relationships.iter().map(|r| r.name.clone()).collect();

    format!(
        r#"你是一个修仙游戏状态解析器。分析以下叙事，提取**仅本回合**发生的玩家状态变化。输出纯JSON。

最近对话（追踪角色身份和物品消耗）:
{}

当前已有（勿重复添加）:
- 功法: {}
- 物品（含数量）: {}
- 已探索地点: {}
- 任务: {}
- 人物关系: {}
- flag: {}

当前数值:
- 境界: {} (进度 {:.0}%)
- 灵力: {}/{}
- 六维: 剑道{} 术法{} 气血{} 神魂{} 神识{} 道心{}
- 灵石: {}
- 当前地点: {}

本回合叙事:
{}

输出JSON（严格按此schema，无字段则null或空数组）:
{{
  "realm_progress": 0.05,
  "qi_delta": -10,
  "qi_set": null,
  "max_qi_delta": null,
  "spirit_stones_delta": 30,
  "sword_art_delta": null,
  "spell_art_delta": null,
  "blood_qi_delta": null,
  "spirit_soul_delta": null,
  "divine_sense_delta": null,
  "dao_heart_delta": null,
  "add_techniques": [],
  "add_items": [],
  "remove_items": [],
  "consume_items": [],
  "relationship_changes": [],
  "rename_relationships": [],
  "new_locations": [],
  "set_current_location": null,
  "quest_updates": [],
  "add_flag": null,
  "add_event": null
}}

关键规则:
1. 只提取**本回合叙事中明确发生**的变化，不要推测
2. **绝不重复添加**已存在于"当前已有"列表中的功法、物品、地点、任务、人物、flag
3. 新增物品时提供完整 name/item_type/quality/quantity/effect。**消耗物品时用 consume_items**（如 "consume_items": [{{"name":"凝脉丹","quantity":1}}]），系统会自动减少库存
4. 人物好感变化用 relationship_changes。**已知角色获得新名字时用 rename_relationships**（如 "rename_relationships": [{{"old_name":"未知壮汉","new_name":"张三"}}]），不要重复添加
5. 数值变化精确反映叙事，不要编造"#,
        recent_context,
        existing_techs.join("、"), 
        format_items_with_qty(state),
        existing_locs.join("、"), existing_quests.join("、"),
        existing_rels.join("、"), existing_flags.join("、"),
        state.realm, state.realm_progress * 100.0,
        state.qi, state.max_qi,
        state.stats.sword_art, state.stats.spell_art,
        state.stats.blood_qi, state.stats.spirit_soul,
        state.stats.divine_sense, state.stats.dao_heart,
        state.spirit_stones,
        state.current_location,
        narrative,
    )
}

/// Pre-process JSON: convert string elements to objects in add_techniques and add_items.
/// LLM sometimes outputs ["清心诀"] instead of [{"name":"清心诀",...}]
fn fix_string_arrays(json_str: &str) -> String {
    let mut val: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return json_str.to_string(),
    };

    if let Some(obj) = val.as_object_mut() {
        // Fix add_techniques: convert strings to Technique objects
        if let Some(arr) = obj.get_mut("add_techniques").and_then(|v| v.as_array_mut()) {
            for item in arr.iter_mut() {
                if item.is_string() {
                    let name = item.as_str().unwrap_or("").to_string();
                    *item = serde_json::json!({
                        "name": name,
                        "tier": "黄阶",
                        "tech_type": "心法",
                        "proficiency": 0.1
                    });
                }
            }
        }
        // Fix add_items: convert strings to InventoryItem objects
        if let Some(arr) = obj.get_mut("add_items").and_then(|v| v.as_array_mut()) {
            for item in arr.iter_mut() {
                if item.is_string() {
                    let name = item.as_str().unwrap_or("").to_string();
                    *item = serde_json::json!({
                        "name": name,
                        "item_type": "杂物",
                        "quality": "普通",
                        "quantity": 1,
                        "effect": ""
                    });
                }
            }
        }
    }

    val.to_string()
}

/// Parse and review the LLM's JSON response into a StateChange.
/// Deduplicates: skips techniques/items/flags/locations/quests already present.
pub fn parse_state_change_json(json_str: &str, state: &crate::state::GameState) -> crate::state::StateChange {
    let cleaned = json_str
        .trim()
        .trim_start_matches("```json")
        .trim()
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Pre-process: the LLM sometimes outputs technique/item names as plain strings
    // instead of objects. Convert them: "清心诀" → {"name":"清心诀"}
    let fixed = fix_string_arrays(&cleaned);

    let change: crate::state::StateChange = match serde_json::from_str(&fixed) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[state extraction] JSON parse error: {}. Raw (first 300 chars): {}",
                e, &cleaned.chars().take(300).collect::<String>());
            return crate::state::StateChange::default();
        }
    };

    // ... dedup code continues
    let mut change = change;

    // ---- Dedup review ----
    // Remove already-known techniques
    if let Some(ref techs) = change.add_techniques {
        let filtered: Vec<_> = techs.iter()
            .filter(|t| !state.techniques.iter().any(|existing| existing.name == t.name))
            .cloned()
            .collect();
        change.add_techniques = if filtered.is_empty() { None } else { Some(filtered) };
    }

    // Remove already-known items (just filter by name, merge quantity handled in apply)
    if let Some(ref items) = change.add_items {
        let filtered: Vec<_> = items.iter()
            .filter(|i| !state.inventory.iter().any(|existing| existing.name == i.name))
            .cloned()
            .collect();
        change.add_items = if filtered.is_empty() { None } else { Some(filtered) };
    }

    // Remove already-known locations
    if let Some(ref locs) = change.new_locations {
        let filtered: Vec<_> = locs.iter()
            .filter(|l| !state.locations.contains(l))
            .cloned()
            .collect();
        change.new_locations = if filtered.is_empty() { None } else { Some(filtered) };
    }

    // Remove already-known flags
    if let Some(ref flag) = change.add_flag {
        if state.flags.contains(flag) {
            change.add_flag = None;
        }
    }

    // Clamp realm_progress to sane range
    if let Some(rp) = change.realm_progress {
        change.realm_progress = Some(rp.clamp(-0.3, 0.5));
    }

    // Clamp stat changes to reasonable range
    for delta in [&mut change.sword_art_delta, &mut change.spell_art_delta,
                  &mut change.blood_qi_delta, &mut change.spirit_soul_delta,
                  &mut change.divine_sense_delta, &mut change.dao_heart_delta] {
        if let Some(ref mut d) = delta {
            *d = (*d).clamp(-5, 5);
        }
    }

    change
}
