use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════
// Character Creation — raw Q1-Q7 choices from the client
// ══════════════════════════════════════════════════════════════════════

/// Raw answers from the 7-question character creation wizard.
/// Q3 is split into Q3a (sect category) and Q3b (join reason).
/// Q5 is split into Q5a-Q5d.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationChoices {
    /// Q1: 家世出身 — family background (e.g. "寒门之后", "修真世家")
    pub family_background: String,
    /// Q2: 少年经历 — childhood experience (e.g. "静心读书", "习武强身")
    pub childhood_experience: String,
    /// Q3a: 宗门选择 — sect category (e.g. "仙门正宗", "魔道宗门")
    pub sect_category: String,
    /// Q3b: 加入缘由 — why you joined (e.g. "仰慕其名，主动拜入")
    pub join_reason: String,
    /// Q4: 入道机缘 — how you entered cultivation (e.g. "仙师路过，被收为记名弟子")
    pub entry_method: String,
    /// Q5a: 对魔道修士的态度
    pub demonic_stance: String,
    /// Q5b: 行事风格最接近的修士 (e.g. "韩立", "王林")
    pub personality_archetype: String,
    /// Q5c: 修仙之路上最重要的是什么
    pub core_value: String,
    /// Q5d: 洞府场景选择
    pub altruism: String,
    /// Q6: 问道之志 — why you seek the Dao
    pub dao_quest: String,
    /// Q7: 姓名
    pub player_name: String,
    /// Q7: 道号 (optional)
    pub dao_name: String,
    /// Gender → narrative style: "male" = 男频, "female" = 女频
    pub narrative_style: String,
}

impl CreationChoices {
    /// Calculate initial player stats from all Q1-Q7 choices.
    /// Returns the starting PlayerStats based on the design doc's stat tables.
    pub fn calculate_initial_stats(&self) -> PlayerStats {
        let mut stats = PlayerStats::default();

        // Q1: 家世出身
        match self.family_background.as_str() {
            "寒门之后" => { stats.sword_art = 8; stats.spell_art = 6; stats.blood_qi = 6; stats.spirit_soul = 5; stats.divine_sense = 4; stats.dao_heart = 8; }
            "修真世家" => { stats.sword_art = 11; stats.spell_art = 7; stats.blood_qi = 7; stats.spirit_soul = 5; stats.divine_sense = 4; stats.dao_heart = 4; }
            "山野遗孤" => { stats.sword_art = 7; stats.spell_art = 5; stats.blood_qi = 10; stats.spirit_soul = 6; stats.divine_sense = 5; stats.dao_heart = 7; }
            "商贾之家" => { stats.sword_art = 7; stats.spell_art = 7; stats.blood_qi = 6; stats.spirit_soul = 5; stats.divine_sense = 4; stats.dao_heart = 5; }
            "书香门第" => { stats.sword_art = 6; stats.spell_art = 8; stats.blood_qi = 5; stats.spirit_soul = 7; stats.divine_sense = 8; stats.dao_heart = 7; }
            "将门虎子" => { stats.sword_art = 12; stats.spell_art = 5; stats.blood_qi = 8; stats.spirit_soul = 4; stats.divine_sense = 3; stats.dao_heart = 5; }
            "医道传家" => { stats.sword_art = 5; stats.spell_art = 9; stats.blood_qi = 5; stats.spirit_soul = 6; stats.divine_sense = 7; stats.dao_heart = 7; }
            "乞儿流浪" => { stats.sword_art = 6; stats.spell_art = 4; stats.blood_qi = 9; stats.spirit_soul = 4; stats.divine_sense = 7; stats.dao_heart = 8; }
            _ => {}
        }

        // Q2: 少年经历
        match self.childhood_experience.as_str() {
            "静心读书" => { stats.divine_sense += 2; stats.dao_heart += 1; }
            "习武强身" => { stats.sword_art += 1; stats.blood_qi += 2; }
            "市井谋生" => { stats.spirit_soul += 1; /* +15 灵石 handled separately */ }
            "山林探险" => { stats.divine_sense += 1; stats.blood_qi += 1; stats.spell_art += 1; }
            "拜入道观" => { stats.spell_art += 2; stats.dao_heart += 2; }
            "随商队游历" => { stats.spirit_soul += 2; stats.divine_sense += 1; }
            "服侍贵人" => { stats.dao_heart += 2; stats.spirit_soul += 1; }
            "独自修炼" => { stats.spell_art += 1; stats.dao_heart += 1; stats.divine_sense += 1; }
            _ => {}
        }

        // Q3a: 宗门选择
        match self.sect_category.as_str() {
            "仙门正宗" => { stats.dao_heart += 2; stats.spell_art += 1; }
            "魔道宗门" => { stats.sword_art += 2; stats.spirit_soul += 1; }
            "旁门左道" => { stats.divine_sense += 1; stats.blood_qi += 1; /* +20 灵石 */ }
            "散修联盟" => { stats.spirit_soul += 2; /* +30 灵石 */ }
            "隐世宗门" => { stats.divine_sense += 2; stats.dao_heart += 1; }
            "佛门禅院" => { stats.dao_heart += 3; stats.blood_qi += 1; }
            "散修独行" => { stats.blood_qi += 2; stats.spirit_soul += 2; }
            _ => {}
        }

        // Q3b: 加入缘由
        match self.join_reason.as_str() {
            "仰慕其名，主动拜入" => { stats.dao_heart += 1; stats.divine_sense += 1; }
            "为报恩情，被恩人引入" => { stats.dao_heart += 2; }
            "为避仇家，托庇于此" => { stats.spirit_soul += 2; }
            "家族安排，身不由己" => { stats.spirit_soul += 1; /* +10 灵石 */ }
            "被胁迫加入，身不由己" => { stats.blood_qi += 2; stats.dao_heart += 1; }
            "作为卧底潜入，另有目的" => { stats.divine_sense += 2; stats.spirit_soul += 1; }
            "机缘巧合，误打误撞" => { stats.divine_sense += 1; /* 获得机缘信物 */ }
            "被其收留，入其门下" => { stats.dao_heart += 1; stats.spirit_soul += 1; }
            _ => {}
        }

        // Q4: 入道机缘
        match self.entry_method.as_str() {
            "仙师路过，被收为记名弟子" => { stats.spell_art += 3; stats.divine_sense += 1; }
            "宗门大开山门，通过考核入外门" => { stats.sword_art += 2; stats.dao_heart += 1; }
            "误入上古遗迹，获得传承玉简" => { stats.divine_sense += 3; stats.spirit_soul += 1; }
            "家传秘法，终于入门" => { stats.spell_art += 2; stats.spirit_soul += 2; }
            "落崖奇遇，得遇洞府" => { stats.blood_qi += 3; }
            "散修收留，入门为杂役" => { stats.dao_heart += 3; stats.divine_sense += 1; }
            "被仇家追杀，误入仙门" => { stats.sword_art += 3; stats.spirit_soul += 1; }
            _ => {}
        }

        // Q6: 问道之志
        match self.dao_quest.as_str() {
            "超脱生死，得证长生" => { stats.dao_heart += 4; stats.divine_sense += 1; }
            "复仇雪恨，手刃仇敌" => { stats.sword_art += 3; stats.spirit_soul += 2; }
            "守护苍生，以武止戈" => { stats.blood_qi += 2; stats.dao_heart += 2; }
            "探寻真理，穷究天道" => { stats.divine_sense += 3; stats.spell_art += 2; }
            "重振家声，光耀门楣" => { stats.spirit_soul += 2; stats.dao_heart += 1; /* +20 灵石 */ }
            "只为自由，不受束缚" => { stats.blood_qi += 3; stats.sword_art += 2; }
            "无他，随波逐流" => { stats.dao_heart += 2; stats.divine_sense += 1; stats.spirit_soul += 1; }
            _ => {}
        }

        // Q5: 心性品格 — no stat effects per design doc, only narrative flags
        // (except Q5d: altruism has stat effects)
        match self.altruism.as_str() {
            "先救人，再取丹。人命关天。" => { stats.dao_heart += 2; }
            "取丹离去，心中默念抱歉。筑基丹太珍贵了。" => { stats.divine_sense += 1; /* 获得筑基丹 */ }
            "救醒他，了解情况后再决定。" => { stats.spirit_soul += 1; }
            "救醒他，然后让他拿身上的东西作为报答。" => { stats.dao_heart += 1; /* +50 灵石 */ }
            _ => {}
        }

        stats
    }

