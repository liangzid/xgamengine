use super::loader;
use crate::state::{GameState, WorldConfig, CreationChoices};
use crate::memory::{ConversationWindow, Message};
use std::path::PathBuf;

const FORMAT_INSTRUCTION: &str = r#"每次回复必须输出一个严格的 JSON 对象，不要添加任何文字、注释或代码块标记。

{"narrative":"…叙事…","meta_text":"…询问下一步…","options":["选项1","选项2","选项3","选项4"]}

要求：
- 必须恰好4个选项，每个简短具体（不超过15字），与当前场景和剧情紧密相关
- 至少1个选项提供与当前场景类型不同的方向（探索、交谈、修炼、战斗等交叉推荐）
- 选项排列考虑风险梯度：保守安全 → 到冒险激进
- 叙事中的对话引用中文引号「」，JSON 键和字符串分隔符仍用英文双引号"#;

/// Build the full system prompt for a game turn.
/// Uses WorldConfig for dynamic world setting and NPC handling.
pub fn build_system_prompt(
    template_dir: &PathBuf,
    state: &GameState,
    world_config: &WorldConfig,
) -> Result<String, String> {
    let world_rules_raw = loader::load_template(template_dir, "world-rules")?;
    let guardrails_raw = loader::load_template(template_dir, "guardrails")?;
    let narrative = state.to_narrative();

    let world_rules = loader::render_template(&world_rules_raw, &[
        ("state-narrative", &narrative)
    ]);

    // Build dynamic world section from WorldConfig
    let world_section = world_config.to_system_prompt_section();

    Ok(format!("{}\n\n{}\n\n{}\n\n{}",
        world_section, world_rules, guardrails_raw, FORMAT_INSTRUCTION))
}

/// Legacy build_system_prompt kept for backward compatibility.
/// Constructs a default WorldConfig and delegates to the main function.
pub fn build_system_prompt_legacy(
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

/// Build the full messages array for a game turn
pub fn build_messages(
    template_dir: &PathBuf,
    state: &GameState,
    window: &ConversationWindow,
    user_input: &str,
    world_config: &WorldConfig,
) -> Result<Vec<Message>, String> {
    let system_prompt = build_system_prompt(template_dir, state, world_config)?;
    let mut messages = vec![
        Message { role: "system".into(), content: system_prompt }
    ];
    messages.extend(window.get_context_messages());
    messages.push(Message { role: "user".into(), content: user_input.into() });
    Ok(messages)
}

/// Legacy build_messages for backward compatibility
pub fn build_messages_legacy(
    template_dir: &PathBuf,
    state: &GameState,
    window: &ConversationWindow,
    user_input: &str,
    npc: &str,
) -> Result<Vec<Message>, String> {
    let system_prompt = build_system_prompt_legacy(template_dir, state, npc)?;
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

/// JSON structure the AI is asked to output.
#[derive(Debug, Clone, serde::Deserialize)]
struct JsonResponse {
    narrative: Option<String>,
    meta_text: Option<String>,
    options: Option<Vec<String>>,
}

/// Parse the AI response. Tries JSON first, falls back to legacy text format.
pub fn parse_structured_response(raw: &str) -> StructuredResponse {
    // ── JSON path (primary) ──
    if let Some(result) = try_parse_json_response(raw) {
        return result;
    }

    // ── Legacy text path (fallback) ──
    parse_legacy_text_response(raw)
}

/// Attempt to find and parse a JSON object in the AI response.
fn try_parse_json_response(raw: &str) -> Option<StructuredResponse> {
    // ── First pass: strip markdown fences that LLMs love to add ──
    let mut cleaned = raw.trim().to_string();
    if cleaned.starts_with("```") {
        if let Some(after_fence) = cleaned.find('\n') {
            cleaned = cleaned[after_fence + 1..].to_string();
        }
    }
    if cleaned.ends_with("```") {
        cleaned = cleaned.trim_end_matches("```").trim_end().to_string();
    }

    // ── Find JSON object boundaries ──
    let start = cleaned.find('{')?;
    let end = cleaned.rfind('}')?;
    if end <= start {
        eprintln!("[json parser] no valid JSON object boundaries: start={}, end={}", start, end);
        return None;
    }
    let json_str = &cleaned[start..=end];

    // ── Parse ──
    let parsed: JsonResponse = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[json parser] serde error: {}. Raw (first 300 chars): {}",
                e, &json_str.chars().take(300).collect::<String>());
            return None;
        }
    };

    let narrative = parsed.narrative.unwrap_or_default();
    if narrative.is_empty() {
        eprintln!("[json parser] parsed OK but narrative is empty");
        return None;
    }

    let meta_text = parsed.meta_text.filter(|m| !m.is_empty());
    let options = parsed.options.unwrap_or_default()
        .into_iter()
        .filter(|o| !o.is_empty())
        .take(4)
        .collect::<Vec<_>>();

    let scene_type = detect_scene_type(&narrative);

    if options.len() < 4 {
        eprintln!("[json parser] only {} options parsed", options.len());
    }

    Some(StructuredResponse { narrative, meta_text, options, scene_type })
}

