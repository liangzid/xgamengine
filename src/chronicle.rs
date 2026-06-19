use serde::{Deserialize, Serialize};

/// A single volume in the chronicle — covers a story arc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleVolume {
    pub title: String,
    pub summary: String,
    pub start_round: i32,
    pub end_round: i32,
    pub key_events: Vec<String>,
    pub realm_at_end: String,
}

/// The 岁月史书 — a growing record of the player's journey.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Chronicle {
    pub volumes: Vec<ChronicleVolume>,
    /// Events accumulated for the current (in-progress) volume.
    current_events: Vec<String>,
    current_start_round: i32,
    /// How many rounds since the last volume was closed.
    rounds_since_last_volume: i32,
}

/// Major story flags that trigger volume closure.
const VOLUME_TRIGGER_FLAGS: &[&str] = &[
    "breakthrough-baset",       // 筑基突破
    "breakthrough-golden-core", // 金丹突破
    "breakthrough-nascent",     // 元婴突破
    "entered-inner-sect",       // 进入内门
    "entered-secret-realm",     // 进入秘境
    "sect-tournament",          // 宗门大比
    "defeated-major-foe",       // 击败强敌
    "found-ancient-legacy",     // 获得上古传承
    "master-departed",          // 师尊离去
];

/// Auto-split after this many rounds without a trigger flag.
const AUTO_SPLIT_ROUNDS: i32 = 30;

impl Chronicle {
    pub fn new() -> Self {
        Self {
            volumes: Vec::new(),
            current_events: Vec::new(),
            current_start_round: 1,
            rounds_since_last_volume: 0,
        }
    }

    /// Record a round's events. Called every turn after state changes are applied.
    pub fn record_round(&mut self, round: i32, realm: &str, changes: &crate::state::StateChange, flags: &[String]) {
        self.rounds_since_last_volume += 1;

        // Collect notable events from this round
        if let Some(ref event) = changes.add_event {
            self.current_events.push(event.clone());
        }
        if let Some(ref flag) = changes.add_flag {
            self.current_events.push(format!("获得标记: {}", flag));
        }
        if let Some(ref techs) = changes.add_techniques {
            for t in techs {
                self.current_events.push(format!("习得功法: {}", t.name));
            }
        }
        if let Some(ref items) = changes.add_items {
            for item in items {
                self.current_events.push(format!("获得物品: {} ({} {})", item.name, item.quality, item.item_type));
            }
        }
        if let Some(ref realm) = changes.set_realm {
            self.current_events.push(format!("境界突破: {}", realm));
        }
        if let Some(ref loc) = changes.set_current_location {
            self.current_events.push(format!("前往: {}", loc));
        }

        // Check if a volume should be closed
        let should_close = flags.iter().any(|f| VOLUME_TRIGGER_FLAGS.contains(&f.as_str()))
            || self.rounds_since_last_volume >= AUTO_SPLIT_ROUNDS;

        if should_close && !self.current_events.is_empty() {
            self.close_volume(round, realm);
        }
    }

    /// Close the current volume and start a new one.
    fn close_volume(&mut self, round: i32, realm: &str) {
        let volume = ChronicleVolume {
            title: format!("第{}卷", self.volumes.len() + 1),
            summary: String::new(), // filled later by LLM summarization
            start_round: self.current_start_round,
            end_round: round,
            key_events: std::mem::take(&mut self.current_events),
            realm_at_end: realm.to_string(),
        };
        self.volumes.push(volume);
        self.current_start_round = round + 1;
        self.rounds_since_last_volume = 0;
    }

    /// Build a prompt to ask the LLM to summarize a specific volume.
    pub fn build_summary_prompt(&self, volume_index: usize) -> Option<String> {
        let vol = self.volumes.get(volume_index)?;
        let events_text = vol.key_events.join("\n  - ");

        Some(format!(
            r#"你是修仙世界的岁月史官。请为以下修仙者的"第{}卷"撰写一段简洁的卷首摘要（150字以内），以古风史书笔法，记录此卷的关键事件与境界变化。

起始回合: {}  →  结束回合: {}
结束境界: {}
关键事件:
  - {}

卷首摘要（150字以内）："#,
            volume_index + 1, vol.start_round, vol.end_round,
            vol.realm_at_end, events_text
        ))
    }

    /// Apply an LLM-generated summary to a volume.
    pub fn set_volume_summary(&mut self, volume_index: usize, summary: &str) {
        if let Some(vol) = self.volumes.get_mut(volume_index) {
            vol.summary = summary.to_string();
        }
    }

    /// Generate the full chronicle as a Markdown string.
    pub fn to_markdown(&self) -> String {
        let mut md = String::from("# 岁月史书\n\n");
        md.push_str("> 天道无情，岁月留痕。此卷记录修仙者自踏入仙途以来的每一步脚印。\n\n");

        for (i, vol) in self.volumes.iter().enumerate() {
            md.push_str(&format!("## 第{}卷: {}\n\n", i + 1, vol.title));
            md.push_str(&format!("**第{}回合 — 第{}回合**  |  终境: {}\n\n", 
                vol.start_round, vol.end_round, vol.realm_at_end));
            if !vol.summary.is_empty() {
                md.push_str(&format!("{}\n\n", vol.summary));
            }
            md.push_str("### 关键事件\n\n");
            for event in &vol.key_events {
                md.push_str(&format!("- {}\n", event));
            }
            md.push('\n');
        }

        if self.volumes.is_empty() {
            md.push_str("*史书尚未着墨，仙途方才启程。*\n");
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chronicle_empty() {
        let c = Chronicle::new();
        assert_eq!(c.volumes.len(), 0);
        assert!(c.to_markdown().contains("尚未着墨"));
    }

    #[test]
    fn test_record_and_close() {
        let mut c = Chronicle::new();
        let changes = crate::state::StateChange {
            add_event: Some("发现灵石矿脉".into()),
            add_flag: Some("breakthrough-baset".into()),
            ..Default::default()
        };
        let flags = vec!["breakthrough-baset".to_string()];
        c.record_round(20, "筑基期初期", &changes, &flags);
        assert_eq!(c.volumes.len(), 1);
        assert_eq!(c.volumes[0].end_round, 20);
        assert_eq!(c.volumes[0].realm_at_end, "筑基期初期");
    }
}
