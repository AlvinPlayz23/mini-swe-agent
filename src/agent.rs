use crate::env::{LocalEnvironment, ExecutionResult};
use crate::model::{OpenAIModel, Message, MessageRole, Action};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AgentEvent {
    TextDelta(String),
    ActionStarted(Action),
    ActionFinished(Action, ExecutionResult),
    ConversationReset,
    Error(String),
    TaskFinished,
}

pub struct Agent {
    model: OpenAIModel,
    env: LocalEnvironment,
    messages: Arc<Mutex<Vec<Message>>>,
    system_prompt: String,
}

impl Agent {
    pub fn new(model: OpenAIModel, env: LocalEnvironment) -> Self {
        let system_prompt = "You are a helpful AI software engineer. \
            You can use bash commands by wrapping them in ```bash code blocks. \
            To reset the conversation, use the command [[RESET]]. \
            Always explain your reasoning before executing a command."
            .to_string();

        Self {
            model,
            env,
            messages: Arc::new(Mutex::new(vec![Message {
                role: MessageRole::System,
                content: system_prompt.clone(),
            }])),
            system_prompt,
        }
    }

    pub async fn run_task(&self, task: String, event_tx: mpsc::UnboundedSender<AgentEvent>) {
        {
            let mut msgs = self.messages.lock().unwrap();
            msgs.push(Message {
                role: MessageRole::User,
                content: task,
            });
        }

        loop {
            let messages = {
                let msgs = self.messages.lock().unwrap();
                msgs.clone()
            };

            let mut stream = match self.model.query_stream(messages).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = event_tx.send(AgentEvent::Error(format!("Model error: {}", e)));
                    break;
                }
            };

            let mut full_content = String::new();
            let mut stream_error = false;
            while let Some(delta) = stream.next().await {
                match delta {
                    Ok(text) => {
                        full_content.push_str(&text);
                        let _ = event_tx.send(AgentEvent::TextDelta(text));
                    }
                    Err(e) => {
                        let _ = event_tx.send(AgentEvent::Error(format!("Stream error: {}", e)));
                        stream_error = true;
                        break;
                    }
                }
            }

            if stream_error {
                break;
            }

            {
                let mut msgs = self.messages.lock().unwrap();
                msgs.push(Message {
                    role: MessageRole::Assistant,
                    content: full_content.clone(),
                });
            }

            let actions = self.model.parse_actions(&full_content);
            if actions.is_empty() {
                break;
            }

            let mut should_reset = false;
            for action in actions {
                if action == Action::Reset {
                    should_reset = true;
                    let _ = event_tx.send(AgentEvent::ActionStarted(action));
                    break;
                }

                let _ = event_tx.send(AgentEvent::ActionStarted(action.clone()));
                if let Action::Bash(command) = &action {
                    let result = self.env.execute(command);

                    let observation = format!(
                        "Command: {}\nReturn code: {}\nOutput:\n{}",
                        command, result.returncode, result.output
                    );

                    {
                        let mut msgs = self.messages.lock().unwrap();
                        msgs.push(Message {
                            role: MessageRole::User,
                            content: observation,
                        });
                    }

                    let _ = event_tx.send(AgentEvent::ActionFinished(action, result));
                }
            }

            if should_reset {
                let mut msgs = self.messages.lock().unwrap();
                msgs.clear();
                msgs.push(Message {
                    role: MessageRole::System,
                    content: self.system_prompt.clone(),
                });
                let _ = event_tx.send(AgentEvent::ConversationReset);
                break;
            }
        }
        let _ = event_tx.send(AgentEvent::TaskFinished);
    }
}
