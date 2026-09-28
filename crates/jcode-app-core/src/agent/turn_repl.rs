use super::*;
use crate::{terminal_eprintln as eprintln, terminal_println as println};

impl Agent {
    pub async fn run_once_capture(&mut self, user_message: &str) -> Result<String> {
        self.run_once_capture_with_display_role(user_message, None)
            .await
    }

    pub(crate) async fn run_once_capture_with_display_role(
        &mut self,
        user_message: &str,
        display_role: Option<crate::session::StoredDisplayRole>,
    ) -> Result<String> {
        self.announce_late_mcp_tools().await;
        let input_id = self.add_message_with_display_role(
            Role::User,
            vec![ContentBlock::Text {
                text: user_message.to_string(),
                cache_control: None,
            }],
            display_role,
        );
        if !user_message.trim().is_empty() {
            self.begin_model_usage_turn(&input_id);
        }
        self.session.save()?;
        if trace_enabled() {
            eprintln!("[trace] session_id {}", self.session.id);
        }
        self.run_turn(false).await
    }

    /// Start an interactive REPL
    pub async fn repl(&mut self) -> Result<()> {
        println!("J-Code - Coding Agent");
        println!("Type your message, or 'quit' to exit.");

        // Show available skills
        let skills = self.current_skills_snapshot();
        let skill_list = skills.list();
        if !skill_list.is_empty() {
            println!(
                "Available skills: {}",
                skill_list
                    .iter()
                    .map(|s| format!("/{}", s.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!();

        loop {
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let input = input.trim();
            if input.is_empty() {
                continue;
            }

            if input == "quit" || input == "exit" {
                break;
            }

            if input == "clear" {
                self.clear();
                println!("Conversation cleared.");
                continue;
            }

            // Check for skill invocation. Resolve against the registry (not
            // the bare tokenizer) so a `SKILL.md` `name:` field containing
            // spaces, e.g. "My Custom Skill", can still be matched: the
            // bare parse always stops at the first whitespace.
            if let Some(invocation) = skills.resolve_invocation(input) {
                if let Some(skill) = skills.get(invocation.name) {
                    println!("Activating skill: {}", skill.name);
                    println!("{}\n", skill.description);
                    self.active_skill = Some(invocation.name.to_string());
                    if let Some(prompt) = invocation.prompt {
                        if let Err(e) = self.run_once(prompt).await {
                            eprintln!("\nError: {}\n", e);
                        }
                        println!();
                    }
                    continue;
                } else {
                    println!("Unknown skill: /{}", invocation.name);
                    println!(
                        "Available: {}",
                        skills
                            .list()
                            .iter()
                            .map(|s| format!("/{}", s.name))
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    continue;
                }
            }

            if let Err(e) = self.run_once(input).await {
                eprintln!("\nError: {}\n", e);
            }

            println!();
        }

        // Extract memories from session before exiting
        self.extract_session_memories().await;

        Ok(())
    }
}
