#[cfg(test)]
mod tests {
    use crate::model::OpenAIModel;
    use crate::model::Action;

    #[test]
    fn test_parse_actions() {
        let model = OpenAIModel::new(None);

        let content = "I will list the files.\n```bash\nls -la\n```\nAnd then I will reset.\nRESET_AGENT_STATE";
        let actions = model.parse_actions(content);

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::Bash("ls -la".to_string()));
        assert_eq!(actions[1], Action::Reset);
    }

    #[test]
    fn test_parse_multiple_bash() {
        let model = OpenAIModel::new(None);
        let content = "```bash\necho 1\n```\n```bash\necho 2\n```";
        let actions = model.parse_actions(content);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::Bash("echo 1".to_string()));
        assert_eq!(actions[1], Action::Bash("echo 2".to_string()));
    }
}
