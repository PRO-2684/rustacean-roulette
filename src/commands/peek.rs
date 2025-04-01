use super::{Command, Roulette};
use frankenstein::{client_reqwest::Bot, types::Message};
use tokio::sync::Mutex;

/// Peek the left-over chambers, acquiring count of filled and left chambers.
pub struct PeekCommand;

impl Command for PeekCommand {
    const TRIGGER: &'static str = "peek";
    const HELP: &'static str =
        "Peek the left-over chambers, acquiring count of filled and left chambers.";
    async fn execute(
        _bot: &Bot,
        _msg: Message,
        roulette: &Mutex<Roulette>,
    ) -> Option<String> {
        // Peek the roulette
        let roulette = roulette.lock().await;
        let (filled, left) = roulette.peek();
        // Respond with the result
        let response = format!(
            "You stole a quick glimpse at the revolver... There're {filled} filled chambers, out of {left} left-over chambers."
        );
        Some(response)
    }
}
