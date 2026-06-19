use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Player combat attributes (六维 — xianxia style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub sword_art: i32,     // 剑道
    pub spell_art: i32,     // 术法
    pub blood_qi: i32,      // 气血
    pub spirit_soul: i32,   // 神魂
    pub divine_sense: i32,  // 神识
    pub dao_heart: i32,     // 道心
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            sword_art: 10,
            spell_art: 5,
            blood_qi: 8,
            spirit_soul: 4,
            divine_sense: 3,
            dao_heart: 6,
        }
    }
}

/// Technique (structured)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technique {
    pub name: String,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub tech_type: String,
    #[serde(default)]
    pub proficiency: f32,
}

/// Inventory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub name: String,
    #[serde(default)]
    pub item_type: String,
    #[serde(default)]
    pub quality: String,
    #[serde(default)]
    pub quantity: i32,
    #[serde(default)]
    pub effect: String,
}

/// Relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub name: String,
    pub role: String,
    pub affinity: i32,
}

/// Quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub name: String,
    pub status: QuestStatus,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuestStatus {
    Active,
    Completed,
}

/// Core game state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: String,
    pub scenario: String,
    pub round: i32,

    // Cultivation
    pub realm: String,
    pub realm_progress: f32,
    pub qi: i32,
    pub max_qi: i32,

    // Combat
    pub stats: PlayerStats,

    // Techniques & Items
    pub techniques: Vec<Technique>,
    pub inventory: Vec<InventoryItem>,
    pub spirit_stones: i32,

    // Exploration
    pub locations: Vec<String>,
    pub current_location: String,

    // Quests
    pub quests: Vec<Quest>,

    // Social
    pub sect: String,
    pub relationships: Vec<Relationship>,
    /// Player's accumulated knowledge about each character (name → description)
    #[serde(default)]
    pub character_notes: std::collections::HashMap<String, String>,

    // Narrative
    pub flags: Vec<String>,
    pub recent_events: Vec<String>,
    pub last_narrative: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            id: format!("qingyun-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            scenario: "qingyun".into(),
            round: 0,
            realm: "练气期初期".into(),
            realm_progress: 0.0,
            qi: 100,
            max_qi: 100,
            stats: PlayerStats::default(),
            techniques: vec![
                Technique {
                    name: "青云吐纳术".into(),
                    tier: "黄阶".into(),
                    tech_type: "心法".into(),
                    proficiency: 0.3,
                }
            ],
            inventory: vec![
                InventoryItem {
                    name: "凝脉丹".into(), item_type: "丹药".into(),
                    quality: "普通".into(), quantity: 3,
                    effect: "稳固经脉，小幅提升修炼效率".into(),
                },
                InventoryItem {
                    name: "凡铁剑".into(), item_type: "法器".into(),
                    quality: "普通".into(), quantity: 1,
                    effect: "无特殊效果".into(),
                },
            ],
            spirit_stones: 50,
            locations: vec!["青云宗·外门洞府".into(), "青云宗·传功殿".into()],
            current_location: "青云宗·外门洞府".into(),
            quests: vec![],
            sect: "青云宗".into(),
            relationships: vec![
                Relationship { name: "清虚道人".into(), role: "师尊".into(), affinity: 20 }
            ],
            character_notes: {
                let mut m = HashMap::new();
                m.insert("清虚道人".into(), "青云宗传功长老，元婴后期大修士，外表四十余岁，面容清瘦，双目如电。表面冷淡，内心护短。".into());
                m
            },
            flags: vec!["game-started".into()],
            recent_events: vec![],
            last_narrative: String::new(),
        }
    }
}

