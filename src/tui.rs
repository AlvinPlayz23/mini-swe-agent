use crate::agent::{Agent, AgentEvent};
use crate::env::LocalEnvironment;
use crate::model::OpenAIModel;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

pub struct TuiApp<'a> {
    pub textarea: TextArea<'a>,
    pub transcript: Vec<String>,
    pub agent: Arc<Agent>,
    pub current_agent_text: String,
    pub scroll: u16,
    pub auto_scroll: bool,
    pub is_busy: bool,
}

impl<'a> TuiApp<'a> {
    pub fn new(agent: Arc<Agent>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Enter task here...");
        textarea.set_block(Block::default().borders(Borders::ALL).title("> Input (Enter to send, Esc to quit)"));

        Self {
            textarea,
            transcript: Vec::new(),
            agent,
            current_agent_text: String::new(),
            scroll: 0,
            auto_scroll: true,
            is_busy: false,
        }
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
    }
}

pub async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = TerminalGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let model_name = std::env::var("MSWEA_MODEL_NAME").ok();
    let model = OpenAIModel::new(model_name);
    let env = LocalEnvironment::new(None);
    let agent = Arc::new(Agent::new(model, env));

    let mut app = TuiApp::new(agent);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(10))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Enter if !app.is_busy => {
                            let task = app.textarea.lines().join("\n");
                            if !task.is_empty() {
                                app.is_busy = true;
                                app.transcript.push(format!("User: {}", task));
                                app.textarea = TextArea::default();
                                app.textarea.set_placeholder_text("Waiting for agent...");
                                app.textarea.set_block(Block::default().borders(Borders::ALL).title("> Input (Enter to send, Esc to quit)"));
                                app.auto_scroll = true;

                                let agent_clone = Arc::clone(&app.agent);
                                let tx_clone = event_tx.clone();
                                tokio::spawn(async move {
                                    agent_clone.run_task(task, tx_clone).await;
                                });
                            }
                        }
                        KeyCode::Up => {
                            app.scroll = app.scroll.saturating_sub(1);
                            app.auto_scroll = false;
                        }
                        KeyCode::Down => {
                            app.scroll = app.scroll.saturating_add(1);
                            app.auto_scroll = false;
                        }
                        KeyCode::Char('f') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                             app.auto_scroll = true;
                        }
                        _ if !app.is_busy => {
                            app.textarea.input(key);
                        }
                        _ => {}
                    }
                }
                _ if !app.is_busy => {
                    app.textarea.input(ev);
                }
                _ => {}
            }
        }

        while let Ok(event) = event_rx.try_recv() {
            match event {
                AgentEvent::TextDelta(delta) => {
                    app.current_agent_text.push_str(&delta);
                }
                AgentEvent::ActionStarted(action) => {
                    if !app.current_agent_text.is_empty() {
                        app.transcript.push(format!("Agent: {}", app.current_agent_text));
                        app.current_agent_text = String::new();
                    }
                    app.transcript.push(format!("Action: {:?}", action));
                }
                AgentEvent::ActionFinished(_action, result) => {
                    app.transcript.push(format!("Result: (Code {})", result.returncode));
                    if !result.output.is_empty() {
                        app.transcript.push(format!("Output: {}", result.output));
                    }
                }
                AgentEvent::ConversationReset => {
                    app.transcript.push("--- Conversation Reset ---".to_string());
                    app.current_agent_text = String::new();
                }
                AgentEvent::Error(e) => {
                    app.transcript.push(format!("Error: {}", e));
                }
                AgentEvent::TaskFinished => {
                    if !app.current_agent_text.is_empty() {
                        app.transcript.push(format!("Agent: {}", app.current_agent_text));
                        app.current_agent_text = String::new();
                    }
                    app.is_busy = false;
                }
            }
            if app.auto_scroll {
                 let total_lines = app.transcript.len() + if app.current_agent_text.is_empty() { 0 } else { 1 };
                 // Estimate lines based on typical width, though ratatui Wrap handles this dynamically.
                 // This is a simple heuristic.
                 if total_lines > 10 {
                     app.scroll = (total_lines as u16).saturating_sub(10);
                 }
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(10),
                Constraint::Length(5),
            ]
            .as_ref(),
        )
        .split(f.area());

    let mut transcript_lines = app.transcript.clone();
    if !app.current_agent_text.is_empty() {
        transcript_lines.push(format!("Agent: {}", app.current_agent_text));
    }

    let transcript_text = transcript_lines.join("\n");
    let transcript = Paragraph::new(transcript_text)
        .block(Block::default().borders(Borders::ALL).title("Transcript (Up/Down to scroll, Ctrl-F to follow)"))
        .wrap(Wrap { trim: true })
        .scroll((app.scroll, 0));
    f.render_widget(transcript, chunks[0]);

    f.render_widget(&app.textarea, chunks[1]);
}
