use crate::errors::Error;
use crate::i18n::Language;
use crate::utils::PathProvider;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::RwLock;

const SETTINGS_FILE_NAMES: [&str; 2] = ["settings.vks", "server.vks"];

#[derive(Default)]
pub struct AppSettings {
    settings: RwLock<HashMap<String, String>>,
    servers: RwLock<HashMap<String, String>>,
}

impl AppSettings {
    pub fn new() -> Self {
        AppSettings {
            settings: HashMap::new().into(),
            servers: HashMap::new().into(),
        }
    }

    pub fn new_from_file() -> Self {
        let settings = Self::default();
        let _ = settings.load_from_file();
        settings
    }

    pub fn load_from_file(&self) -> Result<(), Error> {
        self.settings.write()?.clear();
        self.servers.write()?.clear();

        let config_path = PathProvider::get_config_path()?;

        for file_name in SETTINGS_FILE_NAMES.iter() {
            let mut path = config_path.clone();
            path.push(*file_name);
            if !fs::exists(&path)? {
                continue;
            }
            let file = File::open(path)?;
            let reader = BufReader::new(file);

            for line_result in reader.lines() {
                let line = line_result?;
                if line.trim().is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim().to_string();
                    let value = line[eq_pos + 1..].trim().to_string();
                    if *file_name == "settings.vks" {
                        self.settings.write()?.insert(key, value);
                    } else if *file_name == "server.vks" {
                        self.servers.write()?.insert(key, value);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn save_to_file(&self) -> Result<(), Error> {
        let config_path = PathProvider::get_config_path()?;

        for file_name in SETTINGS_FILE_NAMES.iter() {
            let mut path = config_path.clone();
            path.push(*file_name);

            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)?;

            if *file_name == "settings.vks" {
                for (key, value) in self.settings.read()?.iter() {
                    writeln!(file, "{key}={value}")?;
                }
            } else if *file_name == "server.vks" {
                for (key, value) in self.servers.read()?.iter() {
                    writeln!(file, "{key}={value}")?;
                }
            }
        }
        Ok(())
    }

    pub fn get_settings_value(&self, key: &str, default_value: &str) -> String {
        match self.settings.read() {
            Ok(s) => s
                .get(key)
                .cloned()
                .unwrap_or_else(|| default_value.to_string()),
            Err(_) => default_value.to_string(),
        }
    }

    pub fn set_settings_value(&self, key: &str, value: &str) -> Result<(), Box<Error>> {
        self.settings
            .write()?
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub fn delete_settings_files(&self) -> Result<(), Box<Error>> {
        for file_name in SETTINGS_FILE_NAMES.iter() {
            let config_path = PathProvider::get_config_path()?;
            let mut path = config_path.clone();
            path.push(*file_name);
            if fs::exists(&path)? {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    pub fn get_language(&self) -> Language {
        Language::from(self.get_settings_value("language", "english"))
    }

    pub fn set_language(&self, language: &Language) -> Result<(), Box<Error>> {
        self.set_settings_value("language", &language.to_string().to_lowercase())
    }

    pub fn get_theme(&self) -> String {
        self.get_settings_value("theme", "system")
    }

    pub fn set_theme(&self, theme: &str) -> Result<(), Box<Error>> {
        self.set_settings_value("theme", theme)
    }

    pub fn add_server(&self, alias: &str, url: &str) -> Result<(), Box<Error>> {
        if !self.servers.read()?.contains_key(alias) {
            self.servers
                .write()?
                .insert(alias.to_owned(), url.to_owned());
        }
        Ok(())
    }

    pub fn get_servers(&self) -> Result<HashMap<String, String>, Box<Error>> {
        Ok(self.servers.read()?.clone())
    }

    pub fn update_server(&self, alias: &str, url: &str) -> Result<(), Box<Error>> {
        if self.servers.read()?.contains_key(alias) {
            self.servers
                .write()?
                .insert(alias.to_owned(), url.to_owned());
            self.save_to_file()?;
        }
        Ok(())
    }

    pub fn delete_server(&self, alias: &str) -> Result<(), Box<Error>> {
        self.servers.write()?.remove(alias);
        self.save_to_file()?;
        Ok(())
    }
}

#[test]
fn load_settings_from_file_test() {
    let settings = AppSettings::new();
    settings
        .delete_settings_files()
        .expect("Failed to delete settings files.");
    settings.load_from_file().expect("Failed to load settings.");

    let enabled = settings.get_settings_value("enabled", "false");
    assert_eq!(enabled, "false");

    settings
        .set_settings_value("enabled", "true")
        .expect("Failed to set settings.");
    settings.save_to_file().expect("Failed to save settings.");
    settings.load_from_file().expect("Failed to load settings.");
    let enabled = settings.get_settings_value("enabled", "false");
    assert_eq!(enabled, "true");

    let servers = settings.get_servers();
    assert_eq!(servers.expect("Cannot get servers").len(), 0);
    settings
        .add_server("srv0", "127.0.0.1:1234")
        .expect("Failed to add server.");
    settings.save_to_file().expect("Failed to save settings.");
    settings.load_from_file().expect("Failed to load settings.");
    let servers = settings.get_servers().expect("Cannot get servers");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers.get("srv0"), Some(&"127.0.0.1:1234".to_string()));
}
