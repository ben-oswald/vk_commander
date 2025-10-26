use crate::errors::Error;
use crate::i18n::Language;
use crate::utils::{PathProvider, ValkeyUrl, ValkeyUrlBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const SETTINGS_FILE: &str = "settings.json";
const SERVER_FILE: &str = "servers.json";
const OLD_SETTINGS_FILE: &str = "settings.vks";
const OLD_SERVER_FILE: &str = "server.vks";

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum AppTheme {
    Dark,
    Light,
}

#[derive(Serialize, Deserialize, Clone)]
struct Server {
    database_alias: String,
    host: String,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    database_index: Option<usize>,
    connection_type: Option<String>,
    last_connection: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    language: Option<Language>,
    theme: Option<AppTheme>,
    servers: Vec<Server>,

    migrated: RwLock<bool>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Some(Language::English),
            theme: None, // None => system theme
            servers: vec![],
            migrated: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct SettingsFileContent {
    language: Option<Language>,
    theme: Option<AppTheme>,
}

impl AppSettings {
    pub fn new_from_file() -> Self {
        let settings = Self::default();
        let _ = settings.load_from_file();
        settings
    }

    pub fn load_from_file(&self) -> Result<(), Error> {
        let config_path = PathProvider::get_config_path()?;
        let mut needs_migration = false;

        let mut settings_path = config_path.clone();
        settings_path.push(SETTINGS_FILE);
        if !fs::exists(&settings_path)? {
            let mut old_path = config_path.clone();
            old_path.push(OLD_SETTINGS_FILE);
            if fs::exists(&old_path)? {
                needs_migration = true;
                self.migrate_settings_vks(&old_path, &settings_path)?;
            } else {
                self.write_settings_file(&settings_path, &SettingsFileContent::default())?;
            }
        }

        let mut servers_path = config_path.clone();
        servers_path.push(SERVER_FILE);
        if !fs::exists(&servers_path)? {
            let mut old_path = config_path.clone();
            old_path.push(OLD_SERVER_FILE);
            if fs::exists(&old_path)? {
                needs_migration = true;
                self.migrate_servers_vks(&old_path, &servers_path)?;
            } else {
                self.write_servers_file(&servers_path, &vec![])?;
            }
        }

        if needs_migration {
            self.remove_old_files(&config_path)?;
            *self.migrated.write()? = false;
        } else {
            let mut old_path = config_path.clone();
            old_path.push(OLD_SETTINGS_FILE);
            let old_settings_exists = fs::exists(&old_path)?;
            let mut old_path = config_path.clone();
            old_path.push(OLD_SERVER_FILE);
            let old_servers_exists = fs::exists(&old_path)?;
            if old_settings_exists || old_servers_exists {
                self.remove_old_files(&config_path)?;
            }
        }
        Ok(())
    }

    pub fn save_to_file(&self) -> Result<(), Error> {
        let config_path = PathProvider::get_config_path()?;
        if *self.migrated.read()? {
            self.remove_old_files(&config_path)?;
            *self.migrated.write()? = false;
        }
        Ok(())
    }

    #[cfg(test)]
    fn delete_settings_files(&self) -> Result<(), Box<Error>> {
        let config_path = PathProvider::get_config_path()?;

        for name in [
            SETTINGS_FILE,
            SERVER_FILE,
            OLD_SETTINGS_FILE,
            OLD_SERVER_FILE,
        ] {
            let mut path = config_path.clone();
            path.push(name);
            if fs::exists(&path)? {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    pub fn get_language(&self) -> Language {
        let content = self.read_settings_file().unwrap_or_default();
        content.language.unwrap_or(Language::English)
    }

    pub fn set_language(&self, language: &Language) -> Result<(), Box<Error>> {
        let mut content = self.read_settings_file().unwrap_or_default();
        content.language = Some(*language);
        let path = self.settings_path()?;
        self.write_settings_file(&path, &content)?;
        Ok(())
    }

    pub fn get_theme(&self) -> String {
        match self.read_settings_file().ok().and_then(|s| s.theme) {
            Some(AppTheme::Light) => "light".to_string(),
            Some(AppTheme::Dark) => "dark".to_string(),
            None => "system".to_string(),
        }
    }

    pub fn set_theme(&self, theme: &str) -> Result<(), Box<Error>> {
        let mut content = self.read_settings_file().unwrap_or_default();
        content.theme = match theme.to_lowercase().as_str() {
            "light" => Some(AppTheme::Light),
            "dark" => Some(AppTheme::Dark),
            _ => None, // system
        };
        let path = self.settings_path()?;
        self.write_settings_file(&path, &content)?;
        Ok(())
    }

    pub fn add_server(&self, alias: &str, url: &str) -> Result<(), Box<Error>> {
        let mut servers = self.read_servers_file().unwrap_or_default();
        let parsed = ValkeyUrl::parse_valkey_url(Some(alias), url)?;
        let entry = Self::server_from_parsed(alias, &parsed);
        if let Some(pos) = servers
            .iter()
            .position(|s| s.database_alias == entry.database_alias)
        {
            servers[pos] = entry;
        } else {
            servers.push(entry);
        }
        let path = self.servers_path()?;
        self.write_servers_file(&path, &servers)?;
        Ok(())
    }

    pub fn update_server(&self, alias: &str, url: &str) -> Result<(), Box<Error>> {
        self.add_server(alias, url)
    }

    pub fn delete_server(&self, alias: &str) -> Result<(), Box<Error>> {
        let mut servers = self.read_servers_file().unwrap_or_default();
        servers.retain(|s| s.database_alias != alias);
        let path = self.servers_path()?;
        self.write_servers_file(&path, &servers)?;
        Ok(())
    }

    pub fn get_servers(&self) -> Result<Vec<(String, String)>, Box<Error>> {
        let servers = self.read_servers_file().unwrap_or_default();
        let mut result = Vec::with_capacity(servers.len());
        for s in servers {
            let mut builder = ValkeyUrlBuilder::new()
                .connection_name(s.database_alias.clone())
                .host(s.host.clone());
            if let Some(p) = s.port {
                builder = builder.port(p);
            }
            if let Some(u) = s.username {
                builder = builder.username(u);
            }
            if let Some(pw) = s.password {
                builder = builder.password(pw);
            }
            if let Some(db) = s.database_index {
                builder = builder.db(db as u32);
            }
            let valkey_url = builder.build()?;
            let mut conn = valkey_url.connection_string();
            if let Some(ct) = &s.connection_type {
                conn.push_str(&format!("|type:{}", ct));
            }
            if let Some(lc) = &s.last_connection {
                conn.push_str(&format!("|last:{}", lc));
            }
            result.push((valkey_url.connection_name().unwrap_or("").to_string(), conn));
        }
        Ok(result)
    }

    fn settings_path(&self) -> Result<PathBuf, Error> {
        let mut p = PathProvider::get_config_path()?;
        p.push(SETTINGS_FILE);
        Ok(p)
    }

    fn servers_path(&self) -> Result<PathBuf, Error> {
        let mut p = PathProvider::get_config_path()?;
        p.push(SERVER_FILE);
        Ok(p)
    }

    fn read_json_file_from_path<T: DeserializeOwned + Default>(path: &Path) -> Result<T, Error> {
        if fs::exists(path)? {
            let s = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&s).unwrap_or_default())
        } else {
            Ok(T::default())
        }
    }

    fn write_json_file_to_path<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
        let json_content = serde_json::to_string_pretty(value)?;
        let mut file = File::create(path)?;
        file.write_all(json_content.as_bytes())?;
        Ok(())
    }

    fn foreach_kv_line(content: &str, mut handler: impl FnMut(&str, &str)) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();
                handler(key, value);
            }
        }
    }

    fn server_from_parsed(alias: &str, parsed: &ValkeyUrl) -> Server {
        Server {
            database_alias: alias.to_string(),
            host: parsed.host().to_string(),
            port: Some(parsed.port()),
            username: parsed.username().map(|s| s.to_string()),
            password: parsed.password().map(|s| s.to_string()),
            database_index: parsed.db().map(|d| d as usize),
            connection_type: parsed.connection_type().map(|s| s.to_string()),
            last_connection: parsed.last_connection().map(|s| s.to_string()),
        }
    }

    fn read_settings_file(&self) -> Result<SettingsFileContent, Error> {
        let path = self.settings_path()?;
        Self::read_json_file_from_path(&path)
    }

    fn write_settings_file(&self, path: &Path, content: &SettingsFileContent) -> Result<(), Error> {
        Self::write_json_file_to_path(path, content)
    }

    fn read_servers_file(&self) -> Result<Vec<Server>, Error> {
        let path = self.servers_path()?;
        Self::read_json_file_from_path(&path)
    }

    fn write_servers_file(&self, path: &Path, servers: &Vec<Server>) -> Result<(), Error> {
        Self::write_json_file_to_path(path, servers)
    }

    fn remove_old_files(&self, config_path: &Path) -> Result<(), Error> {
        for name in [OLD_SETTINGS_FILE, OLD_SERVER_FILE] {
            let mut old_path = config_path.to_path_buf();
            old_path.push(name);
            if fs::exists(&old_path)? {
                fs::remove_file(&old_path)?;
            }
        }
        Ok(())
    }

    fn migrate_settings_vks(&self, from: &Path, to: &Path) -> Result<(), Error> {
        let content = fs::read_to_string(from)?;
        let mut result = SettingsFileContent::default();
        Self::foreach_kv_line(&content, |key, value| match key.to_lowercase().as_str() {
            "language" => {
                result.language = match value.to_lowercase().as_str() {
                    "english" => Some(Language::English),
                    "german" => Some(Language::German),
                    "spanish" => Some(Language::Spanish),
                    _ => None,
                }
            }
            "theme" => {
                result.theme = match value.to_lowercase().as_str() {
                    "light" => Some(AppTheme::Light),
                    "dark" => Some(AppTheme::Dark),
                    _ => None,
                }
            }
            _ => {}
        });
        self.write_settings_file(to, &result)
    }

    fn migrate_servers_vks(&self, from: &Path, to: &Path) -> Result<(), Error> {
        let content = fs::read_to_string(from)?;
        let mut servers: Vec<Server> = vec![];
        Self::foreach_kv_line(&content, |alias, url| {
            if let Ok(parsed) = ValkeyUrl::parse_valkey_url(Some(alias), url) {
                servers.push(Self::server_from_parsed(alias, &parsed));
            }
        });
        self.write_servers_file(to, &servers)
    }
}

