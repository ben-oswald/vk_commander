use crate::errors::Error;
use std::fmt::{Display, Formatter};

pub struct ValkeyUrl {
    alias: Option<String>,
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    db: Option<u32>,
    connection_type: Option<String>,
    last_connection: Option<String>,
}

impl Default for ValkeyUrl {
    fn default() -> Self {
        Self {
            alias: None,
            host: "127.0.0.1".to_string(),
            port: 6379,
            username: None,
            password: None,
            db: None,
            connection_type: None,
            last_connection: None,
        }
    }
}

impl Display for ValkeyUrl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.alias {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", self.connection_string())
        }
    }
}

impl<T: AsRef<str>> From<T> for ValkeyUrl {
    fn from(value: T) -> Self {
        Self::parse_valkey_url(None, value.as_ref()).unwrap_or_default()
    }
}

impl ValkeyUrl {
    /// Manually parse a Redis URL of the form:
    ///     valkey://[username[:password]@]host[:port][/db]
    ///
    /// Examples:
    ///     valkey://user:secret@127.0.0.1:6379/2
    ///     valkey://user@127.0.0.1:6379
    ///     valkey://:my_password@127.0.0.1:6379/1
    ///     valkey://127.0.0.1:6379
    pub fn connection_string(&self) -> String {
        let mut result = String::from("valkey://");

        if self.username.is_some() || self.password.is_some() {
            if let Some(ref user) = self.username {
                result.push_str(user);
            }
            if self.password.is_some() {
                result.push(':');
                if let Some(ref pass) = self.password {
                    result.push_str(pass);
                }
            }
            result.push('@');
        }

        result.push_str(&self.host);
        result.push(':');
        result.push_str(&self.port.to_string());

        if let Some(db) = self.db {
            result.push('/');
            result.push_str(&db.to_string());
        }
        result
    }

    pub fn connection_name(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }
    pub fn db(&self) -> Option<u32> {
        self.db
    }

    pub fn connection_type(&self) -> Option<&str> {
        self.connection_type.as_deref()
    }