    /// Calculate initial spirit stones from all Q1-Q7 choices
    pub fn calculate_initial_spirit_stones(&self) -> i32 {
        let mut stones = 0;

        // Q1: 家世出身
        match self.family_background.as_str() {
            "修真世家" => stones += 30,
            "商贾之家" => stones += 80,
            "书香门第" => stones += 10,
            "将门虎子" => stones += 20,
            "医道传家" => stones += 15,
            _ => {}
        }

        // Q2: 少年经历
        if self.childhood_experience == "市井谋生" { stones += 15; }

        // Q3a: 宗门选择
        match self.sect_category.as_str() {
            "旁门左道" => stones += 20,
            "散修联盟" => stones += 30,
            _ => {}
        }

        // Q3b: 加入缘由
        if self.join_reason == "家族安排，身不由己" { stones += 10; }

        // Q6: 问道之志
        if self.dao_quest == "重振家声，光耀门楣" { stones += 20; }

        // Q5d altruism
        if self.altruism == "救醒他，然后让他拿身上的东西作为报答。" { stones += 50; }

        stones
    }

    /// Calculate initial inventory items from Q1-Q7 choices
    pub fn calculate_initial_items(&self) -> Vec<InventoryItem> {
        let mut items = Vec::new();

        // Q1 items
        match self.family_background.as_str() {
            "山野遗孤" => items.push(InventoryItem {
                name: "疗伤丹".into(), item_type: "丹药".into(), quality: "普通".into(),
                quantity: 1, effect: "治疗轻伤".into(),
            }),
            "书香门第" => items.push(InventoryItem {
                name: "旧书卷".into(), item_type: "杂物".into(), quality: "普通".into(),
                quantity: 1, effect: "记载着残缺的上古文字".into(),
            }),
            "将门虎子" => items.push(InventoryItem {
                name: "铁剑".into(), item_type: "法器".into(), quality: "普通".into(),
                quantity: 1, effect: "凡铁所铸，胜在结实".into(),
            }),
            "医道传家" => items.push(InventoryItem {
                name: "疗伤丹".into(), item_type: "丹药".into(), quality: "普通".into(),
                quantity: 3, effect: "治疗轻伤".into(),
            }),
            _ => {}
        }

        // Q4 items
        match self.entry_method.as_str() {
            "误入上古遗迹，获得传承玉简" => items.push(InventoryItem {
                name: "残破玉简".into(), item_type: "杂物".into(), quality: "稀有".into(),
                quantity: 1, effect: "蕴含上古传承的残缺信息".into(),
            }),
            "落崖奇遇，得遇洞府" => items.push(InventoryItem {
                name: "未知丹药".into(), item_type: "丹药".into(), quality: "精良".into(),
                quantity: 2, effect: "异香扑鼻，药性未知".into(),
            }),
            _ => {}
        }

        // Q3b items
        if self.join_reason == "机缘巧合，误打误撞" {
            items.push(InventoryItem {
                name: "机缘信物".into(), item_type: "杂物".into(), quality: "精良".into(),
                quantity: 1, effect: "散发着微光，似有灵性".into(),
            });
        }

        // Q5d items
        match self.altruism.as_str() {
            "取丹离去，心中默念抱歉。筑基丹太珍贵了。" => items.push(InventoryItem {
                name: "筑基丹".into(), item_type: "丹药".into(), quality: "稀有".into(),
                quantity: 1, effect: "大幅提升筑基成功率".into(),
            }),
            "取走丹药和令牌，放任其自生自灭。" => {
                items.push(InventoryItem {
                    name: "筑基丹".into(), item_type: "丹药".into(), quality: "稀有".into(),
                    quantity: 1, effect: "大幅提升筑基成功率".into(),
                });
                items.push(InventoryItem {
                    name: "内门令牌".into(), item_type: "杂物".into(), quality: "精良".into(),
                    quantity: 1, effect: "某宗门内门弟子的身份令牌".into(),
                });
            }
            _ => {}
        }

        items
    }

