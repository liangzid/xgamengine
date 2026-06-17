use serde::{Deserialize, Serialize};

/// Context limit: 1M tokens for deepseek-v4-pro, trigger compaction at 90%
const CONTEXT_LIMIT: usize = 1_000_000;
const COMPACTION_THRESHOLD: usize = (CONTEXT_LIMIT as f64 * 0.9) as usize; // 900K
/// Keep the most recent N rounds intact after compaction
const KEEP_RECENT_ROUNDS: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversationWindow {
    messages: Vec<Message>,
    /// Compressed summary of older history (set after first compaction)
    summary: Option<String>,
    /// Round counter (incremented on each user-assistant pair)
    round_count: usize,
}

impl ConversationWindow {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            summary: None,
            round_count: 0,
        }
    }

    pub fn append_turn(&mut self, user_msg: &str, assistant_msg: &str) {
        self.messages.push(Message { role: "user".into(), content: user_msg.into() });
        self.messages.push(Message { role: "assistant".into(), content: assistant_msg.into() });
        self.round_count += 1;
        // No truncation — we use compaction instead
    }

    /// Rough token estimation: ~0.5 tokens per Chinese character, ~1 per ASCII word
    pub fn estimated_tokens(&self) -> usize {
        let mut total = 0usize;
        for msg in &self.messages {
            total += estimate_tokens(&msg.content);
        }
        if let Some(ref s) = self.summary {
            total += estimate_tokens(s);
        }
        total
    }

    /// Check if compaction is needed
    pub fn needs_compaction(&self) -> bool {
        self.round_count > KEEP_RECENT_ROUNDS + 5
            && self.estimated_tokens() > COMPACTION_THRESHOLD
    }

    /// Compact: keep recent messages, summarize the rest.
    /// Returns the summary text that should be sent to the LLM for compaction.
    /// `history_text` is the concatenated older messages to summarize.
    pub fn prepare_compaction(&self) -> Option<String> {
        if self.round_count <= KEEP_RECENT_ROUNDS + 5 {
            return None; // not enough history to compact
        }

        let keep_msgs = KEEP_RECENT_ROUNDS * 2; // user + assistant per round
        if self.messages.len() <= keep_msgs + 4 {
            return None;
        }

        // Take oldest messages (before the keep window) for summarization
        let compact_count = self.messages.len() - keep_msgs;
        let mut text = String::from(
            "请将以下修仙游戏对话历史压缩为一段简洁的剧情摘要（200字以内），保留：关键事件、人物关系变化、物品得失、境界变化、重要flag。\n\n"
        );

        for msg in self.messages.iter().take(compact_count) {
            text.push_str(&format!("[{}]: {}\n", msg.role, msg.content));
        }

        Some(text)
    }

    /// Apply the compaction: replace old messages with summary
    pub fn apply_compaction(&mut self, summary_text: &str) {
        let keep_msgs = KEEP_RECENT_ROUNDS * 2;
        if self.messages.len() <= keep_msgs + 4 {
            return;
        }

        let compact_count = self.messages.len() - keep_msgs;
        // Remove old messages, keep recent ones
        self.messages.drain(0..compact_count);

        // Store the summary
        self.summary = Some(summary_text.to_string());
    }

    /// Build the full context messages array for API call
    pub fn get_context_messages(&self) -> Vec<Message> {
        let mut result = Vec::new();

        // Insert summary as a system-level context message if present
        if let Some(ref summary) = self.summary {
            result.push(Message {
                role: "system".into(),
                content: format!("【剧情回顾】\n{}", summary),
            });
        }

        result.extend(self.messages.clone());
        result
    }

    /// Number of rounds so far
    pub fn round_count(&self) -> usize {
        self.round_count
    }
}

/// Rough token count: Chinese char ≈ 0.5 token, ASCII word ≈ 1 token,
/// whitespace/punctuation ≈ 0.25 token
fn estimate_tokens(text: &str) -> usize {
    let mut count = 0usize;
    let mut in_ascii_word = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if in_ascii_word {
                count += 1; // end of word
                in_ascii_word = false;
            }
            count += 1; // whitespace ≈ 0.25, round up slightly
        } else if ch.is_ascii_alphanumeric() {
            in_ascii_word = true;
        } else if ch as u32 > 0x7F {
            // CJK or other multi-byte char
            if in_ascii_word {
                count += 1;
                in_ascii_word = false;
            }
            count += 1; // CJK ≈ 1-2 chars per token, rough estimate
        } else {
            // Punctuation
            if in_ascii_word {
                count += 1;
                in_ascii_word = false;
            }
        }
    }
    if in_ascii_word {
        count += 1;
    }

    // Conservative: divide by 2 to get approximate token count
    // (most Chinese chars use 1-2 tokens, English ~1 token per word)
    count / 2
}