    pub fn last_connection(&self) -> Option<&str> {
        self.last_connection.as_deref()
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn parse_valkey_url(connection_name: Option<&str>, url: &str) -> Result<ValkeyUrl, Error> {
        let mut connection_type = None;
        let mut last_connection = None;
        let url_to_parse = if let Some(pipe_idx) = url.find('|') {
            let metadata_part = &url[pipe_idx + 1..];
            for metadata in metadata_part.split('|') {
                if let Some(colon_idx) = metadata.find(':') {
                    let key = &metadata[..colon_idx];
                    let value = &metadata[colon_idx + 1..];
                    match key {
                        "type" => connection_type = Some(value.to_string()),
                        "last" => {
                            if let Ok(timestamp) = value.parse::<u64>() {
                                let secs_per_day = 86400;
                                let secs_per_hour = 3600;
                                let secs_per_minute = 60;

                                let days_since_epoch = timestamp / secs_per_day;
                                let remaining_secs = timestamp % secs_per_day;

                                let hours = remaining_secs / secs_per_hour;
                                let minutes = (remaining_secs % secs_per_hour) / secs_per_minute;
                                let seconds = remaining_secs % secs_per_minute;

                                let mut year = 1970;
                                let mut remaining_days = days_since_epoch as i64;

                                loop {
                                    let days_in_year = if (year % 4 == 0 && year % 100 != 0)
                                        || (year % 400 == 0)
                                    {
                                        366
                                    } else {
                                        365
                                    };
                                    if remaining_days < days_in_year {
                                        break;
                                    }
                                    remaining_days -= days_in_year;
                                    year += 1;
                                }

                                let is_leap =
                                    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
                                let days_in_months = if is_leap {
                                    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                } else {
                                    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                                };

                                let mut month = 1;
                                for &days_in_month in &days_in_months {
                                    if remaining_days < days_in_month {
                                        break;
                                    }
                                    remaining_days -= days_in_month;
                                    month += 1;
                                }
                                let day = remaining_days + 1;

                                let formatted = format!(
                                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                                    year, month, day, hours, minutes, seconds
                                );
                                last_connection = Some(formatted);
                            } else {
                                last_connection = Some(value.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            &url[..pipe_idx]
        } else {
            url
        };

        let prefix = "valkey://";
        if !url_to_parse.starts_with(prefix) {
            return Err(Error::InvalidInput(
                "URL must start with 'valkey://'".into(),
            ));
        }

        let mut trimmed = &url_to_parse[prefix.len()..];
        let mut username = None;
        let mut password = None;
        let mut db = None;

        if let Some(slash_idx) = trimmed.find('/') {
            let db_str = &trimmed[slash_idx + 1..].trim();
            trimmed = &trimmed[..slash_idx];
            if !db_str.is_empty() {
                db = db_str.parse::<u32>().ok();
            }
        }

        if let Some(at_idx) = trimmed.find('@') {
            let userinfo = &trimmed[..at_idx];
            trimmed = &trimmed[at_idx + 1..];

            if let Some(colon_idx) = userinfo.find(':') {
                let user_part = &userinfo[..colon_idx];
                let pass_part = &userinfo[colon_idx + 1..];
                if !user_part.is_empty() {
                    username = Some(user_part.to_string());
                }
                if !pass_part.is_empty() {
                    password = Some(pass_part.to_string());
                }
            } else if !userinfo.is_empty() {
                username = Some(userinfo.to_string());
            }
        }

        let mut host = trimmed;
        let mut port = 6379;

        if let Some(colon_idx) = trimmed.rfind(':') {
            let port_str = &trimmed[colon_idx + 1..];
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                port = parsed_port;
                host = &trimmed[..colon_idx];
            }
        }

        Ok(ValkeyUrl {
            alias: connection_name.map(|s| s.to_string()),
            username,
            password,
            host: host.to_string(),
            port,
            db,
            connection_type,
            last_connection,
        })
    }
}

pub struct ValkeyUrlBuilder {
    connection_name: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    db: Option<u32>,
    connection_type: Option<String>,
    last_connection: Option<String>,
}

impl Default for ValkeyUrlBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for ValkeyUrlBuilder {
    fn from(value: String) -> Self {
        if let Ok(valkey_url) = ValkeyUrl::parse_valkey_url(None, &value) {
            Self {
                connection_name: None,
                host: Some(valkey_url.host),
                port: Some(valkey_url.port),
                username: valkey_url.username,
                password: valkey_url.password,
                db: valkey_url.db,
                connection_type: valkey_url.connection_type,
                last_connection: valkey_url.last_connection,
            }
        } else {
            Self::new()
        }
    }
}

impl ValkeyUrlBuilder {
    pub fn new() -> Self {
        ValkeyUrlBuilder {
            connection_name: None,
            host: None,
            port: None,
            username: None,
            password: None,
            db: None,
            connection_type: None,
            last_connection: None,
        }
    }

    pub fn connection_name(mut self, connection_name: impl Into<String>) -> Self {
        self.connection_name = Some(connection_name.into());
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn db(mut self, db: u32) -> Self {
        self.db = Some(db);
        self
    }

    pub fn connection_type(mut self, connection_type: impl Into<String>) -> Self {
        self.connection_type = Some(connection_type.into());
        self
    }

    pub fn last_connection(mut self, last_connection: impl Into<String>) -> Self {
        self.last_connection = Some(last_connection.into());
        self
    }

    pub fn build(self) -> Result<ValkeyUrl, Error> {
        let host = self.host.ok_or("Invalid hostname")?;
        let port = self.port.unwrap_or(6379);
        Ok(ValkeyUrl {
            alias: self.connection_name,
            host,
            port,
            username: self.username,
            password: self.password,
            db: self.db,
            connection_type: self.connection_type,
            last_connection: self.last_connection,
        })
    }
}