    /// Generate all background flags from choices
    pub fn collect_background_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();

        // Q1 flags
        let q1_flag = match self.family_background.as_str() {
            "寒门之后" => "humble-origin",
            "修真世家" => "clan-descendant",
            "山野遗孤" => "wild-orphan",
            "商贾之家" => "merchant-born",
            "书香门第" => "scholar-born",
            "将门虎子" => "general-descendant",
            "医道传家" => "healer-born",
            "乞儿流浪" => "beggar-origin",
            _ => "",
        };
        if !q1_flag.is_empty() { flags.push(q1_flag.into()); }

        // Q2 flags
        let q2_flag = match self.childhood_experience.as_str() {
            "静心读书" => "studious-youth",
            "习武强身" => "martial-youth",
            "市井谋生" => "streetwise-youth",
            "山林探险" => "explorer-youth",
            "拜入道观" => "taoist-youth",
            "随商队游历" => "traveler-youth",
            "服侍贵人" => "servant-youth",
            "独自修炼" => "solitary-youth",
            _ => "",
        };
        if !q2_flag.is_empty() { flags.push(q2_flag.into()); }

        // Q3a flags
        let q3a_flag = match self.sect_category.as_str() {
            "仙门正宗" => "orthodox-sect",
            "魔道宗门" => "demonic-sect",
            "旁门左道" => "unorthodox-sect",
            "散修联盟" => "loose-cultivator",
            "隐世宗门" => "hidden-sect",
            "佛门禅院" => "buddhist-temple",
            "散修独行" => "rogue-cultivator",
            _ => "",
        };
        if !q3a_flag.is_empty() { flags.push(q3a_flag.into()); }

        // Q5 flags
        let q5b_flag = match self.personality_archetype.as_str() {
            "韩立" => "cautious",
            "王林" => "ruthless",
            "徐缺" => "witty",
            "萧炎" => "loyal",
            "白小纯" => "lucky",
            "方平" => "strategic",
            "李长寿" => "hidden",
            _ => "",
        };
        if !q5b_flag.is_empty() { flags.push(q5b_flag.into()); }

        let q5c_flag = match self.core_value.as_str() {
            "长生久视，寿与天齐" => "value-immortality",
            "逍遥自在，不受束缚" => "value-freedom",
            "登临绝顶，俯瞰众生" => "value-power",
            "守护所爱，庇护一方" => "value-protection",
            "探寻真理，穷究天道" => "value-knowledge",
            "快意恩仇，不负此生" => "value-justice",
            _ => "",
        };
        if !q5c_flag.is_empty() { flags.push(q5c_flag.into()); }

        // Q5d altruism flag
        let q5d_flag = match self.altruism.as_str() {
            s if s.starts_with("先救人") => "altruistic",
            s if s.starts_with("取丹离去") => "self-interested",
            s if s.starts_with("救醒他，了解") => "cautiously-kind",
            s if s.starts_with("取走丹药") => "ruthless",
            s if s.starts_with("救醒他，然后") => "pragmatic",
            _ => "",
        };
        if !q5d_flag.is_empty() { flags.push(q5d_flag.into()); }

