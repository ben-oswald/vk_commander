use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_commands_dir() -> PathBuf {
    if Path::new("/.flatpak-info").exists() {
        PathBuf::from("/app/share/valkey_insight/commands")
    } else if !cfg!(debug_assertions) {
        PathBuf::from("/usr/share/valkey_insight/commands/")
    } else {
        PathBuf::from("commands")
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub full_name: String,
    pub summary: String,
    pub arguments_desc: String,
}

#[derive(Debug, Deserialize)]
struct CommandFile {
    #[serde(flatten)]
    commands: HashMap<String, CommandDefinition>,
}

#[derive(Debug, Deserialize)]
struct CommandDefinition {
    summary: Option<String>,
    container: Option<String>,
    #[serde(default)]
    arguments: Vec<Argument>,
}

#[derive(Debug, Deserialize)]
struct Argument {
    name: String,
    token: Option<String>,
    #[serde(default)]
    arguments: Vec<Argument>,
}

pub struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn load_from_directory<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let mut registry = Self::new();
        let dir_path = path.as_ref();

        if !dir_path.exists() {
            return Ok(registry);
        }

        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && let Ok(content) = fs::read_to_string(&path)
                && let Some(command) = Self::parse_command_json(&content)
            {
                registry.commands.push(command);
            }
        }

        registry
            .commands
            .sort_by(|a, b| a.full_name.cmp(&b.full_name));

        Ok(registry)
    }

    fn parse_command_json(json: &str) -> Option<Command> {
        let command_file: CommandFile = serde_json::from_str(json).ok()?;

        let (command_name, definition) = command_file.commands.into_iter().next()?;

        let summary = definition.summary.unwrap_or_default();

        let full_name = if let Some(container) = definition.container {
            format!(
                "{} {}",
                container.to_uppercase(),
                command_name.to_uppercase()
            )
        } else {
            command_name.to_uppercase()
        };

        let arguments_desc = Self::format_arguments(&definition.arguments);

        Some(Command {
            full_name,
            summary,
            arguments_desc,
        })
    }

    fn format_arguments(arguments: &[Argument]) -> String {
        let mut result = Vec::new();
        Self::collect_argument_tokens(arguments, &mut result);
        result.join(" ")
    }

    fn collect_argument_tokens(arguments: &[Argument], result: &mut Vec<String>) {
        for arg in arguments {
            if let Some(token) = &arg.token {
                result.push(token.clone());
            } else {
                result.push(format!("<{}>", arg.name));
            }

            if !arg.arguments.is_empty() {
                Self::collect_argument_tokens(&arg.arguments, result);
            }
        }
    }

    pub fn get_suggestions(&self, input: &str) -> Vec<Command> {
        if input.is_empty() {
            return Vec::new();
        }

        let input_upper = input.to_uppercase();
        let mut suggestions: Vec<Command> = self
            .commands
            .iter()
            .filter(|cmd| cmd.full_name.starts_with(&input_upper))
            .cloned()
            .collect();

        suggestions.truncate(10);

        suggestions
    }

    pub fn get_all_commands(&self) -> &[Command] {
        &self.commands
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
