use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, List, ListItem, Wrap},
    Frame,
};
use tui_textarea::TextArea;
use crate::engine::EngineOutput;
use crate::state::GameState;

pub fn render_ui(frame: &mut Frame, app: &AppState) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(68),
            Constraint::Percentage(32),
        ])
        .split(frame.area());

    // Left: narrative area
    render_narrative(frame, main_layout[0], app);

    // Right: system panel
    render_panel(frame, main_layout[1], &app.state);
}

fn render_narrative(frame: &mut Frame, area: Rect, app: &AppState) {
    let narrative_block = Block::default()
        .title(" 修仙录 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let text = match &app.narrative {
        Some(n) => n.clone(),
        None => "天道玉简微震，正在推演天机...".into(),
    };

    let paragraph = Paragraph::new(text)
        .block(narrative_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_panel(frame: &mut Frame, area: Rect, state: &GameState) {
    let mut lines: Vec<Line> = Vec::new();

    // Realm
    lines.push(Line::from(vec![
        Span::styled(
            format!("境界: {}", state.realm),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(format!("进度: {:.0}%", state.realm_progress * 100.0)));

    // Progress bar
    let bar_width: usize = 20;
    let filled = (state.realm_progress * bar_width as f32) as usize;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width.saturating_sub(filled)));
    lines.push(Line::from(Span::styled(bar, Style::default().fg(Color::Green))));

    // Qi
    lines.push(Line::from(Span::styled(
        format!("灵力: {}/{}", state.qi, state.max_qi),
        Style::default().fg(Color::Yellow),
    )));

    // Spirit stones
    lines.push(Line::from(Span::styled(
        format!("灵石: {}", state.spirit_stones),
        Style::default().fg(Color::Yellow),
    )));

    lines.push(Line::from(""));

    // Stats — 六维
    lines.push(Line::from("六维属性:"));
    lines.push(Line::from(Span::styled(
        format!("  物攻{}  法攻{}", state.stats.physical_attack, state.stats.magical_attack),
        Style::default().fg(Color::Red),
    )));
    lines.push(Line::from(Span::styled(
        format!("  物防{}  法防{}", state.stats.physical_defense, state.stats.magical_defense),
        Style::default().fg(Color::Blue),
    )));
    lines.push(Line::from(Span::styled(
        format!("  神识攻{}  神识防{}", state.stats.divine_attack, state.stats.divine_defense),
        Style::default().fg(Color::Magenta),
    )));

    // Techniques
    if !state.techniques.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("功法:"));
        for t in &state.techniques {
            lines.push(Line::from(Span::styled(
                format!("  • {} ({})", t.name, t.tier),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
    }

    // Inventory
    if !state.inventory.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("物品:"));
        for item in &state.inventory {
            lines.push(Line::from(Span::styled(
                format!("  • {} x{}", item.name, item.quantity),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
    }

    // Relationships
    if !state.relationships.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from("人物:"));
        for r in &state.relationships {
            lines.push(Line::from(Span::styled(
                format!("  • {} ({}) 好感:{}", r.name, r.role, r.affinity),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
    }

    let panel_block = Block::default()
        .title(" 天道玉简 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let panel = Paragraph::new(Text::from(lines))
        .block(panel_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(panel, area);
}

pub fn render_options(frame: &mut Frame, area: Rect, meta_text: &Option<String>, options: &[String], selected: usize) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(options.len() as u16 + 2),
        ])
        .split(area);

    // Meta text
    if let Some(ref mt) = meta_text {
        let mt_para = Paragraph::new(mt.as_str())
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(mt_para, inner[0]);
    }

    // Options
    let items: Vec<ListItem> = options.iter().enumerate().map(|(i, opt)| {
        if i == selected {
            ListItem::new(Line::from(Span::styled(
                format!(" ▶ {}. {}", i + 1, opt),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )))
        } else {
            ListItem::new(Line::from(Span::styled(
                format!("   {}. {}", i + 1, opt),
                Style::default().add_modifier(Modifier::DIM),
            )))
        }
    }).collect();

    let list = List::new(items);
    frame.render_widget(list, inner[1]);
}

pub fn render_prompt(frame: &mut Frame, area: Rect, app: &AppState) {
    let text = match app.mode {
        AppMode::CustomInput => "输入模式: Enter 提交  Esc 取消".to_string(),
        _ => "1-4 选择  5/`:` 自由输入  j/k 上下  Enter 确认  q 退出".to_string(),
    };
    let prompt = Paragraph::new(text)
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(prompt, area);
}

/// Render the inline text input area
pub fn render_input(frame: &mut Frame, area: Rect, app: &AppState) {
    frame.render_widget(&app.textarea, area);
}

/// Render a loading spinner
pub fn render_spinner(frame: &mut Frame, area: Rect, app: &AppState) {
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let c = spinner_chars[app.spinner_frame % spinner_chars.len()];
    let text = format!("{} 天道玉简微震，推演天机...", c);
    let para = Paragraph::new(text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM));
    frame.render_widget(para, area);
}

/// Spin the spinner forward one frame
pub fn tick_spinner(app: &mut AppState) {
    app.spinner_frame = app.spinner_frame.wrapping_add(1);
}

/// Application state
pub struct AppState {
    pub state: GameState,
    pub narrative: Option<String>,
    pub meta_text: Option<String>,
    pub options: Vec<String>,
    pub selected: usize,
    pub scene_type: Option<String>,
    pub status_message: String,
    pub mode: AppMode,
    pub textarea: TextArea<'static>,
    pub spinner_frame: usize,
}

#[derive(PartialEq)]
pub enum AppMode {
    Loading,
    Displaying,
    Selecting,
    CustomInput,
    Quit,
}

impl AppState {
    pub fn new(state: GameState) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().title(" 输入行动 ").borders(Borders::ALL));
        textarea.set_placeholder_text("输入你的行动，Enter 提交，Esc 取消");
        Self {
            state,
            narrative: None,
            meta_text: None,
            options: vec![],
            selected: 0,
            scene_type: None,
            status_message: "正在推演天机...".into(),
            mode: AppMode::Loading,
            textarea,
            spinner_frame: 0,
        }
    }

    pub fn set_output(&mut self, output: EngineOutput) {
        self.narrative = Some(output.narrative);
        self.meta_text = output.meta_text;
        self.options = output.options;
        self.scene_type = output.scene_type;
        self.selected = 0;
        self.mode = AppMode::Selecting;
    }

    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len().min(4);
        }
    }

    pub fn select_prev(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + self.options.len().min(4) - 1) % self.options.len().min(4);
        }
    }

    pub fn selected_option(&self) -> Option<String> {
        self.options.get(self.selected).cloned()
    }
}