        flags
    }

    /// Build the text summary of all Q1-Q7 choices for the world-generation prompt.
    pub fn to_prompt_text(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("修士姓名: {}\n", self.player_name));
        if !self.dao_name.is_empty() {
            s.push_str(&format!("道号: {}\n", self.dao_name));
        }
        s.push_str(&format!("修士性别: {}\n", if self.narrative_style == "female" { "女" } else { "男" }));
        s.push_str(&format!("叙事风格: {}\n", if self.narrative_style == "female" { "女频" } else { "男频" }));
        s.push_str("\n## 修士背景\n\n");
        s.push_str(&format!("家世出身: {}\n", self.family_background));
        s.push_str(&format!("少年经历: {}\n", self.childhood_experience));
        s.push_str(&format!("宗门选择: {}\n", self.sect_category));
        s.push_str(&format!("加入缘由: {}\n", self.join_reason));
        s.push_str(&format!("入道机缘: {}\n", self.entry_method));
        s.push_str(&format!("对魔道态度: {}\n", self.demonic_stance));
        s.push_str(&format!("行事风格: {}式\n", self.personality_archetype));
        s.push_str(&format!("核心追求: {}\n", self.core_value));
        s.push_str(&format!("道德场景选择: {}\n", self.altruism));
        s.push_str(&format!("问道之志: {}\n", self.dao_quest));
        s
    }
}

// ══════════════════════════════════════════════════════════════════════
// World Config — LLM-generated world setting
// ══════════════════════════════════════════════════════════════════════

/// World configuration generated by the LLM from CreationChoices.
/// This is stored with the game state and injected into every prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// "male" or "female" — determines narrative style (男频/女频)
    #[serde(default)]
    pub narrative_style: String,

    /// All background flags from Q1-Q7 (for reference)
    #[serde(default)]
    pub background_flags: Vec<String>,

    /// Q3 — sect choice (preserved, not generated)
    #[serde(default)]
    pub sect_category: String,
    #[serde(default)]
    pub join_reason: String,

    /// Q5 — personality
    #[serde(default)]
    pub demonic_stance: String,
    #[serde(default)]
    pub personality_archetype: String,
    #[serde(default)]
    pub core_value: String,
    #[serde(default)]
    pub altruism: String,

    /// LLM-generated world fields
    #[serde(default)]
    pub era_name: String,
    #[serde(default)]
    pub era_description: String,
    #[serde(default)]
    pub continent_name: String,
    #[serde(default)]
    pub continent_description: String,
    #[serde(default)]
    pub sect_name: String,
    #[serde(default)]
    pub sect_type: String,
    #[serde(default)]
    pub sect_scale: String,
    #[serde(default)]
    pub sect_description: String,
    #[serde(default)]
    pub sect_atmosphere: String,
    #[serde(default)]
    pub player_title: String,
    #[serde(default)]
    pub player_title_description: String,
    #[serde(default)]
    pub starting_location_name: String,
    #[serde(default)]
    pub starting_location_description: String,
    /// "无" if no mentor
    #[serde(default)]
    pub key_npc_name: String,
    #[serde(default)]
    pub key_npc_role: String,
    #[serde(default)]
    pub key_npc_realm: String,
    #[serde(default)]
    pub key_npc_description: String,
    #[serde(default)]
    pub nearby_threat_name: String,
    #[serde(default)]
    pub nearby_threat_description: String,
    #[serde(default)]
    pub world_hook: String,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            narrative_style: "male".into(),
            background_flags: vec!["default-world".into()],
            sect_category: "仙门正宗".into(),
            join_reason: "仰慕其名，主动拜入".into(),
            demonic_stance: "势不两立，见之必除".into(),
            personality_archetype: "韩立".into(),
            core_value: "长生久视，寿与天齐".into(),
            altruism: "先救人，再取丹。人命关天。".into(),
            era_name: "太虚历".into(),
            era_description: "太虚历三万年后，天地灵气渐薄，修仙界进入末法时代的前夜。各大宗门明争暗斗，散修艰难求生。".into(),
            continent_name: "东荒".into(),
            continent_description: "东荒大陆，群山环绕，灵脉纵横。大小宗门星罗棋布，妖兽横行于荒野，凡人与修士共处一方天地。".into(),
            sect_name: "青云宗".into(),
            sect_type: "仙门正宗".into(),
            sect_scale: "中等门派".into(),
            sect_description: "青云宗坐落于东荒群山之中，以剑道筑基闻名。宗门传承三千余年，虽非顶尖大派，却也是东荒有头有脸的修仙势力。".into(),
            sect_atmosphere: "师徒情深，团结互助。长老虽严，却真心为弟子着想。".into(),
            player_title: "外门弟子".into(),
            player_title_description: "青云宗外门弟子，刚入门不久，正在熟悉宗门规矩和基础功法。".into(),
            starting_location_name: "青云宗·外门洞府".into(),
            starting_location_description: "一处简陋的石洞，位于青云宗外门区域。洞内只有一张石床、一个蒲团和一盏长明灯。".into(),
            key_npc_name: "清虚道人".into(),
            key_npc_role: "师尊".into(),
            key_npc_realm: "元婴后期".into(),
            key_npc_description: "青云宗传功长老，元婴后期大修士，外表四十余岁，面容清瘦，双目如电。表面冷淡，内心护短。".into(),
            nearby_threat_name: "黑风岭散修".into(),
            nearby_threat_description: "青云宗外围黑风岭中盘踞着一伙散修，时常劫掠落单的外门弟子，是新手修士的首要威胁。".into(),
            world_hook: "东荒大陆看似平静，实则暗流涌动。古老的秘境即将开启，各方势力蠢蠢欲动。而你，一个刚入门的小修士，即将被卷入这场风暴之中。".into(),
        }
    }
}

