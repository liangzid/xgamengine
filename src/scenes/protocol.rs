/// Scene type keywords
pub const SCENE_TYPES: &[&str] = &["cultivation", "combat", "debate", "exploration", "trade"];

/// Get scene-specific prompt suffix
pub fn scene_prompt_suffix(scene_type: &str) -> &'static str {
    match scene_type {
        "cultivation" => "当前为修炼场景。请侧重描写灵气运转、境界感悟与瓶颈突破。",
        "combat" => "当前为战斗场景。请侧重招式描述、攻防转换、灵力消耗与胜负判定。",
        "debate" => "当前为辩论场景。请侧重话术交锋、道心碰撞、逻辑辩证。",
        "exploration" => "当前为探索场景。请侧重环境描写、未知发现、机缘与陷阱。",
        "trade" => "当前为交易场景。请侧重价格博弈、物品鉴定、机缘巧合。",
        _ => "",
    }
}
