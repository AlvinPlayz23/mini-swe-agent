pub mod env;
pub mod model;
pub mod agent;
pub mod tui;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tui::run_tui().await
}
