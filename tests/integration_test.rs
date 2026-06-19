/// Integration tests with real DeepSeek API.
/// Requires DEEPSEEK_API_KEY env var.
/// Run with: cargo test --test integration_test -- --nocapture
use std::path::PathBuf;
use xgamengine::engine::Engine;
use xgamengine::llm::client::LlmClient;
use xgamengine::state::CreationChoices;

fn make_engine() -> Engine {
    let client = LlmClient::from_env().expect("DEEPSEEK_API_KEY must be set");
    let template_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent().unwrap()  // up from xgamengine/
        .join("templates");
    Engine::new(template_dir, client)
}

fn test_choices_male_orthodox() -> CreationChoices {
    CreationChoices {
        family_background: "寒门之后".into(),
        childhood_experience: "静心读书".into(),
        sect_category: "仙门正宗".into(),
        join_reason: "仰慕其名，主动拜入".into(),
        entry_method: "仙师路过，被收为记名弟子".into(),
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

fn test_choices_female_buddhist() -> CreationChoices {
    CreationChoices {
        family_background: "书香门第".into(),
        childhood_experience: "拜入道观".into(),
        sect_category: "佛门禅院".into(),
        join_reason: "为报恩情，被恩人引入".into(),
        entry_method: "散修收留，入门为杂役".into(),
        demonic_stance: "道不同不相为谋，但尊重其选择".into(),
        personality_archetype: "李长寿".into(),
        core_value: "守护所爱，庇护一方".into(),
        altruism: "先救人，再取丹。人命关天。".into(),
        dao_quest: "探寻真理，穷究天道".into(),
        player_name: "柳青鸾".into(),
        dao_name: "慈心".into(),
        narrative_style: "female".into(),
    }
}

/// Test full world generation — send Q1-Q7 to LLM, get WorldConfig back.
/// Verifies the LLM generates a coherent world matching the choices.
#[tokio::test]
async fn test_generate_world_from_choices() {
    let engine = make_engine();
    let choices = test_choices_male_orthodox();

    let (world_config, _tokens) = engine.generate_world(&choices).await
        .expect("generate_world should succeed");

    // Verify LLM-generated fields are populated
    assert!(!world_config.era_name.is_empty(), "era_name should not be empty");
    assert!(!world_config.era_description.is_empty());
    assert!(!world_config.continent_name.is_empty());
    assert!(!world_config.continent_description.is_empty());
    assert!(!world_config.sect_name.is_empty(), "sect_name should not be empty");
    assert!(!world_config.sect_scale.is_empty());
    assert!(!world_config.sect_description.is_empty());
    assert!(!world_config.player_title.is_empty());
    assert!(!world_config.starting_location_name.is_empty());
    assert!(!world_config.world_hook.is_empty());

    // Sect type must match the choice (仙门正宗)
    assert!(world_config.sect_type.contains("仙门") || world_config.sect_type.contains("正宗"),
        "sect_type should reflect orthodox sect: got '{}'", world_config.sect_type);

    // Since entry_method is "仙师路过", there should be a mentor
    assert!(!world_config.key_npc_name.is_empty(),
        "仙师路过 entry should produce a mentor, got empty key_npc_name");
    assert_ne!(world_config.key_npc_name, "无",
        "仙师路过 entry should produce a mentor, got '无'");

    // Merged choices should be present
    assert_eq!(world_config.narrative_style, "male");
    assert_eq!(world_config.sect_category, "仙门正宗");
    assert_eq!(world_config.personality_archetype, "韩立");

    println!("=== World Gen Result ===");
    println!("Era: {} — {}", world_config.era_name, world_config.era_description);
    println!("Continent: {} — {}", world_config.continent_name, world_config.continent_description);
    println!("Sect: {} ({}, {})", world_config.sect_name, world_config.sect_type, world_config.sect_scale);
    println!("Sect desc: {}", world_config.sect_description);
    println!("Player: {} — {}", world_config.player_title, world_config.player_title_description);
    println!("Location: {} — {}", world_config.starting_location_name, world_config.starting_location_description);
    println!("NPC: {} ({}) — {}", world_config.key_npc_name, world_config.key_npc_role, world_config.key_npc_description);
    println!("Threat: {} — {}", world_config.nearby_threat_name, world_config.nearby_threat_description);
    println!("Hook: {}", world_config.world_hook);
}

/// Test full creation flow: generate world → init engine → opening narrative.
#[tokio::test]
async fn test_full_creation_to_opening_narrative() {
    let mut engine = make_engine();
    let choices = test_choices_female_buddhist();

    // Step 1: Generate world
    let (world_config, _tokens) = engine.generate_world(&choices).await
        .expect("world generation should succeed");
    println!("\n=== Generated World (Female/Buddhist) ===");
    println!("Sect: {}", world_config.sect_name);
    println!("NPC: {}", world_config.key_npc_name);

    // Step 2: Initialize engine from choices + world
    engine.init_from_creation(&choices, world_config);

    // Verify state was initialized correctly
    assert!(!engine.state.sect.is_empty());
    assert!(!engine.state.current_location.is_empty());
    assert_eq!(engine.state.round, 0);
    // Stats should be non-zero
    assert!(engine.state.stats.dao_heart > 10, 
        "Buddhist+taoist+scholar should have high dao_heart, got {}", 
        engine.state.stats.dao_heart);
    // flags should contain background flags from choices
    assert!(engine.state.flags.iter().any(|f| f == "scholar-born"),
        "flags should contain scholar-born");

    // Step 3: Generate opening narrative
    let opening = xgamengine::prompt::builder::build_opening_prompt(
        &engine.world_config,
        &choices.player_name,
        &choices.dao_name,
        &choices.family_background,
        &choices.entry_method,
    );

    let output = engine.start_game_ex(&opening).await
        .expect("opening narrative should succeed");

    // Verify opening narrative output
    assert!(!output.narrative.is_empty(), "narrative should not be empty");
    assert!(output.narrative.len() >= 50, 
        "narrative too short: {} chars", output.narrative.len());
    assert!(!output.options.is_empty(), "should have options");
    assert!(output.options.len() >= 4, 
        "should have at least 4 options (got {})", output.options.len());
    assert!(output.round == 0);

    println!("\n=== Opening Narrative ===");
    println!("{}", output.narrative);
    println!("\n--- Options ---");
    for (i, opt) in output.options.iter().enumerate() {
        println!("{}. {}", i + 1, opt);
    }
}

/// Test opening narrative matches narrative style (女频).
#[tokio::test]
async fn test_opening_narrative_female_style() {
    let mut engine = make_engine();
    let choices = test_choices_female_buddhist();

    let (world_config, _tokens) = engine.generate_world(&choices).await
        .expect("world generation should succeed");
    engine.init_from_creation(&choices, world_config);

    let opening = xgamengine::prompt::builder::build_opening_prompt(
        &engine.world_config,
        &choices.player_name,
        &choices.dao_name,
        &choices.family_background,
        &choices.entry_method,
    );

    // Verify the system prompt includes female narrative style
    let messages = xgamengine::prompt::builder::build_messages(
        &engine.template_dir,
        &engine.state,
        &engine.window,
        &opening,
        &engine.world_config,
    ).expect("build_messages should succeed");

    let system_prompt = &messages[0].content;
    assert!(system_prompt.contains("女频"), 
        "system prompt should contain 女频 directive");
    assert!(system_prompt.contains("侧重人物关系"),
        "system prompt should contain female style description");

    let output = engine.start_game_ex(&opening).await
        .expect("opening narrative should succeed");

    println!("\n=== Female-Style Opening ===");
    println!("{}", output.narrative);
}

/// Test that the engine correctly handles a mentor-less character (散修独行).
#[tokio::test]
async fn test_rogue_cultivator_no_mentor() {
    let mut engine = make_engine();
    let choices = CreationChoices {
        family_background: "乞儿流浪".into(),
        childhood_experience: "市井谋生".into(),
        sect_category: "散修独行".into(),
        join_reason: "机缘巧合，误打误撞".into(),
        entry_method: "落崖奇遇，得遇洞府".into(),
        demonic_stance: "魔道亦有可取之处，手段不重要，结果才重要".into(),
        personality_archetype: "白小纯".into(),
        core_value: "逍遥自在，不受束缚".into(),
        altruism: "救醒他，了解情况后再决定。".into(),
        dao_quest: "只为自由，不受束缚".into(),
        player_name: "云无痕".into(),
        dao_name: "".into(),
        narrative_style: "male".into(),
    };

    let (world_config, _tokens) = engine.generate_world(&choices).await
        .expect("world generation should succeed");
    engine.init_from_creation(&choices, world_config);

    // Verify no relationships
    println!("\n=== Rogue Cultivator ===");
    println!("Sect: {}", engine.state.sect);
    println!("NPC: {}", engine.world_config.key_npc_name);
    println!("Location: {}", engine.state.current_location);
    println!("Relationships: {:?}", engine.state.relationships);

    // Rogue should have no default-qingxu relationships
    if engine.world_config.key_npc_name == "无" || engine.world_config.key_npc_name.is_empty() {
        assert!(engine.state.relationships.is_empty(),
            "rogue with no mentor should have empty relationships, got {:?}", 
            engine.state.relationships);
    }

    let opening = xgamengine::prompt::builder::build_opening_prompt(
        &engine.world_config,
        &choices.player_name,
        &choices.dao_name,
        &choices.family_background,
        &choices.entry_method,
    );

    let output = engine.start_game_ex(&opening).await
        .expect("opening narrative should succeed");

    assert!(!output.narrative.is_empty());
    println!("Narrative: {}", output.narrative);
}
