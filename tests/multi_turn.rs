/// Multi-turn JSON format persistence test.
/// Verifies: after storing raw JSON in conversation history,
/// the LLM continues to output valid JSON with options on subsequent turns.
/// Run with: cargo test --test multi_turn -- --nocapture
use std::path::PathBuf;
use xgamengine::engine::Engine;
use xgamengine::llm::client::LlmClient;
use xgamengine::prompt::builder;

#[tokio::test]
async fn test_multi_turn_json_persistence() {
    let client = LlmClient::from_env().expect("DEEPSEEK_API_KEY must be set");
    let template_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent().unwrap()
        .join("templates");
    let mut engine = Engine::new(template_dir, client);

    // Turn 1: opening
    let opening = builder::build_opening_prompt(
        &engine.world_config, "孙若虚", "", "寒门之后",
        "宗门大开山门，通过考核入外门",
    );

    println!("=== Turn 1 (opening) ===");
    let out1 = engine.start_game_ex(&opening).await.expect("turn 1 failed");
    println!("Options: {:?}", out1.options);
    assert!(!out1.options.is_empty(), "turn 1 must have options");
    assert_eq!(out1.options.len(), 5, "turn 1: 4 + custom input = 5"); // includes "✍ 自由输入"
    assert!(!out1.narrative.is_empty());

    // Verify the window contains raw JSON (not just narrative)
    let msgs = engine.window.get_context_messages();
    let last_asst = msgs.iter().rev()
        .find(|m| m.role == "assistant")
        .expect("should have assistant message");
    assert!(last_asst.content.contains("\"narrative\""),
        "assistant message should contain JSON format markers, but got: {}",
        &last_asst.content.chars().take(200).collect::<String>());
    assert!(last_asst.content.contains("\"options\""),
        "assistant message should contain options JSON key");

    // Turn 2: follow-up input
    println!("\n=== Turn 2 (follow-up) ===");
    let out2 = engine.process_input(&out1.options[0]).await.expect("turn 2 failed");
    println!("Options: {:?}", out2.options);
    assert!(!out2.options.is_empty(), "turn 2 must have options after JSON history");
    assert!(out2.options.len() >= 4, "turn 2 must have 4+ options, got {}", out2.options.len());
    assert!(!out2.narrative.is_empty());

    // Turn 3: another follow-up
    println!("\n=== Turn 3 ===");
    let out3 = engine.process_input(&out2.options[0]).await.expect("turn 3 failed");
    println!("Options: {:?}", out3.options);
    assert!(!out3.options.is_empty(), "turn 3 must have options after JSON history");
    assert!(out3.options.len() >= 4, "turn 3 must have 4+ options, got {}", out3.options.len());

    // Check conversation history for JSON format
    let msgs3 = engine.window.get_context_messages();
    let json_count = msgs3.iter()
        .filter(|m| m.role == "assistant" && m.content.contains("\"narrative\""))
        .count();
    println!("\nAssistant messages with JSON format: {}/{}", json_count,
        msgs3.iter().filter(|m| m.role == "assistant").count());
    assert!(json_count >= 2, "at least 2 assistant messages should have JSON format");

    println!("\n=== SUCCESS: JSON format persists across {} turns ===", 3);
}
