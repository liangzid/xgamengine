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
/// ULTRA-EXPLICIT: forces the LLM to check every single state field methodically.
/// `recent_context` should be structured turn log for entity identity tracking.
pub fn build_state_extraction_prompt(state: &crate::state::GameState, narrative: &str, recent_context: &str) -> String {
    let existing_techs: Vec<String> = state.techniques.iter().map(|t| t.name.clone()).collect();
    let existing_flags: Vec<String> = state.flags.clone();
    let existing_locs: Vec<String> = state.locations.clone();
    let existing_quests: Vec<String> = state.quests.iter().map(|q| q.name.clone()).collect();
    let existing_rels: Vec<String> = state.relationships.iter().map(|r| r.name.clone()).collect();

    format!(
        r#"你是一个修仙游戏状态解析器。你的任务是逐项检查本回合叙事中玩家的状态是否发生变化。
你必须按以下清单，逐条检查，每一条都必须给出明确的答案。不要跳过任何一条。

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
- 境界: {} (进度 {:.0}%)
- 灵力: {}/{}
- 六维: 剑道{} 术法{} 气血{} 神魂{} 神识{} 道心{}
- 灵石: {}
- 当前地点: {}

══════════════════════════════════════════
本回合叙事（玩家行动后发生的事）:
{}

══════════════════════════════════════════
请逐项检查以下所有状态是否在本回合叙事中发生了变化。
对每一项，如果叙事中明确发生了该变化，则填写具体数值；如果没有发生，则填 null 或空数组。

【修炼状态】
1. realm_progress (修炼进度变化): 
   - 关键词: "修为精进""功力增长""突破""瓶颈松动""灵力提升""经脉稳固"
   - 小幅进步填 0.02~0.05，显著进步填 0.08~0.15，突破填 1.0
   - 如果没有修炼相关描写，填 null

2. qi_delta (灵力消耗或恢复): 
   - 关键词: "灵力消耗""真气亏损""元气大伤""灵力枯竭" → 负数 (-10~-50)
   - 关键词: "灵力恢复""真气充盈""服下丹药恢复""灵气入体" → 正数 (+10~+50)
   - 如果没有灵力变化描写，填 null

3. qi_set (灵力直接设置为某个值): 
   - 突破后灵力回满: 填 max_qi 的值
   - 特殊事件导致灵力精确变化: 填具体数值
   - 一般情况填 null

4. max_qi_delta (灵力上限变化): 
   - 突破境界后上限提升: 填正数
   - 一般情况填 null

5. spirit_stones_delta (灵石变化): 
   - 关键词: "获得XX灵石""收入XX灵石""赚取XX灵石" → 正数
   - 关键词: "花费XX灵石""支付XX灵石""消耗XX灵石" → 负数
   - 如果没有提及灵石，填 null

【六维属性】
对每一个六维属性（剑道、术法、气血、神魂、神识、道心），检查叙事中是否有明确提到其提升或下降：
6. sword_art_delta: 关键词 "剑道""剑术""剑法""剑气" 有提升 → +1~+3
7. spell_art_delta: 关键词 "术法""法术""神通""符箓" 有提升 → +1~+3
8. blood_qi_delta: 关键词 "气血""肉身""体魄""炼体" 有提升 → +1~+3
9. spirit_soul_delta: 关键词 "神魂""魂魄""元神""精神" 有提升 → +1~+3
10. divine_sense_delta: 关键词 "神识""灵觉""感知""天眼" 有提升 → +1~+3
11. dao_heart_delta: 关键词 "道心""心性""意志""定力" 有提升 → +1~+3
如果没有对应描写，填 null

【功法和物品】
12. add_techniques (新习得的功法): 
    关键词: "习得""学会""领悟""掌握""参悟""顿悟"
    格式: [{{"name":"功法名","tier":"黄阶/玄阶/地阶/天阶","tech_type":"攻击/防御/身法/心法","proficiency":0.1}}]
    如果功法名已存在于"当前已有功法"列表中，不要重复添加，填 []
    叙事中未出现，填 []

13. add_items (新获得的物品): 
    关键词: "获得""捡到""发现""入手""收到""赠与""购买""拾取"
    格式: [{{"name":"物品名","item_type":"丹药/法器/材料/杂物","quality":"普通/精良/稀有/传说","quantity":数量,"effect":"效果描述"}}]
    如果物品名已存在于"当前已有物品"列表中，不要重复添加，填 []
    叙事中未出现，填 []

14. remove_items (丢弃或失去的物品): 
    关键词: "丢弃""毁坏""遗失""被夺""损坏"
    格式: ["物品名1","物品名2"]
    叙事中未出现，填 []

15. consume_items (消耗/使用的物品): 
    关键词: "服下""服用""吞下""吃下""饮下""使用""捏碎""燃烧"
    格式: [{{"name":"物品名（必须精确匹配已有物品名）","quantity":消耗数量}}]
    系统会自动从库存中扣除。如果消耗数量未明确，默认填 1
    注意：消耗的物品必须在"当前已有物品"中存在，否则忽略
    叙事中未出现，填 []

【地点】
16. set_current_location (当前位置变更): 
    关键词: "前往""来到""进入""抵达""返回""回到" + 地点名
    格式: "新地点全名"
    叙事中未出现地点变更，填 null

17. new_locations (新发现的未知地点): 
    仅当玩家初次到达一个不在"已探索地点"列表中的地点时填写
    格式: ["新地点名"]
    如果该地点已在已探索列表中，不要重复添加，填 []
    叙事中未出现新地点，填 []

【人物关系】
18. relationship_changes (人物好感变化): 
    关键词: "好感大增/更加信任/亲近/尊敬/崇拜" → affinity_delta 填 +10~+20
    关键词: "好感提升/略有改观/态度缓和" → affinity_delta 填 +3~+10
    关键词: "好感下降/冷淡/疏远/不满/厌恶/敌视/愤怒" → affinity_delta 填 -5~-20
    格式: [{{"name":"人物名（必须精确匹配已有关系名）","affinity_delta":±数值,"new_role":"新身份（可选）"}}]
    如果人物不在已有关系列表中：不要添加新人物！这时可能应该用"最近对话"来判断此人是否已存在但用别名
    叙事中未出现好感变化，填 []

19. rename_relationships (已知人物获得新名字): 
    关键词: "自称""原来叫""真名""名为""叫做""本名""姓"
    格式: [{{"old_name":"旧名字（当前关系列表中的名字）","new_name":"新名字"}}]
    关键规则: 不要添加新人物！如果叙事中出现了已有角色的新名字，用这个字段来改名
    例如：之前认识"受伤的壮汉"，现在得知他叫"赵铁柱" → old_name:"受伤的壮汉", new_name:"赵铁柱"
    叙事中未出现改名，填 []

【任务】
20. quest_updates (任务进度变化): 
    关键词: "任务完成""委托达成""目标达成" → status: "completed"
    关键词: "接到新任务""接受委托""新的使命" → status: "active" 且提供 description
    格式: [{{"name":"任务名","status":"active/completed","description":"任务描述"}}]
    叙事中未出现任务变化，填 []

【Flag和事件】
21. add_flag (新标记): 
    关键词: 重大剧情节点，如突破大境界、加入宗门、发现秘境、击败强敌等
    格式: "具体flag名称（英文小写，用-分隔）"
    如果flag已存在于已有flag列表中，不要重复添加，填 null
    叙事中未出现重大节点，填 null

22. add_event (新事件记录): 
    叙事中值得记录的简短事件摘要（15字以内）
    格式: "事件描述"
    叙事中无特别事件，填 null

══════════════════════════════════════════
输出严格JSON（必须包含以下所有22个字段，每个字段都必须出现）:

{{
  "realm_progress": null,
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
}}

══════════════════════════════════════════
以下是一个正确输出的示例:

叙事: "你从怀中取出凝脉丹服下，药力化开，经脉稳固了几分。随后你离开洞府，前往传功殿向清虚道人请教。清虚道人微微点头：'不错，你的吐纳术已入门。'你感到道心更加稳固。"
已有物品: 凝脉丹 x3
已有的功法: 青云吐纳术
已有人物: 清虚道人 (师尊, 好感20)
已知地点: 青云宗·外门洞府

正确输出:
{{
  "realm_progress": 0.03,
  "qi_delta": null,
  "qi_set": null,
  "max_qi_delta": null,
  "spirit_stones_delta": null,
  "sword_art_delta": null,
  "spell_art_delta": null,
  "blood_qi_delta": null,
  "spirit_soul_delta": null,
  "divine_sense_delta": null,
  "dao_heart_delta": 1,
  "add_techniques": [],
  "add_items": [],
  "remove_items": [],
  "consume_items": [{{"name":"凝脉丹","quantity":1}}],
  "set_current_location": "青云宗·传功殿",
  "new_locations": [],
  "relationship_changes": [{{"name":"清虚道人","affinity_delta":5}}],
  "rename_relationships": [],
  "quest_updates": [],
  "add_flag": null,
  "add_event": null
}}

错误示例（不要这样）:
- 把"吐纳术已入门"当成新功法 → 错！它已在功法列表中
- 因为清虚道人说了话就加好感 → 只在叙事明确表明好感变化时才加
- 忽略灵力消耗 → 如果叙事中描写了战斗或施法，必须检查 qi_delta"#,
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
