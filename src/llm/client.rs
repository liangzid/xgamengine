use serde_json::{json, Value};
use tokio::sync::mpsc;

const API_BASE_URL: &str = "https://api.deepseek.com";
const MODEL: &str = "deepseek-v4-pro";

#[derive(Clone)]
pub struct LlmClient {
    api_key: String,
}

/// Token usage from a single LLM API call.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl TokenUsage {
    /// Total tokens. Convenience alias so callers don't need to pick a field.
    pub fn total(&self) -> u64 {
        self.total_tokens
    }
}

impl std::ops::Add for TokenUsage {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + rhs.prompt_tokens,
            completion_tokens: self.completion_tokens + rhs.completion_tokens,
            total_tokens: self.total_tokens + rhs.total_tokens,
        }
    }
}

impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, rhs: Self) {
        self.prompt_tokens += rhs.prompt_tokens;
        self.completion_tokens += rhs.completion_tokens;
        self.total_tokens += rhs.total_tokens;
    }
}

#[derive(Debug, Clone)]
pub enum SseEvent {
    Content(String),
    /// Streaming completed. `usage` is Some when the API included usage in the
    /// final SSE chunk; None when the stream ended without usage data (e.g.
    /// `[DONE]` before usage arrived).
    Done { usage: Option<TokenUsage> },
    Error(String),
}

impl LlmClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn from_env() -> Result<Self, String> {
        let key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY not set".to_string())?;
        Ok(Self::new(key))
    }

    /// Non-streaming chat completion. Returns (content, usage).
    pub async fn chat_completion_sync(
        &self,
        messages: &[crate::memory::Message],
    ) -> Result<(String, TokenUsage), String> {
        let client = reqwest::Client::new();
        let msgs: Vec<Value> = messages.iter().map(|m| json!({
            "role": m.role,
            "content": m.content,
        })).collect();

        let body = json!({
            "model": MODEL,
            "messages": msgs,
            "thinking": { "type": "disabled" },
            "temperature": 0.9,
            "max_tokens": 8192,
        });

        let resp = client.post(format!("{}/chat/completions", API_BASE_URL))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        let json_resp: Value = resp.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let content = json_resp["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No content in response: {:?}", json_resp))?;

        let usage = TokenUsage {
            prompt_tokens: json_resp["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
            completion_tokens: json_resp["usage"]["completion_tokens"].as_u64().unwrap_or(0),
            total_tokens: json_resp["usage"]["total_tokens"].as_u64().unwrap_or(0),
        };

        Ok((content, usage))
    }

    /// Streaming chat completion — yields SseEvent via channel.
    /// The final Done event carries token usage when available.
    pub async fn chat_completion_streaming(
        &self,
        messages: &[crate::memory::Message],
        tx: mpsc::UnboundedSender<SseEvent>,
    ) -> Result<(), String> {
        let client = reqwest::Client::new();
        let msgs: Vec<Value> = messages.iter().map(|m| json!({
            "role": m.role,
            "content": m.content,
        })).collect();

        let body = json!({
            "model": MODEL,
            "messages": msgs,
            "thinking": { "type": "disabled" },
            "temperature": 0.9,
            "max_tokens": 8192,
            "stream": true,
        });

        let resp = client.post(format!("{}/chat/completions", API_BASE_URL))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        use futures::StreamExt;
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut last_usage: Option<TokenUsage> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete SSE lines
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() { continue; }
                if line.starts_with(':') { continue; } // comment

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        let _ = tx.send(SseEvent::Done { usage: last_usage });
                        return Ok(());
                    }

                    // Parse the JSON data chunk
                    if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            let _ = tx.send(SseEvent::Content(content.to_string()));
                        }
                        // Capture usage if present (typically in the last chunk)
                        if let Some(total) = parsed["usage"]["total_tokens"].as_u64() {
                            last_usage = Some(TokenUsage {
                                prompt_tokens: parsed["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
                                completion_tokens: parsed["usage"]["completion_tokens"].as_u64().unwrap_or(0),
                                total_tokens: total,
                            });
                        }
                        // Check finish reason — if present, the stream may end without [DONE]
                        if let Some(finish) = parsed["choices"][0]["finish_reason"].as_str() {
                            if !finish.is_empty() {
                                let _ = tx.send(SseEvent::Done { usage: last_usage });
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        let _ = tx.send(SseEvent::Done { usage: last_usage });
        Ok(())
    }
}