impl WorldConfig {
    /// Serialize to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Build the system prompt section for world setting
    pub fn to_system_prompt_section(&self) -> String {
        let mut s = String::new();

        // Narrative style directive
        if self.narrative_style == "female" {
            s.push_str("主角设定: 女性修士。\n");
            s.push_str("当前叙事风格: 女频\n");
            s.push_str("女频：侧重人物关系、情感描写、成长蜕变、社会互动、命运纠葛。文风细腻，注重内心戏。\n");
        } else {
            s.push_str("主角设定: 男性修士。\n");
            s.push_str("当前叙事风格: 男频\n");
            s.push_str("男频：侧重力量成长、战斗描写、功法体系、资源竞争、宗门政治。文风爽快，注重升级感。\n");
        }

        s.push_str("\n## 世界设定\n\n");
        s.push_str(&format!("时代: {} — {}\n", self.era_name, self.era_description));
        s.push_str(&format!("大陆: {} — {}\n", self.continent_name, self.continent_description));

        s.push_str("\n## 宗门\n\n");
        s.push_str(&format!("名称: {}\n", self.sect_name));
        s.push_str(&format!("类型: {} ({})\n", self.sect_type, self.sect_scale));
        s.push_str(&format!("描述: {}\n", self.sect_description));
        s.push_str(&format!("氛围: {}\n", self.sect_atmosphere));

        s.push_str("\n## 你的身份\n\n");
        s.push_str(&format!("职位: {}\n", self.player_title));
        s.push_str(&format!("描述: {}\n", self.player_title_description));

        s.push_str("\n## 起始地点\n\n");
        s.push_str(&format!("{} — {}\n", self.starting_location_name, self.starting_location_description));

        // Key NPC
        if self.key_npc_name == "无" || self.key_npc_name.is_empty() {
            s.push_str("\n## 你的处境\n\n");
            s.push_str(&format!("你在{}，目前独自一人。宗门之中，你需要靠自己去结识他人、寻找机缘。\n", self.player_title_description));
        } else {
            s.push_str("\n## 关键人物\n\n");
            s.push_str(&format!("{}是你的{}，{}修士。\n", self.key_npc_name, self.key_npc_role, self.key_npc_realm));
            s.push_str(&format!("{}\n", self.key_npc_description));
        }

        // Nearby threat
        if !self.nearby_threat_name.is_empty() {
            s.push_str(&format!("\n## 附近威胁\n\n{} — {}\n", self.nearby_threat_name, self.nearby_threat_description));
        }

        // World hook
        if !self.world_hook.is_empty() {
            s.push_str(&format!("\n## 世界暗流\n\n{}\n", self.world_hook));
        }

        s
    }
}

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
            inventory: vec![],
            spirit_stones: 0,
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
        // Handle explicit realm name change (LLM-detected)
        if let Some(ref new_realm) = change.set_realm {
            if REALM_ORDER.contains(&new_realm.as_str()) {
                self.realm = new_realm.clone();
                self.realm_progress = 0.0;
                // Full restore on breakthrough to a new major realm
                if self.realm.ends_with("初期") || self.realm.ends_with("中期") || self.realm.ends_with("后期") || self.realm.ends_with("圆满") {
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

// ══════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──

    fn sample_choices() -> CreationChoices {
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

    fn alt_choices() -> CreationChoices {
        CreationChoices {
            family_background: "将门虎子".into(),
            childhood_experience: "习武强身".into(),
            sect_category: "魔道宗门".into(),
            join_reason: "被胁迫加入，身不由己".into(),
            entry_method: "被仇家追杀，误入仙门".into(),
            demonic_stance: "魔道亦有可取之处，手段不重要，结果才重要".into(),
            personality_archetype: "王林".into(),
            core_value: "快意恩仇，不负此生".into(),
            altruism: "取走丹药和令牌，放任其自生自灭。".into(),
            dao_quest: "复仇雪恨，手刃仇敌".into(),
            player_name: "铁剑心".into(),
            dao_name: "杀生".into(),
            narrative_style: "male".into(),
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // CreationChoices — calculate_initial_stats
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_stats_humble_scholar_path() {
        // 寒门之后 + 静心读书 + 仙门正宗 + 仰慕 + 宗门考核 + 超脱生死 + 先救人
        // Full trace done in test_stats_humble_scholar_detailed below
        let stats = sample_choices().calculate_initial_stats();
        assert_eq!(stats.sword_art, 10);
        assert_eq!(stats.spell_art, 7);
        assert_eq!(stats.blood_qi, 6);
        assert_eq!(stats.spirit_soul, 5);
        assert_eq!(stats.divine_sense, 8);
        assert_eq!(stats.dao_heart, 19);
    }

    #[test]
    fn test_stats_general_demonic_path() {
        let stats = alt_choices().calculate_initial_stats();
        // Q1 将门: sword=12, spell=5, blood=8, soul=4, sense=3, heart=5
        // Q2 习武: sword+1, blood+2 → s=13, sp=5, b=10, so=4, se=3, h=5
        // Q3a 魔道: sword+2, soul+1 → s=15, sp=5, b=10, so=5, se=3, h=5
        // Q3b 被胁迫: blood+2, heart+1 → s=15, sp=5, b=12, so=5, se=3, h=6
        // Q4 被追杀: sword+3, soul+1 → s=18, sp=5, b=12, so=6, se=3, h=6
        // Q6 复仇: sword+3, soul+2 → s=21, sp=5, b=12, so=8, se=3, h=6
        // Q5d: no stat effect for "取走丹药和令牌"
        assert_eq!(stats.sword_art, 21);
        assert_eq!(stats.spell_art, 5);
        assert_eq!(stats.blood_qi, 12);
        assert_eq!(stats.spirit_soul, 8);
        assert_eq!(stats.divine_sense, 3);
        assert_eq!(stats.dao_heart, 6);
    }

    #[test]
    fn test_stats_healer_buddhist_path() {
        let choices = CreationChoices {
            family_background: "医道传家".into(),
            childhood_experience: "拜入道观".into(),
            sect_category: "佛门禅院".into(),
            join_reason: "为报恩情，被恩人引入".into(),
            entry_method: "散修收留，入门为杂役".into(),
            demonic_stance: "道不同不相为谋，但尊重其选择".into(),
            personality_archetype: "李长寿".into(),
            core_value: "守护所爱，庇护一方".into(),
            altruism: "救醒他，然后让他拿身上的东西作为报答。".into(),
            dao_quest: "探寻真理，穷究天道".into(),
            player_name: "柳青鸾".into(),
            dao_name: "慈心".into(),
            narrative_style: "female".into(),
        };
        let stats = choices.calculate_initial_stats();
        // Q1 医道: s=5, sp=9, b=5, so=6, se=7, h=7
        // Q2 道观: sp+2, h+2 → s=5, sp=11, b=5, so=6, se=7, h=9
        // Q3a 佛门: h+3, b+1 → s=5, sp=11, b=6, so=6, se=7, h=12
        // Q3b 报恩: h+2 → s=5, sp=11, b=6, so=6, se=7, h=14
        // Q4 散修收留: h+3, se+1 → s=5, sp=11, b=6, so=6, se=8, h=17
        // Q6 探寻真理: se+3, sp+2 → s=5, sp=13, b=6, so=6, se=11, h=17
        // Q5d 救醒报酬: h+1 → s=5, sp=13, b=6, so=6, se=11, h=18
        assert_eq!(stats.sword_art, 5);
        assert_eq!(stats.spell_art, 13);
        assert_eq!(stats.blood_qi, 6);
        assert_eq!(stats.spirit_soul, 6);
        assert_eq!(stats.divine_sense, 11);
        assert_eq!(stats.dao_heart, 18);
    }

    #[test]
    fn test_stats_orphan_explorer_rogue() {
        let choices = CreationChoices {
            family_background: "山野遗孤".into(),
            childhood_experience: "山林探险".into(),
            sect_category: "散修独行".into(),
            join_reason: "机缘巧合，误打误撞".into(),
            entry_method: "落崖奇遇，得遇洞府".into(),
            demonic_stance: "正道虚伪，魔道直率。更欣赏魔道".into(),
            personality_archetype: "白小纯".into(),
            core_value: "逍遥自在，不受束缚".into(),
            altruism: "取丹离去，心中默念抱歉。筑基丹太珍贵了。".into(),
            dao_quest: "只为自由，不受束缚".into(),
            player_name: "云无痕".into(),
            dao_name: "".into(),
            narrative_style: "male".into(),
        };
        let stats = choices.calculate_initial_stats();
        // Q1 山野: s=7, sp=5, b=10, so=6, se=5, h=7
        // Q2 山林: se+1, b+1, sp+1 → s=7, sp=6, b=11, so=6, se=6, h=7
        // Q3a 独行: b+2, so+2 → s=7, sp=6, b=13, so=8, se=6, h=7
        // Q3b 机缘: se+1 → s=7, sp=6, b=13, so=8, se=7, h=7
        // Q4 落崖: b+3 → s=7, sp=6, b=16, so=8, se=7, h=7
        // Q6 自由: b+3, s+2 → s=9, sp=6, b=19, so=8, se=7, h=7
        // Q5d 取丹: se+1 → s=9, sp=6, b=19, so=8, se=8, h=7
        assert_eq!(stats.sword_art, 9);
        assert_eq!(stats.blood_qi, 19);
        assert_eq!(stats.divine_sense, 8);
        assert_eq!(stats.dao_heart, 7);
    }

    // ═══════════════════════════════════════════════════════════════
    // CreationChoices — calculate_initial_spirit_stones
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_spirit_stones_merchant_path() {
        let choices = CreationChoices {
            family_background: "商贾之家".into(),     // +80
            childhood_experience: "市井谋生".into(),   // +15
            sect_category: "旁门左道".into(),           // +20
            join_reason: "家族安排，身不由己".into(),   // +10
            entry_method: "仙师路过，被收为记名弟子".into(),
            demonic_stance: "势不两立，见之必除".into(),
            personality_archetype: "韩立".into(),
            core_value: "长生久视，寿与天齐".into(),
            altruism: "先救人，再取丹。人命关天。".into(), // +0
            dao_quest: "重振家声，光耀门楣".into(),   // +20
            player_name: "陆书白".into(),
            dao_name: "".into(),
            narrative_style: "male".into(),
        };
        let stones = choices.calculate_initial_spirit_stones();
        assert_eq!(stones, 145, "80+15+20+10+20 = 145");
    }

    #[test]
    fn test_spirit_stones_clan_path() {
        let choices = CreationChoices {
            family_background: "修真世家".into(),       // +30
            childhood_experience: "静心读书".into(),     // +0
            sect_category: "散修联盟".into(),             // +30
            join_reason: "仰慕其名，主动拜入".into(),   // +0
            entry_method: "宗门大开山门，通过考核入外门".into(),
            demonic_stance: "势不两立，见之必除".into(),
            personality_archetype: "韩立".into(),
            core_value: "长生久视，寿与天齐".into(),
            altruism: "救醒他，然后让他拿身上的东西作为报答。".into(), // +50
            dao_quest: "超脱生死，得证长生".into(),     // +0
            player_name: "赵明远".into(),
            dao_name: "".into(),
            narrative_style: "male".into(),
        };
        let stones = choices.calculate_initial_spirit_stones();
        assert_eq!(stones, 110, "30+30+50 = 110");
    }

    #[test]
    fn test_spirit_stones_humble_zero() {
        let stones = sample_choices().calculate_initial_spirit_stones();
        // 寒门: 0, 静心: 0, 仙门: 0, 仰慕: 0, 宗门考核: 0, 先救人: 0, 超脱: 0
        assert_eq!(stones, 0);
    }

    // ═══════════════════════════════════════════════════════════════
    // CreationChoices — calculate_initial_items
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_items_general_path() {
        let items = alt_choices().calculate_initial_items();
        // 将门: 铁剑
        // Q5d 取走丹药和令牌: 筑基丹 + 内门令牌
        assert_eq!(items.len(), 3);
        assert!(items.iter().any(|i| i.name == "铁剑"));
        assert!(items.iter().any(|i| i.name == "筑基丹"));
        assert!(items.iter().any(|i| i.name == "内门令牌"));
    }

    #[test]
    fn test_items_healer_path() {
        let choices = CreationChoices {
            family_background: "医道传家".into(),
            childhood_experience: "静心读书".into(),
            sect_category: "仙门正宗".into(),
            join_reason: "仰慕其名，主动拜入".into(),
            entry_method: "误入上古遗迹，获得传承玉简".into(),
            demonic_stance: "势不两立，见之必除".into(),
            personality_archetype: "韩立".into(),
            core_value: "长生久视，寿与天齐".into(),
            altruism: "先救人，再取丹。人命关天。".into(),
            dao_quest: "超脱生死，得证长生".into(),
            player_name: "孙若虚".into(),
            dao_name: "".into(),
            narrative_style: "male".into(),
        };
        let items = choices.calculate_initial_items();
        // 医道传家: 疗伤丹x3
        // 上古遗迹: 残破玉简
        assert_eq!(items.len(), 2);
        let healer_dan = items.iter().find(|i| i.name == "疗伤丹").unwrap();
        assert_eq!(healer_dan.quantity, 3);
        assert!(items.iter().any(|i| i.name == "残破玉简"));
    }

    #[test]
    fn test_items_orphan_cliff_path() {
        let choices = CreationChoices {
            family_background: "山野遗孤".into(),
            childhood_experience: "山林探险".into(),
            sect_category: "散修独行".into(),
            join_reason: "机缘巧合，误打误撞".into(),
            entry_method: "落崖奇遇，得遇洞府".into(),
            demonic_stance: "道不同不相为谋，但尊重其选择".into(),
            personality_archetype: "白小纯".into(),
            core_value: "逍遥自在，不受束缚".into(),
            altruism: "取丹离去，心中默念抱歉。筑基丹太珍贵了。".into(),
            dao_quest: "只为自由，不受束缚".into(),
            player_name: "云无痕".into(),
            dao_name: "".into(),
            narrative_style: "male".into(),
        };
        let items = choices.calculate_initial_items();
        // 山野: 疗伤丹x1
        // 机缘巧合: 机缘信物
        // 落崖: 未知丹药x2
        // 取丹: 筑基丹x1
        assert_eq!(items.len(), 4);
        assert!(items.iter().any(|i| i.name == "疗伤丹" && i.quantity == 1));
        assert!(items.iter().any(|i| i.name == "机缘信物"));
        assert!(items.iter().any(|i| i.name == "未知丹药" && i.quantity == 2));
        assert!(items.iter().any(|i| i.name == "筑基丹"));
    }

    // ═══════════════════════════════════════════════════════════════
    // CreationChoices — collect_background_flags
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_flags_basic() {
        let flags = sample_choices().collect_background_flags();
        assert!(flags.contains(&"humble-origin".into()));
        assert!(flags.contains(&"studious-youth".into()));
        assert!(flags.contains(&"orthodox-sect".into()));
        assert!(flags.contains(&"cautious".into()));
        assert!(flags.contains(&"value-immortality".into()));
        assert!(flags.contains(&"altruistic".into()));
    }

    #[test]
    fn test_flags_demonic() {
        let flags = alt_choices().collect_background_flags();
        assert!(flags.contains(&"general-descendant".into()));
        assert!(flags.contains(&"martial-youth".into()));
        assert!(flags.contains(&"demonic-sect".into()));
        assert!(flags.contains(&"ruthless".into()));
        assert!(flags.contains(&"value-justice".into()));
    }

    #[test]
    fn test_flags_all_q5_archetypes() {
        let archetypes = [
            ("韩立", "cautious"),
            ("王林", "ruthless"),
            ("徐缺", "witty"),
            ("萧炎", "loyal"),
            ("白小纯", "lucky"),
            ("方平", "strategic"),
            ("李长寿", "hidden"),
        ];
        for (name, expected) in &archetypes {
            let mut c = sample_choices();
            c.personality_archetype = name.to_string();
            let flags = c.collect_background_flags();
            assert!(flags.contains(&expected.to_string()),
                "archetype '{}' should give flag '{}'", name, expected);
        }
    }

    #[test]
    fn test_flags_all_core_values() {
        let values = [
            ("长生久视，寿与天齐", "value-immortality"),
            ("逍遥自在，不受束缚", "value-freedom"),
            ("登临绝顶，俯瞰众生", "value-power"),
            ("守护所爱，庇护一方", "value-protection"),
            ("探寻真理，穷究天道", "value-knowledge"),
            ("快意恩仇，不负此生", "value-justice"),
        ];
        for (label, expected) in &values {
            let mut c = sample_choices();
            c.core_value = label.to_string();
            let flags = c.collect_background_flags();
            assert!(flags.contains(&expected.to_string()),
                "core value '{}' should give flag '{}'", label, expected);
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // CreationChoices — to_prompt_text
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_to_prompt_text_contains_all_fields() {
        let text = sample_choices().to_prompt_text();
        assert!(text.contains("孙若虚"));
        assert!(text.contains("寒门之后"));
        assert!(text.contains("静心读书"));
        assert!(text.contains("仙门正宗"));
        assert!(text.contains("仰慕其名"));
        assert!(text.contains("宗门大开山门"));
        assert!(text.contains("韩立式"));
        assert!(text.contains("超脱生死"));
    }

    #[test]
    fn test_to_prompt_text_female_style() {
        let mut c = sample_choices();
        c.narrative_style = "female".into();
        c.dao_name = "青鸾".into();
        let text = c.to_prompt_text();
        assert!(text.contains("女频"));
        assert!(text.contains("青鸾"));
    }

    // ═══════════════════════════════════════════════════════════════
    // WorldConfig — default / serialization / prompt section
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_world_config_default_values() {
        let wc = WorldConfig::default();
        assert_eq!(wc.narrative_style, "male");
        assert_eq!(wc.era_name, "太虚历");
        assert_eq!(wc.continent_name, "东荒");
        assert_eq!(wc.sect_name, "青云宗");
        assert_eq!(wc.key_npc_name, "清虚道人");
        assert_eq!(wc.key_npc_role, "师尊");
        assert_eq!(wc.key_npc_realm, "元婴后期");
    }

    #[test]
    fn test_world_config_json_roundtrip() {
        let wc = WorldConfig::default();
        let json = wc.to_json();
        let parsed = WorldConfig::from_json(&json).expect("roundtrip failed");
        assert_eq!(parsed.sect_name, wc.sect_name);
        assert_eq!(parsed.key_npc_name, wc.key_npc_name);
        assert_eq!(parsed.narrative_style, wc.narrative_style);
    }

    #[test]
    fn test_world_config_from_invalid_json() {
        assert!(WorldConfig::from_json("not json").is_none());
    }

    #[test]
    fn test_world_config_prompt_section_male_style() {
        let wc = WorldConfig::default();
        let section = wc.to_system_prompt_section();
        assert!(section.contains("男频"));
        assert!(section.contains("侧重力量成长"));
        assert!(section.contains("青云宗"));
        assert!(section.contains("清虚道人"));
        assert!(section.contains("师尊"));
        assert!(section.contains("元婴后期"));
        assert!(section.contains("黑风岭散修"));
        assert!(!section.contains("女频"));
    }

    #[test]
    fn test_world_config_prompt_section_female_style() {
        let mut wc = WorldConfig::default();
        wc.narrative_style = "female".into();
        let section = wc.to_system_prompt_section();
        assert!(section.contains("女频"));
        assert!(section.contains("侧重人物关系"));
    }

    #[test]
    fn test_world_config_prompt_section_no_npc() {
        let mut wc = WorldConfig::default();
        wc.key_npc_name = "无".into();
        let section = wc.to_system_prompt_section();
        assert!(!section.contains("清虚道人"));
        assert!(!section.contains("师尊"));
        assert!(section.contains("独自一人"));
        assert!(section.contains("靠自己去结识他人"));
    }

    #[test]
    fn test_world_config_prompt_section_no_threat() {
        let mut wc = WorldConfig::default();
        wc.nearby_threat_name = "".into();
        wc.nearby_threat_description = "".into();
        let section = wc.to_system_prompt_section();
        assert!(!section.contains("附近威胁"));
    }

    #[test]
    fn test_world_config_prompt_section_rogue_cultivator() {
        let mut wc = WorldConfig::default();
        wc.sect_name = "无".into();
        wc.sect_type = "散修".into();
        wc.sect_scale = "无门无派".into();
        wc.player_title = "散修".into();
        wc.player_title_description = "一名漂泊无依的散修，无门无派，四海为家。".into();
        wc.starting_location_name = "黑风岭·废弃矿洞".into();
        wc.key_npc_name = "无".into();
        let section = wc.to_system_prompt_section();
        assert!(section.contains("散修"));
        assert!(section.contains("黑风岭"));
        assert!(section.contains("独自一人"));
    }
}
