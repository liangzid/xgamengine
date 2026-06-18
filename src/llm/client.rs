use serde_json::{json, Value};
use tokio::sync::mpsc;

const API_BASE_URL: &str = "https://api.deepseek.com";
const MODEL: &str = "deepseek-v4-pro";

#[derive(Clone)]
pub struct LlmClient {
    api_key: String,
}

#[derive(Debug, Clone)]
pub enum SseEvent {
    Content(String),
    Done,
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

    /// Non-streaming chat completion. Returns (content, total_tokens_used).
    pub async fn chat_completion_sync(
        &self,
        messages: &[crate::memory::Message],
    ) -> Result<(String, u64), String> {
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

        let tokens = json_resp["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0);

        Ok((content, tokens))
    }

    /// Streaming chat completion — yields SseEvent via channel
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
                        let _ = tx.send(SseEvent::Done);
                        return Ok(());
                    }

                    // Parse the JSON data chunk
                    if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                        if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                            let _ = tx.send(SseEvent::Content(content.to_string()));
                        }
                        // Check finish reason
                        if let Some(finish) = parsed["choices"][0]["finish_reason"].as_str() {
                            if !finish.is_empty() {
                                let _ = tx.send(SseEvent::Done);
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        let _ = tx.send(SseEvent::Done);
        Ok(())
    }
}
