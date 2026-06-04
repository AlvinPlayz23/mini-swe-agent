<div align="center">
<a href="https://mini-swe-agent.com/latest/"><img src="https://github.com/SWE-agent/mini-swe-agent/raw/main/docs/assets/mini-swe-agent-banner.svg" alt="mini-swe-agent banner" style="height: 7em"/></a>
</div>

# mini-swe-agent (Rust Port)

This is a Rust implementation of the minimal AI software engineering agent, featuring a TUI and streaming OpenAI support.

## Features
- **Rust Port**: High-performance, type-safe implementation.
- **Streaming**: Real-time response streaming using `async-openai`.
- **TUI**: Interactive terminal interface built with `ratatui`.
- **Custom Tools**: Support for bash execution and state reset via text-based protocols.

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (latest stable version)
- An OpenAI API Key

### Installation & Running
1. Set your OpenAI API key:
   ```bash
   export OPENAI_API_KEY='your-api-key-here'
   ```
2. (Optional) Set the model name (defaults to `gpt-4o`):
   ```bash
   export MSWEA_MODEL_NAME='gpt-4o'
   ```
3. (Optional) Set a custom base URL:
   ```bash
   export OPENAI_BASEURL='https://your-proxy-endpoint/v1'
   ```
4. Run the application:
   ```bash
   cargo run
   ```

## TUI Shortcuts
- **Enter**: Submit task / input (only when not busy).
- **Up/Down Arrows**: Scroll transcript manually.
- **Ctrl-F**: Toggle "Follow" (auto-scroll to bottom as text streams).
- **Esc**: Exit the application.

## Custom Tools
The agent uses text-based markers to interact with the system:
- **Bash Execution**: Wrap commands in ` ```bash ... ``` ` blocks.
- **Reset State**: Use the keyword `[[RESET]]` to clear history.

---

*Original Python version documentation can be found in the repository history.*