impl GameState {
    /// Serialize to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Convert to narrative text for LLM prompt injection
    pub fn to_narrative(&self) -> String {
        let mut s = String::new();
        s.push_str("【角色状态】\n");
        s.push_str(&format!("修炼境界: {}\n", self.realm));
        s.push_str(&format!("修炼进度: {:.0}%\n", self.realm_progress * 100.0));
        s.push_str(&format!("灵力: {}/{}\n", self.qi, self.max_qi));
        s.push_str(&format!("六维: 剑道{} 术法{} 气血{} 神魂{} 神识{} 道心{}\n",
            self.stats.sword_art, self.stats.spell_art,
            self.stats.blood_qi, self.stats.spirit_soul,
            self.stats.divine_sense, self.stats.dao_heart));
        s.push_str(&format!("灵石: {}\n", self.spirit_stones));
        s.push_str(&format!("宗门: {}\n", self.sect));
        s.push_str(&format!("当前地点: {}\n", self.current_location));

        if !self.techniques.is_empty() {
            s.push_str("功法:\n");
            for t in &self.techniques {
                s.push_str(&format!("  - {} ({} {}, 熟练:{:.0}%)\n",
                    t.name, t.tier, t.tech_type, t.proficiency * 100.0));
            }
        }
        if !self.inventory.is_empty() {
            s.push_str("物品:\n");
            for item in &self.inventory {
                s.push_str(&format!("  - {} ({} {}) x{}: {}\n",
                    item.name, item.quality, item.item_type, item.quantity, item.effect));
            }
        }
        if !self.locations.is_empty() {
            s.push_str(&format!("已探索地点: {}\n", self.locations.join("、")));
        }
        if !self.quests.is_empty() {
            s.push_str("当前任务:\n");
            for q in &self.quests {
                let status = if q.status == QuestStatus::Active { "进行中" } else { "已完成" };
                s.push_str(&format!("  - [{}] {}: {}\n", status, q.name, q.description));
            }
        }
        if !self.relationships.is_empty() {
            s.push_str("人物关系:\n");
            for r in &self.relationships {
                s.push_str(&format!("  - {} ({}) 好感: {}\n", r.name, r.role, r.affinity));
            }
        }
        if !self.recent_events.is_empty() {
            s.push_str("近期事件:\n");
            for e in &self.recent_events {
                s.push_str(&format!("  - {}\n", e));
            }
        }
        s
    }

    pub fn apply_state_change(&mut self, change: &StateChange) {
        if let Some(rp) = change.realm_progress {
            self.realm_progress = (self.realm_progress + rp).clamp(0.0, 1.0);
        }
        // Handle explicit realm name change (LLM-detected)
        if let Some(ref new_realm) = change.set_realm {
            if REALM_ORDER.contains(&new_realm.as_str()) {
                self.realm = new_realm.clone();
                self.realm_progress = 0.0;
            }
        }
        // Auto-advance realm when progress reaches 1.0 (breakthrough)
        if self.realm_progress >= 1.0 {
            if let Some(pos) = REALM_ORDER.iter().position(|&r| r == self.realm.as_str()) {
                if let Some(&next_realm) = REALM_ORDER.get(pos + 1) {
                    self.realm = next_realm.to_string();
                    self.realm_progress = 0.0;
                    // Full restore on breakthrough
                    self.qi = self.max_qi;
                }
            }
        }
        if let Some(qd) = change.qi_delta {
            self.qi = (self.qi + qd).clamp(0, self.max_qi);
        }
        if let Some(qs) = change.qi_set {
            self.qi = qs.clamp(0, self.max_qi);
        }
        if let Some(mq) = change.max_qi_delta {
            self.max_qi = (self.max_qi + mq).max(10);
            self.qi = self.qi.min(self.max_qi);
        }
        if let Some(ssd) = change.spirit_stones_delta {
            self.spirit_stones = (self.spirit_stones + ssd).max(0);
        }
        if let Some(a) = change.sword_art_delta {
            self.stats.sword_art = (self.stats.sword_art + a).max(0);
        }
        if let Some(a) = change.spell_art_delta {
            self.stats.spell_art = (self.stats.spell_art + a).max(0);
        }
        if let Some(d) = change.blood_qi_delta {
            self.stats.blood_qi = (self.stats.blood_qi + d).max(0);
        }
        if let Some(d) = change.spirit_soul_delta {
            self.stats.spirit_soul = (self.stats.spirit_soul + d).max(0);
        }
        if let Some(a) = change.divine_sense_delta {
            self.stats.divine_sense = (self.stats.divine_sense + a).max(0);
        }
        if let Some(d) = change.dao_heart_delta {
            self.stats.dao_heart = (self.stats.dao_heart + d).max(0);
        }
        if let Some(ref techs) = change.add_techniques {
            for t in techs {
                self.techniques.push(t.clone());
            }
        }
        if let Some(ref items) = change.add_items {
            for item in items {
                // Merge with existing if same name + type
                if let Some(existing) = self.inventory.iter_mut()
                    .find(|i| i.name == item.name && i.item_type == item.item_type) {
                    existing.quantity += item.quantity;
                } else {
                    self.inventory.push(item.clone());
                }
            }
        }
        if let Some(ref remove) = change.remove_items {
            for name in remove {
                self.inventory.retain(|i| i.name != *name);
            }
        }
        if let Some(ref rel_changes) = change.relationship_changes {
            for rc in rel_changes {
                if rc.name.is_empty() { continue; }
                if let Some(rel) = self.relationships.iter_mut()
                    .find(|r| r.name == rc.name) {
                    rel.affinity = (rel.affinity + rc.affinity_delta).clamp(-100, 100);
                    if let Some(ref new_role) = rc.new_role {
                        rel.role = new_role.clone();
                    }
                } else {
                    self.relationships.push(Relationship {
                        name: rc.name.clone(),
                        role: rc.new_role.clone().unwrap_or_else(|| "未知".into()),
                        affinity: rc.affinity_delta.clamp(-100, 100),
                    });
                }
            }
        }
        if let Some(ref locs) = change.new_locations {
            for loc in locs {
                if !self.locations.contains(loc) {
                    self.locations.push(loc.clone());
                }
            }
        }
        if let Some(ref loc) = change.set_current_location {
            self.current_location = loc.clone();
            if !self.locations.contains(loc) {
                self.locations.push(loc.clone());
            }
        }
        if let Some(ref quests) = change.quest_updates {
            for qu in quests {
                if let Some(q) = self.quests.iter_mut().find(|q| q.name == qu.name) {
                    if let Some(ref s) = qu.status {
                        q.status = match s.as_str() {
                            "completed" => QuestStatus::Completed,
                            _ => QuestStatus::Active,
                        };
                    }
                    if let Some(ref d) = qu.description {
                        q.description = d.clone();
                    }
                } else {
                    self.quests.push(Quest {
                        name: qu.name.clone(),
                        status: match qu.status.as_deref() {
                            Some("completed") => QuestStatus::Completed,
                            _ => QuestStatus::Active,
                        },
                        description: qu.description.clone().unwrap_or_default(),
                    });
                }
            }
        }
        if let Some(ref flag) = change.add_flag {
            if !self.flags.contains(flag) {
                self.flags.push(flag.clone());
            }
        }
        if let Some(ref event) = change.add_event {
            self.recent_events.push(event.clone());
            if self.recent_events.len() > 10 {
                self.recent_events.remove(0);
            }
        }
        // Consume items: reduce quantity, remove if ≤ 0
        if let Some(ref consumes) = change.consume_items {
            for c in consumes {
                if c.name.is_empty() { continue; }
                if let Some(item) = self.inventory.iter_mut().find(|i| i.name == c.name) {
                    item.quantity = (item.quantity - c.quantity).max(0);
                }
            }
            self.inventory.retain(|i| i.quantity > 0);
        }
        // Rename relationships: update character name
        if let Some(ref renames) = change.rename_relationships {
            for r in renames {
                if r.old_name.is_empty() || r.new_name.is_empty() { continue; }
                if let Some(rel) = self.relationships.iter_mut().find(|x| x.name == r.old_name) {
                    rel.name = r.new_name.clone();
                }
                // Also update character_notes
                if let Some(notes) = self.character_notes.remove(&r.old_name) {
                    self.character_notes.insert(r.new_name.clone(), notes);
                }
            }
        }
    }
}