#[cfg(test)]
static TEST_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
#[cfg(test)]
fn restore_test_environment(prev_test_home: Option<std::ffi::OsString>) {
    if let Some(val) = prev_test_home {
        unsafe {
            std::env::set_var("VKC_TEST_CONFIG_HOME", val);
        }
    } else {
        unsafe {
            std::env::remove_var("VKC_TEST_CONFIG_HOME");
        }
    }
}

#[test]
fn load_settings_from_file_test() {
    let _guard = TEST_MUTEX
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap();

    let tmp_dir = std::env::temp_dir().join(format!(
        "vk_cmd_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_dir).unwrap();
    let prev_test_home = std::env::var_os("VKC_TEST_CONFIG_HOME");
    unsafe {
        std::env::set_var("VKC_TEST_CONFIG_HOME", &tmp_dir);
    }

    let settings = AppSettings::default();
    settings
        .delete_settings_files()
        .expect("Failed to delete settings files.");
    settings.load_from_file().expect("Failed to load settings.");

    let lang = settings.get_language();
    assert!(matches!(lang, Language::English));

    settings
        .set_language(&Language::German)
        .expect("Failed to set language.");
    settings.load_from_file().expect("Failed to load settings.");
    let lang = settings.get_language();
    assert!(matches!(lang, Language::German));

    assert_eq!(settings.get_theme(), "system".to_string());
    settings.set_theme("dark").expect("Failed to set theme.");
    settings.load_from_file().expect("Failed to load settings.");
    assert_eq!(settings.get_theme(), "dark".to_string());

    let servers = settings.get_servers();
    assert_eq!(servers.expect("Cannot get servers").len(), 0);
    settings
        .add_server("srv0", "valkey://127.0.0.1:1234")
        .expect("Failed to add server.");
    settings.load_from_file().expect("Failed to load settings.");
    let servers = settings.get_servers().expect("Cannot get servers");
    assert_eq!(servers.len(), 1);
    assert_eq!(
        servers.get(0).cloned(),
        Some(("srv0".to_string(), "valkey://127.0.0.1:1234".to_string()))
    );

    restore_test_environment(prev_test_home);
}

#[test]
fn migration_from_vks_to_json_test() {
    let _guard = TEST_MUTEX
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap();

    let tmp_dir = std::env::temp_dir().join(format!(
        "vk_cmd_migrate_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_dir).unwrap();
    let prev_test_home = std::env::var_os("VKC_TEST_CONFIG_HOME");
    unsafe {
        std::env::set_var("VKC_TEST_CONFIG_HOME", &tmp_dir);
    }

    let config_path = PathProvider::get_config_path().expect("config path");
    let settings_json = config_path.join("settings.json");
    let servers_json = config_path.join("servers.json");
    let settings_vks = config_path.join("settings.vks");
    let server_vks = config_path.join("server.vks");

    for p in [&settings_json, &servers_json, &settings_vks, &server_vks] {
        let _ = fs::remove_file(p);
    }

    fs::write(&settings_vks, b"language=German\ntheme=dark\n").unwrap();
    fs::write(
        &server_vks,
        b"srv1=valkey://user:pass@127.0.0.1:6380/2|type:tls\n#comment\n",
    )
    .unwrap();

    let settings = AppSettings::default();
    settings.load_from_file().expect("load with migration");

    assert!(
        settings_json.exists(),
        "settings.json should exist after migration"
    );
    assert!(
        servers_json.exists(),
        "servers.json should exist after migration"
    );
    assert!(
        !settings_vks.exists(),
        "settings.vks should be removed after migration"
    );
    assert!(
        !server_vks.exists(),
        "server.vks should be removed after migration"
    );

    assert!(matches!(settings.get_language(), Language::German));
    assert_eq!(settings.get_theme(), "dark");

    let servers = settings.get_servers().expect("servers read");
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].0, "srv1");
    assert_eq!(servers[0].1, "valkey://user:pass@127.0.0.1:6380/2|type:tls");

    restore_test_environment(prev_test_home);
}