/// Legacy text-format parser (kept for backward compatibility).
fn parse_legacy_text_response(raw: &str) -> StructuredResponse {
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
/// ULTRA-EXPLICIT: forces the LLM to check every single state field methodically.
/// `recent_context` should be structured turn log for entity identity tracking.
pub fn build_state_extraction_prompt(state: &crate::state::GameState, narrative: &str, recent_context: &str) -> String {
    let existing_techs: Vec<String> = state.techniques.iter().map(|t| t.name.clone()).collect();
    let existing_flags: Vec<String> = state.flags.clone();
    let existing_locs: Vec<String> = state.locations.clone();
    let existing_quests: Vec<String> = state.quests.iter().map(|q| q.name.clone()).collect();
    let existing_rels: Vec<String> = state.relationships.iter().map(|r| r.name.clone()).collect();

    format!(
        r#"你是一个修仙游戏状态解析器。你的任务是逐项检查本回合叙事中玩家的状态是否发生了变化。

⚠️ 重要警告：大量测试表明，LLM经常遗漏状态变化。你必须极度仔细地阅读叙事中的每一句话。即使是很微妙的描写（如"灵力消耗殆尽""经脉稳固了一分""对某某微微点头"），也意味着状态变化。不要因为变化太小而忽略它。每一个字都可能意味着某个状态发生了变化。

你必须按以下清单，逐条检查。每一条都是一个具体的问题，你必须回答它。

══════════════════════════════════════════
最近对话记录（追踪角色身份和物品）:
{}

══════════════════════════════════════════
玩家当前全部状态（更新前）:
- 功法: {}
- 物品（含数量）: {}
- 已探索地点: {}
- 任务: {}
- 人物关系: {}
- flag: {}
- 境界: {}
- 灵力: {}/{}
- 六维: 剑道{} 术法{} 气血{} 神魂{} 神识{} 道心{}
- 灵石: {}
- 当前地点: {}

══════════════════════════════════════════
本回合叙事（玩家行动后发生的事）:
{}

══════════════════════════════════════════
请逐项回答以下问题。每一项都必须给出答案。如果叙事中没有发生该变化，填 null 或空数组。

1. 境界名称是否变化？叙事中是否明确出现了新的境界名称（如"练气期中期""筑基期初期"）？
   → set_realm (完整境界名称，必须在以下列表中：练气期初期、练气期中期、练气期后期、练气期圆满、筑基期初期、筑基期中期、筑基期后期、筑基期圆满、金丹期初期、金丹期中期、金丹期后期、金丹期圆满、元婴期初期、元婴期中期、元婴期后期、元婴期圆满、化神期初期、化神期中期、化神期后期、化神期圆满)
   如果没有出现新境界名称，填 null

2. 灵力是否变化？叙事中是否有消耗灵力（施法、战斗、御器）、恢复灵力（休息、服药、吸收灵气）的描写？
   → qi_delta (负数表示消耗，正数表示恢复)
   如果没有灵力变化的描写，填 null

3. 灵力是否被设定为某个精确值（如突破后灵力回满）？
   → qi_set (具体数值)
   如果不是精确设定，填 null

4. 灵力上限是否变化（如突破境界后上限提升）？
   → max_qi_delta
   如果没有，填 null

5. 灵石数量是否变化？叙事中是否有获得灵石、花费灵石的描写？
   → spirit_stones_delta (获得为正，花费为负)
   如果没有，填 null

6. 剑道是否提升？叙事中是否有剑术、剑气、剑法相关的修炼或领悟描写？
   → sword_art_delta (+1~+3)
   如果没有，填 null

7. 术法是否提升？叙事中是否有术法、法术、神通、符箓相关的修炼或领悟描写？
   → spell_art_delta (+1~+3)
   如果没有，填 null

8. 气血是否提升？叙事中是否有肉身、体魄、炼体相关的修炼或强化描写？
   → blood_qi_delta (+1~+3)
   如果没有，填 null

9. 神魂是否提升？叙事中是否有魂魄、元神、精神相关的修炼或强化描写？
    → spirit_soul_delta (+1~+3)
    如果没有，填 null

10. 神识是否提升？叙事中是否有神识、灵觉、感知相关的修炼或突破描写？
    → divine_sense_delta (+1~+3)
    如果没有，填 null

11. 道心是否提升？叙事中是否有道心、心性、意志相关的感悟或突破描写？
    → dao_heart_delta (+1~+3)
    如果没有，填 null

12. 是否习得了新功法？叙事中是否出现了新的功法名称（如师父传授、自行领悟、获得秘籍）？
    → add_techniques: [{{"name":"功法名","tier":"黄阶/玄阶/地阶/天阶","tech_type":"攻击/防御/身法/心法","proficiency":0.1}}]
    注意：如果功法名已存在于"当前已有功法"列表中，不要重复添加，填 []
    如果没有新功法，填 []

13. 是否获得了新物品？叙事中是否出现了新的物品（如捡到、收到、购买、发现）？
    → add_items: [{{"name":"物品名","item_type":"丹药/法器/材料/杂物","quality":"普通/精良/稀有/传说","quantity":数量,"effect":"效果描述"}}]
    注意：如果物品名已存在于"当前已有物品"列表中，填 []
    如果没有新物品，填 []

14. 是否丢弃或失去了物品？
    → remove_items: ["物品名"]
    如果没有，填 []

15. 是否消耗/使用了物品？叙事中是否描写了使用物品（如服药、使用符箓、消耗材料）？
    → consume_items: [{{"name":"物品名（必须精确匹配当前已有物品名）","quantity":消耗数量（未明确则默认1）}}]
    如果没有消耗物品，填 []

16. 当前位置是否变更？叙事中玩家是否移动到了新的地点？
    → set_current_location: "新地点全名"
    如果没有，填 null

17. 是否发现了新地点？玩家是否初次到达一个不在"已探索地点"列表中的地点？
    → new_locations: ["新地点名"]
    注意：如果该地点已在已探索列表中，填 []
    如果没有新地点，填 []

18. 人物好感是否变化？叙事中是否有人物对玩家态度变化的描写（如更加信任、冷淡、尊敬、厌恶等）？
    → relationship_changes: [{{"name":"人物名（必须精确匹配当前已有关系名）","affinity_delta":±数值}}]
    小幅变化填 ±3~±10，明显变化填 ±10~±20
    如果没有好感变化，填 []

19. 已知人物是否获得了新名字？叙事中是否揭示了某个人物的真名（如之前叫"未知壮汉"，现在知道叫"赵铁柱"）？
    → rename_relationships: [{{"old_name":"旧名字（当前关系列表中的名字）","new_name":"新名字"}}]
    注意：不要添加新人物！用这个字段来改名
    如果没有改名，填 []

20. 任务是否有变化？叙事中是否有任务完成、接到新任务的描写？
    → quest_updates: [{{"name":"任务名","status":"active/completed","description":"任务描述"}}]
    如果没有，填 []

21. 是否有重大剧情节点需要标记？
    → add_flag: "flag名（英文小写，用-分隔，如 breakthrough-baset, entered-secret-realm）"
    如果没有，填 null

22. 是否有值得记录的事件？
    → add_event: "简短事件描述（15字以内）"
    如果没有，填 null

══════════════════════════════════════════
输出严格JSON（必须包含以下所有22个字段）:

{{
  "set_realm": null,
  "qi_delta": null,
  "qi_set": null,
  "max_qi_delta": null,
  "spirit_stones_delta": null,
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
  "set_current_location": null,
  "new_locations": [],
  "relationship_changes": [],
  "rename_relationships": [],
  "quest_updates": [],
  "add_flag": null,
  "add_event": null
}}"#,
        recent_context,
        existing_techs.join("、"), 
        format_items_with_qty(state),
        existing_locs.join("、"), existing_quests.join("、"),
        existing_rels.join("、"), existing_flags.join("、"),
        state.realm,
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

// ══════════════════════════════════════════════════════════════════════
// World Generation — from CreationChoices to WorldConfig
// ══════════════════════════════════════════════════════════════════════

/// Build the prompt for LLM world generation from character creation choices.
/// The LLM must output a strict JSON object matching WorldConfig.
pub fn build_world_generation_prompt(choices: &CreationChoices) -> String {
    let player_bg = choices.to_prompt_text();

    format!(
        r#"你是一个修仙世界的创世者。根据以下修士的背景设定，生成一个完整的修仙世界。

{}

══════════════════════════════════════════
你的任务：
1. 修士的宗门已经在背景中选定（{}），你不能更改宗门名称或类型。你必须围绕这个宗门构建世界。
2. 宗门名称由你根据修士的背景、宗门类型和加入缘由来创意命名。名字要有古风，符合修仙世界的命名习惯。
3. 生成以下信息（必须输出严格JSON）：

{{
  "era_name": "时代名称（如：太虚历、洪荒纪、灵潮历等）",
  "era_description": "时代简述（一句话，描述当前修仙世界的大环境）",
  "continent_name": "大陆名称（如：东荒、南域、中土等）",
  "continent_description": "大陆简述（一句话，描述大陆的地理和势力分布）",
  "sect_name": "宗门全名（你根据修士背景创意命名，如：青云宗、太虚剑派、幽冥谷等）",
  "sect_type": "宗门类型（原样保留：{}）",
  "sect_scale": "宗门规模（小型门派/中等门派/大型宗门/超级势力 之一）",
  "sect_description": "宗门简述（2-3句话，描述宗门的历史、特色、地位）",
  "sect_atmosphere": "宗门氛围（一句话描述宗门内部的人际关系和文化）",
  "player_title": "修士在宗门中的职位（如：外门弟子、内门弟子、记名弟子、杂役弟子 等，要与修士的入道机缘和背景匹配）",
  "player_title_description": "职位简述（一句话描述修士当前的处境）",
  "starting_location_name": "起始地点名称（如：青云宗·外门洞府、黑风岭·废弃矿洞 等）",
  "starting_location_description": "起始地点简述（一句话描述环境）",
  "key_npc_name": "关键人物姓名（修士的导师/引路人。如果修士的入道机缘中没有明确导师（如宗门考核、散修收留），且加入缘由不是被引入，则填\"无\"）",
  "key_npc_role": "关键人物身份（如：师尊、引路人、师兄 等。如果key_npc_name为\"无\"，填空字符串）",
  "key_npc_realm": "关键人物境界（如：金丹后期、元婴初期 等。如果key_npc_name为\"无\"，填空字符串）",
  "key_npc_description": "关键人物简述（外貌、性格、与修士的关系。如果key_npc_name为\"无\"，填空字符串）",
  "nearby_threat_name": "附近的主要威胁名称（如：黑风岭散修、噬灵兽、夺宝修士 等）",
  "nearby_threat_description": "威胁描述（一句话）",
  "world_hook": "世界钩子（一句话暗示当前世界的暗流或即将发生的大事，作为剧情推动力）"
}}

══════════════════════════════════════════
叙事风格: {}（{}性修士视角）
重要：
- 宗门必须与修士背景中的宗门类型（{}）完全一致
- 如果修士的入道机缘是"仙师路过"或"为报恩情，被恩人引入"，则key_npc_name不能为"无"，必须生成一位导师
- 如果修士是散修独行（无宗门），起始地点应当是野外或临时落脚点，而非宗门洞府
- JSON必须严格有效，不要添加注释或额外的文字说明
- 所有字段都必须填写，字符串字段不能为空（除非明确允许为空的描述）"#,
        player_bg,
        choices.sect_category,
        choices.sect_category,
        if choices.narrative_style == "female" { "女频" } else { "男频" },
        if choices.narrative_style == "female" { "女" } else { "男" },
        choices.sect_category,
    )
}

/// Build the opening narrative prompt for the first game turn.
/// Uses WorldConfig and player info to generate the opening scene.
pub fn build_opening_prompt(
    world_config: &WorldConfig,
    player_name: &str,
    dao_name: &str,
    family_bg: &str,
    entry_method: &str,
) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!("修士姓名: {}\n", player_name));
    if !dao_name.is_empty() {
        prompt.push_str(&format!("道号: {}\n", dao_name));
    }
    prompt.push_str(&format!("家世: {}\n", family_bg));
    prompt.push_str(&format!("入道机缘: {}\n", entry_method));

    prompt.push_str(&format!("\n你叫{}。请根据以下世界设定，生成第一轮叙事作为游戏的开始。\n\n", player_name));
    prompt.push_str(&world_config.to_system_prompt_section());
    prompt.push_str("\n请描写修士苏醒或开始行动的场景，引入关键NPC（如果有），给出当前环境的描写。这是游戏的第一回合，要引人入胜。\n");
    prompt.push_str("\n（必须输出纯 JSON：{\"narrative\":\"...\",\"meta_text\":\"...\",\"options\":[\"...\",\"...\",\"...\",\"...\"]}）");

    prompt
}

/// Parse the LLM's world generation JSON response into a WorldConfig.
/// Merges with choices to fill in non-generated fields.
pub fn parse_world_config_json(json_str: &str, choices: &CreationChoices) -> Option<WorldConfig> {
    let cleaned = json_str
        .trim()
        .trim_start_matches("```json")
        .trim()
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let mut wc: WorldConfig = match serde_json::from_str(cleaned) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[world gen] JSON parse error: {}. Raw (first 500 chars): {}",
                e, &cleaned.chars().take(500).collect::<String>());
            return None;
        }
    };

    // Fill in fields from choices (not generated by LLM)
    wc.narrative_style = choices.narrative_style.clone();
    wc.background_flags = choices.collect_background_flags();
    wc.sect_category = choices.sect_category.clone();
    wc.join_reason = choices.join_reason.clone();
    wc.demonic_stance = choices.demonic_stance.clone();
    wc.personality_archetype = choices.personality_archetype.clone();
    wc.core_value = choices.core_value.clone();
    wc.altruism = choices.altruism.clone();

    Some(wc)
}

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CreationChoices, WorldConfig};

    fn test_choices() -> CreationChoices {
        CreationChoices {
            family_background: "寒门之后".into(),
            childhood_experience: "静心读书".into(),
            sect_category: "仙门正宗".into(),
            join_reason: "仰慕其名，主动拜入".into(),
            entry_method: "宗门大开山门，通过考核入外门".into(),
            demonic_stance: "势不两立，见之必除".into(),
            personality_archetype: "韩立".into(),
            core_value: "长生久视，寿与天齐".into(),
            altruism: "先救人，再取丹。人命关天。".into(),
            dao_quest: "超脱生死，得证长生".into(),
            player_name: "孙若虚".into(),
            dao_name: "".into(),
            narrative_style: "male".into(),
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // build_world_generation_prompt
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_world_gen_prompt_contains_choices() {
        let prompt = build_world_generation_prompt(&test_choices());
        assert!(prompt.contains("孙若虚"));
        assert!(prompt.contains("寒门之后"));
        assert!(prompt.contains("静心读书"));
        assert!(prompt.contains("仙门正宗"));
        assert!(prompt.contains("仰慕其名"));
        assert!(prompt.contains("宗门大开山门"));
        assert!(prompt.contains("韩立式"));
        assert!(prompt.contains("超脱生死"));
    }

    #[test]
    fn test_world_gen_prompt_contains_json_template() {
        let prompt = build_world_generation_prompt(&test_choices());
        assert!(prompt.contains("era_name"));
        assert!(prompt.contains("continent_name"));
        assert!(prompt.contains("sect_name"));
        assert!(prompt.contains("key_npc_name"));
        assert!(prompt.contains("nearby_threat_name"));
        assert!(prompt.contains("world_hook"));
    }

    #[test]
    fn test_world_gen_prompt_has_sect_constraint() {
        let prompt = build_world_generation_prompt(&test_choices());
        // Should tell LLM not to change sect type
        assert!(prompt.contains("仙门正宗"));
    }

    #[test]
    fn test_world_gen_prompt_female_style() {
        let mut c = test_choices();
        c.narrative_style = "female".into();
        let prompt = build_world_generation_prompt(&c);
        assert!(prompt.contains("女频"));
    }

    #[test]
    fn test_world_gen_prompt_rogue_sect() {
        let mut c = test_choices();
        c.sect_category = "散修独行".into();
        let prompt = build_world_generation_prompt(&c);
        assert!(prompt.contains("散修独行"));
    }

    // ═══════════════════════════════════════════════════════════════
    // parse_world_config_json
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_parse_valid_world_config_json() {
        let json = r#"{
            "era_name": "灵潮历",
            "era_description": "灵气复苏的时代",
            "continent_name": "南域",
            "continent_description": "十万大山环绕",
            "sect_name": "太虚剑派",
            "sect_type": "仙门正宗",
            "sect_scale": "中等门派",
            "sect_description": "以剑道闻名天下",
            "sect_atmosphere": "师兄弟团结互助",
            "player_title": "外门弟子",
            "player_title_description": "刚入门的弟子",
            "starting_location_name": "太虚剑派·外门剑庐",
            "starting_location_description": "竹林中的一间茅草屋",
            "key_npc_name": "剑无尘",
            "key_npc_role": "师尊",
            "key_npc_realm": "金丹后期",
            "key_npc_description": "太虚剑派传功长老",
            "nearby_threat_name": "噬灵兽",
            "nearby_threat_description": "潜伏在剑派后山",
            "world_hook": "古老的剑冢即将开启"
        }"#;

        let wc = parse_world_config_json(json, &test_choices())
            .expect("should parse valid JSON");
        assert_eq!(wc.era_name, "灵潮历");
        assert_eq!(wc.continent_name, "南域");
        assert_eq!(wc.sect_name, "太虚剑派");
        assert_eq!(wc.key_npc_name, "剑无尘");
        assert_eq!(wc.nearby_threat_name, "噬灵兽");
        assert_eq!(wc.world_hook, "古老的剑冢即将开启");
    }

    #[test]
    fn test_parse_world_config_merges_choices() {
        let json = r#"{
            "era_name": "洪荒纪",
            "era_description": "上古洪荒",
            "continent_name": "北荒",
            "continent_description": "冰原万里",
            "sect_name": "青云宗",
            "sect_type": "仙门正宗",
            "sect_scale": "大型宗门",
            "sect_description": "正道领袖",
            "sect_atmosphere": "等级森严",
            "player_title": "内门弟子",
            "player_title_description": "被选入内门",
            "starting_location_name": "青云宗·内门",
            "starting_location_description": "灵气充沛的修炼室",
            "key_npc_name": "无",
            "key_npc_role": "",
            "key_npc_realm": "",
            "key_npc_description": "",
            "nearby_threat_name": "天魔教",
            "nearby_threat_description": "魔道巨擘",
            "world_hook": "正邪大战一触即发"
        }"#;

        let wc = parse_world_config_json(json, &test_choices())
            .expect("should parse");
        // LLM-generated fields
        assert_eq!(wc.era_name, "洪荒纪");
        // Merged from choices (not LLM-generated)
        assert_eq!(wc.narrative_style, "male");
        assert_eq!(wc.sect_category, "仙门正宗");
        assert_eq!(wc.demonic_stance, "势不两立，见之必除");
        assert_eq!(wc.personality_archetype, "韩立");
        assert!(wc.background_flags.contains(&"humble-origin".into()));
    }

    #[test]
    fn test_parse_malformed_json_returns_none() {
        assert!(parse_world_config_json("not json at all", &test_choices()).is_none());
        assert!(parse_world_config_json("{broken", &test_choices()).is_none());
    }

    #[test]
    fn test_parse_json_with_markdown_fences() {
        // LLM often wraps JSON in ``` fences
        let json = "```json\n{\"era_name\":\"太虚历\",\"era_description\":\"test\",\"continent_name\":\"东荒\",\"continent_description\":\"test\",\"sect_name\":\"青云宗\",\"sect_type\":\"仙门正宗\",\"sect_scale\":\"中等\",\"sect_description\":\"test\",\"sect_atmosphere\":\"test\",\"player_title\":\"外门弟子\",\"player_title_description\":\"test\",\"starting_location_name\":\"青云宗\",\"starting_location_description\":\"test\",\"key_npc_name\":\"清虚\",\"key_npc_role\":\"师尊\",\"key_npc_realm\":\"元婴\",\"key_npc_description\":\"test\",\"nearby_threat_name\":\"test\",\"nearby_threat_description\":\"test\",\"world_hook\":\"test\"}\n```";

        let wc = parse_world_config_json(json, &test_choices());
        assert!(wc.is_some(), "should handle markdown fences");
        assert_eq!(wc.unwrap().era_name, "太虚历");
    }

    #[test]
    fn test_parse_partial_json_applies_defaults() {
        // Missing fields should be empty strings (handled by serde defaults)
        let json = r#"{"era_name": "简朴纪"}"#;
        let wc = parse_world_config_json(json, &test_choices());
        assert!(wc.is_some());
        let wc = wc.unwrap();
        assert_eq!(wc.era_name, "简朴纪");
        assert_eq!(wc.continent_name, ""); // serde default
        assert_eq!(wc.narrative_style, "male"); // merged from choices
    }

    // ═══════════════════════════════════════════════════════════════
    // build_opening_prompt
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_opening_prompt_contains_player_info() {
        let wc = WorldConfig::default();
        let prompt = build_opening_prompt(
            &wc, "孙若虚", "", "寒门之后",
            "宗门大开山门，通过考核入外门",
        );
        assert!(prompt.contains("孙若虚"));
        assert!(prompt.contains("寒门之后"));
        assert!(prompt.contains("宗门"));
        assert!(prompt.contains("纯 JSON"));
        assert!(prompt.contains("\"narrative\""));
        assert!(prompt.contains("\"options\""));
    }

    #[test]
    fn test_opening_prompt_has_dao_name() {
        let wc = WorldConfig::default();
        let prompt = build_opening_prompt(
            &wc, "柳青鸾", "慈心", "医道传家",
            "仙师路过，被收为记名弟子",
        );
        assert!(prompt.contains("柳青鸾"));
        assert!(prompt.contains("慈心"));
    }
}
