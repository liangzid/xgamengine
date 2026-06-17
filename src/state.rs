use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Player combat attributes (六维 — xianxia style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub physical_attack: i32,   // 物攻
    pub magical_attack: i32,    // 法攻
    pub physical_defense: i32,  // 物防
    pub magical_defense: i32,   // 法防
    pub divine_attack: i32,     // 神识攻
    pub divine_defense: i32,    // 神识防
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            physical_attack: 10,
            magical_attack: 5,
            physical_defense: 8,
            magical_defense: 4,
            divine_attack: 3,
            divine_defense: 6,
        }
    }
}

/// Technique (structured)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technique {
    pub name: String,
    pub tier: String,       // 黄阶/玄阶/地阶/天阶
    pub tech_type: String,  // 攻击/防御/身法/心法
    pub proficiency: f32,   // 0.0 ~ 1.0
}

/// Inventory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub name: String,
    pub item_type: String,  // 丹药/法器/功法/材料/杂物
    pub quality: String,    // 普通/精良/稀有/传说
    pub quantity: i32,
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
        s.push_str(&format!("六维: 物攻{} 法攻{} 物防{} 法防{} 神识攻{} 神识防{}\n",
            self.stats.physical_attack, self.stats.magical_attack,
            self.stats.physical_defense, self.stats.magical_defense,
            self.stats.divine_attack, self.stats.divine_defense));
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
        if let Some(a) = change.physical_attack_delta {
            self.stats.physical_attack = (self.stats.physical_attack + a).max(0);
        }
        if let Some(a) = change.magical_attack_delta {
            self.stats.magical_attack = (self.stats.magical_attack + a).max(0);
        }
        if let Some(d) = change.physical_defense_delta {
            self.stats.physical_defense = (self.stats.physical_defense + d).max(0);
        }
        if let Some(d) = change.magical_defense_delta {
            self.stats.magical_defense = (self.stats.magical_defense + d).max(0);
        }
        if let Some(a) = change.divine_attack_delta {
            self.stats.divine_attack = (self.stats.divine_attack + a).max(0);
        }
        if let Some(d) = change.divine_defense_delta {
            self.stats.divine_defense = (self.stats.divine_defense + d).max(0);
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
    }
}

/// State change extracted from AI narrative (now LLM-powered via JSON)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateChange {
    pub realm_progress: Option<f32>,
    pub qi_delta: Option<i32>,
    pub qi_set: Option<i32>,              // absolute set (for breakthroughs that fully restore)
    pub max_qi_delta: Option<i32>,        // increase max qi
    pub spirit_stones_delta: Option<i32>,
    pub physical_attack_delta: Option<i32>,
    pub magical_attack_delta: Option<i32>,
    pub physical_defense_delta: Option<i32>,
    pub magical_defense_delta: Option<i32>,
    pub divine_attack_delta: Option<i32>,
    pub divine_defense_delta: Option<i32>,
    pub add_techniques: Option<Vec<Technique>>,
    pub add_items: Option<Vec<InventoryItem>>,
    pub remove_items: Option<Vec<String>>,
    pub relationship_changes: Option<Vec<RelationshipChange>>,
    pub new_locations: Option<Vec<String>>,
    pub set_current_location: Option<String>,
    pub quest_updates: Option<Vec<QuestUpdate>>,
    pub add_flag: Option<String>,
    pub add_event: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipChange {
    pub name: String,
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