/// State change extracted from AI narrative (now LLM-powered via JSON)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateChange {
    pub realm_progress: Option<f32>,
    pub set_realm: Option<String>,          // explicit realm name change (e.g. "练气期中期")
    pub qi_delta: Option<i32>,
    pub qi_set: Option<i32>,              // absolute set (for breakthroughs that fully restore)
    pub max_qi_delta: Option<i32>,        // increase max qi
    pub spirit_stones_delta: Option<i32>,
    pub sword_art_delta: Option<i32>,
    pub spell_art_delta: Option<i32>,
    pub blood_qi_delta: Option<i32>,
    pub spirit_soul_delta: Option<i32>,
    pub divine_sense_delta: Option<i32>,
    pub dao_heart_delta: Option<i32>,
    pub add_techniques: Option<Vec<Technique>>,
    pub add_items: Option<Vec<InventoryItem>>,
    pub remove_items: Option<Vec<String>>,
    pub relationship_changes: Option<Vec<RelationshipChange>>,
    pub new_locations: Option<Vec<String>>,
    pub set_current_location: Option<String>,
    pub quest_updates: Option<Vec<QuestUpdate>>,
    pub add_flag: Option<String>,
    pub add_event: Option<String>,
    /// Consume (reduce quantity of) existing inventory items
    #[serde(default)]
    pub consume_items: Option<Vec<ConsumeItem>>,
    /// Rename an existing relationship (e.g. "未知壮汉" → "张三")
    #[serde(default)]
    pub rename_relationships: Option<Vec<RenameRelationship>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumeItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameRelationship {
    #[serde(default)]
    pub old_name: String,
    #[serde(default)]
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChange {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub affinity_delta: i32,
    pub new_role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestUpdate {
    pub name: String,
    pub status: Option<String>,  // "active" or "completed"
    pub description: Option<String>,
}

/// The ordered cultivation realm list
pub const REALM_ORDER: &[&str] = &[
    "练气期初期", "练气期中期", "练气期后期", "练气期圆满",
    "筑基期初期", "筑基期中期", "筑基期后期", "筑基期圆满",
    "金丹期初期", "金丹期中期", "金丹期后期", "金丹期圆满",
    "元婴期初期", "元婴期中期", "元婴期后期", "元婴期圆满",
    "化神期初期", "化神期中期", "化神期后期", "化神期圆满",
];
