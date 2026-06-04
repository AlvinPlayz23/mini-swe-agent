use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    Bash(String),
    Reset,
}

pub struct OpenAIModel {
    client: Option<Client<async_openai::config::OpenAIConfig>>,
    model_name: String,
}

impl OpenAIModel {
    pub fn new(model_name: Option<String>) -> Self {
        let api_key = env::var("OPENAI_API_KEY").ok();
        let client = api_key.map(|key| {
            let mut config = async_openai::config::OpenAIConfig::new().with_api_key(key);
            if let Ok(base_url) = env::var("OPENAI_BASEURL") {
                config = config.with_api_base(base_url);
            }
            Client::with_config(config)
        });

        Self {
            client,
            model_name: model_name.unwrap_or_else(|| "gpt-4o".to_string()),
        }
    }

    pub async fn query_stream(
        &self,
        messages: Vec<Message>,
    ) -> anyhow::Result<impl futures::Stream<Item = anyhow::Result<String>>> {
        let client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY not set"))?;

        let chat_messages: Vec<ChatCompletionRequestMessage> = messages
            .into_iter()
            .map(|m| match m.role {
                MessageRole::System => ChatCompletionRequestSystemMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(),
                MessageRole::User => ChatCompletionRequestUserMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(),
                MessageRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(),
                MessageRole::Tool => ChatCompletionRequestUserMessageArgs::default()
                    .content(m.content)
                    .build()
                    .unwrap()
                    .into(), // Simplified
            })
            .collect();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .messages(chat_messages)
            .stream(true)
            .build()?;

        let stream = client.chat().create_stream(request).await?;

        Ok(stream.map(|res| match res {
            Ok(response) => {
                let content = response
                    .choices
                    .iter()
                    .filter_map(|c| c.delta.content.clone())
                    .collect::<Vec<_>>()
                    .join("");
                Ok(content)
            }
            Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
        }))
    }

    pub fn parse_actions(&self, content: &str) -> Vec<Action> {
        let mut actions = Vec::new();

        let mut start_index = 0;
        while start_index < content.len() {
            let next_bash = content[start_index..].find("```bash");
            let next_reset = content[start_index..].find("[[RESET]]");

            match (next_bash, next_reset) {
                (Some(b), Some(r)) if b < r => {
                    let actual_start = start_index + b + 7;
                    if let Some(bash_end) = content[actual_start..].find("```") {
                        let command = content[actual_start..actual_start + bash_end].trim().to_string();
                        if !command.is_empty() {
                            actions.push(Action::Bash(command));
                        }
                        start_index = actual_start + bash_end + 3;
                    } else {
                        start_index = actual_start;
                    }
                }
                (Some(b), None) => {
                    let actual_start = start_index + b + 7;
                    if let Some(bash_end) = content[actual_start..].find("```") {
                        let command = content[actual_start..actual_start + bash_end].trim().to_string();
                        if !command.is_empty() {
                            actions.push(Action::Bash(command));
                        }
                        start_index = actual_start + bash_end + 3;
                    } else {
                        start_index = actual_start;
                    }
                }
                (Some(_), Some(r)) | (None, Some(r)) => {
                    actions.push(Action::Reset);
                    start_index += r + "[[RESET]]".len();
                }
                (None, None) => break,
            }
        }

        actions
    }
}
