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
}

impl<'a> TuiApp<'a> {
    pub fn new(agent: Arc<Agent>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Enter task here...");
        textarea.set_block(Block::default().borders(Borders::ALL).title("Input"));

        Self {
            textarea,
            transcript: Vec::new(),
            agent,
        }
    }
}

pub async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let model_name = std::env::var("MSWEA_MODEL_NAME").ok();
    let model = OpenAIModel::new(model_name);
    let env = LocalEnvironment::new(None);
    let agent = Arc::new(Agent::new(model, env));

    let mut app = TuiApp::new(agent);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let mut current_agent_text = String::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(10))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Enter => {
                            let task = app.textarea.lines().join("\n");
                            if !task.is_empty() {
                                app.transcript.push(format!("User: {}", task));
                                app.textarea = TextArea::default();
                                app.textarea.set_placeholder_text("Waiting for agent...");

                                let agent_clone = Arc::clone(&app.agent);
                                let tx_clone = event_tx.clone();
                                tokio::spawn(async move {
                                    agent_clone.run_task(task, tx_clone).await;
                                });
                            }
                        }
                        _ => {
                            app.textarea.input(key);
                        }
                    }
                }
                _ => {
                    app.textarea.input(ev);
                }
            }
        }

        while let Ok(event) = event_rx.try_recv() {
            match event {
                AgentEvent::TextDelta(delta) => {
                    current_agent_text.push_str(&delta);
                }
                AgentEvent::ActionStarted(action) => {
                    if !current_agent_text.is_empty() {
                        app.transcript.push(format!("Agent: {}", current_agent_text));
                        current_agent_text = String::new();
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
                    current_agent_text = String::new();
                }
                AgentEvent::Error(e) => {
                    app.transcript.push(format!("Error: {}", e));
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

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

    let transcript_text = app.transcript.join("\n");
    let transcript = Paragraph::new(transcript_text)
        .block(Block::default().borders(Borders::ALL).title("Transcript"))
        .wrap(Wrap { trim: true });
    f.render_widget(transcript, chunks[0]);

    let input_block = Block::default().borders(Borders::ALL).title("Input (Enter to send, Esc to quit)");
    app.textarea.set_block(input_block);
    // Requirement: with a ">" in the textarea or textpromptbox thingy
    // We can use the block's title or just leave it.
    // To have it INSIDE the textarea, we can use a custom prefix if supported or just add it to the textarea lines.
    // TextArea doesn't have an easy "prefix" for every line.
    // Let's change the title to include ">"
    app.textarea.set_block(Block::default().borders(Borders::ALL).title("> Input (Enter to send, Esc to quit)"));

    f.render_widget(&app.textarea, chunks[1]);
}
