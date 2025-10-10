use crate::errors::Error;
use crate::i18n::{I18N, LangKey};
use crate::state::{Event, Info};
use crate::state::{MainWindow, Message};
use crate::utils::ValkeyUrl;
use crate::utils::valkey::{Len, ToResp, ToVec, ValkeyValue, find_crlf};
use egui::mutex::RwLock;
use std::io;
use std::io::ErrorKind;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;

const MIN_VALKEY_VERSION: (u8, u8, u8) = (8, 0, 0);
const SUPPORTED_SERVERS: [&str; 1] = ["valkey"];
const PARTIALLY_SUPPORTED_SERVERS: [&str; 1] = ["redis"];
const SUPPORTED_PROTOCOLS: [&str; 1] = ["RESP3"];

pub struct ValkeyClient {
    stream: RwLock<TcpStream>,
    alias: Arc<Option<String>>,
    url: Arc<String>,
    server_type: Arc<String>,
}

impl AsRef<str> for ValkeyClient {
    fn as_ref(&self) -> &str {
        self.url.as_ref()
    }
}

impl ValkeyClient {
    pub fn new(
        alias: Arc<Option<String>>,
        url: Arc<String>,
        sender: Arc<Sender<Message>>,
        i18n: Arc<I18N>,
    ) -> Result<Self, Error> {
        sender.send(Message::Event(Arc::from(Event::ShowInfo(Info {
            title: i18n.get(LangKey::ConnectingToServer),
            message: i18n.get(LangKey::ConnectingToServerTakesAWhile),
            callback: None,
        }))))?;

        let valkey_url = ValkeyUrl::parse_valkey_url(None, &url.clone())?;

        let addr = valkey_url.address();
        let socket_addr: SocketAddr = addr
            .to_socket_addrs()?
            .next()
            .ok_or(i18n.get(LangKey::NoValidAddress))?;

        let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        stream.set_nodelay(true)?;

        if valkey_url.password().is_some() || valkey_url.username().is_some() {
            let user = valkey_url.username().unwrap_or("");
            let pass = valkey_url.password().unwrap_or("");

            let auth_cmd = if !user.is_empty() {
                format!(
                    "*3\r\n$4\r\nAUTH\r\n${}\r\n{}\r\n${}\r\n{}\r\n",
                    user.len(),
                    user,
                    pass.len(),
                    pass
                )
            } else {
                format!("*2\r\n$4\r\nAUTH\r\n${}\r\n{}\r\n", pass.len(), pass)
            };
            let res = Self::read_stream(&mut stream, &auth_cmd, None)?;
            if ValkeyValue::from(&res).to_string() != "OK" {
                return Err(Error::Network(i18n.get(LangKey::AuthFailed)))?;
            }
        }

        if let Some(db_index) = valkey_url.db() {
            let db_str = db_index.to_string();
            let select_cmd = format!("*2\r\n$6\r\nSELECT\r\n${}\r\n{}\r\n", db_str.len(), db_str);
            let res = Self::read_stream(&mut stream, &select_cmd, None)?;
            if ValkeyValue::from(&res).to_string() != "OK" {
                return Err(Error::Network(i18n.get(LangKey::SelectDbFail)))?;
            }
        }

        let ping_cmd = "*1\r\n$4\r\nPING\r\n";
        let res = Self::read_stream(&mut stream, ping_cmd, None)?;
        if ValkeyValue::from(&res).to_string() != "PONG" {
            return Err(Error::Network(i18n.get(LangKey::ServerConnectionFailed)))?;
        }

        let hello_cmd = "*2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n";
        let res = Self::read_stream(&mut stream, hello_cmd, None)?;
        let server_response = ValkeyValue::from(&res);
        let mut unsupported = false;
        let mut server_type_str = String::from("unknown");
        match server_response {
            ValkeyValue::Maps(map) => {
                let server = map
                    .get(&ValkeyValue::BulkString("server".as_bytes().to_vec()))
                    .ok_or(i18n.get(LangKey::ServerConnectionFailed))?
                    .to_string();

                if let Some(mode_value) =
                    map.get(&ValkeyValue::BulkString("mode".as_bytes().to_vec()))
                {
                    server_type_str = mode_value.to_string();
                }

                let version = map
                    .get(&ValkeyValue::BulkString("version".as_bytes().to_vec()))
                    .ok_or(i18n.get(LangKey::GetServerVersionFailed))?
                    .to_string();

                let version_number = (
                    version
                        .split('.')
                        .next()
                        .ok_or(i18n.get(LangKey::GetServerVersionFailed))?
                        .parse::<u8>()?,
                    version.split('.').nth(1).unwrap_or("0").parse::<u8>()?,
                    version.split('.').nth(2).unwrap_or("0").parse::<u8>()?,
                );
                if version_number < MIN_VALKEY_VERSION {
                    return Err(Error::Network(format!(
                        "{}\n\
                    {} : {}\n\
                    {} : {}",
                        i18n.get(LangKey::UnsupportedValkeyServerError(
                            MIN_VALKEY_VERSION.0,
                            MIN_VALKEY_VERSION.1,
                            SUPPORTED_PROTOCOLS[0]
                        )),
                        i18n.get(LangKey::YourServer),
                        server,
                        i18n.get(LangKey::Version),
                        version
                    )))?;
                }
                if !SUPPORTED_SERVERS.contains(&server.as_str()) {
                    if !PARTIALLY_SUPPORTED_SERVERS.contains(&server.as_str()) {
                        return Err(Error::Network(format!(
                            "{}\n
                        {}: {}",
                            i18n.get(LangKey::UnsupportedValkeyServerError(
                                version_number.0,
                                version_number.1,
                                SUPPORTED_PROTOCOLS[0]
                            )),
                            i18n.get(LangKey::YourServer),
                            server
                        )))?;
                    } else {
                        sender.send(Message::Event(Arc::from(Event::ShowInfo(Info {
                            title: i18n.get(LangKey::UnsupportedServer),
                            message: i18n.get(LangKey::PartiallySupportedServerError(
                                version_number.0,
                                version_number.1,
                                SUPPORTED_PROTOCOLS[0],
                            )),
                            callback: Some(|| {}),
                        }))))?;
                        unsupported = true;
                    }
                };
            }
            _ => {
                return Err(Error::Network(i18n.get(
                    LangKey::UnsupportedValkeyServerError(
                        MIN_VALKEY_VERSION.0,
                        MIN_VALKEY_VERSION.1,
                        SUPPORTED_PROTOCOLS[0],
                    ),
                )))?;
            }
        }

        sender.send(Message::Event(Arc::from(Event::SetMainWindow(
            MainWindow::Connection,
        ))))?;
        if !unsupported {
            sender.send(Message::Event(Arc::from(Event::CloseInfo())))?
        };

        Ok(Self {
            alias,
            stream: RwLock::new(stream),
            url,
            server_type: Arc::from(server_type_str),
        })
    }

    pub fn set(&self, key: &str, value: &str, ttl: Option<usize>) -> Result<String, Error> {
        let value = ValkeyValue::BulkString(value.as_bytes().to_vec());
        let mut stream = self.stream.write();

        let command = if let Some(expire) = ttl {
            format!(
                "*{}\r\n$3\r\nSET\r\n${}\r\n{}\r\n{}$2\r\nEX\r\n${}\r\n{}\r\n",
                value.len() + 4,
                key.len(),
                key,
                value.to_resp(),
                expire.to_string().len(),
                expire
            )
        } else {
            format!(
                "*{}\r\n$3\r\nSET\r\n${}\r\n{}\r\n{}\r\n",
                value.len() + 2,
                key.len(),
                key,
                value.to_resp(),
            )
        };

        let res = Self::read_stream(&mut stream, &command, None)?;
        Ok(res)
    }

    pub fn get(&self, key: &str) -> Result<String, Error> {
        let mut stream = self.stream.write();
        let command = format!("*2\r\n$3\r\nGET\r\n${}\r\n{}\r\n", key.len(), key);

        let response = Self::read_stream(&mut stream, &command, None)?;
        let valkey_value: ValkeyValue = ValkeyValue::from(response.as_str());

        Ok(valkey_value.to_string())
    }

    pub fn exec(&self, commands: &str) -> Result<Vec<String>, Error> {
        let commands = Self::split_commands(commands.trim());
        let mut resp_string = format!("*{}\r\n", commands.len());

        for command in commands {
            resp_string.push_str(format!("${}\r\n{command}\r\n", command.len()).as_str())
        }
        let res = self.exec_raw(&resp_string)?;

        let valkey_value = ValkeyValue::from(res.as_str());
        Ok(valkey_value.to_vec())
    }

    pub fn exec_pipelined(&self, commands: &Vec<String>) -> Result<Vec<String>, Error> {
        let mut resp_string = String::new();
        let mut pipeline_resp_string = String::new();
        for command in commands {
            let individual_commands = Self::split_commands(command.trim());
            resp_string.push_str(format!("*{}\r\n", individual_commands.len()).as_str());
            for individual_command in individual_commands {
                resp_string.push_str(
                    format!("${}\r\n{individual_command}\r\n", individual_command.len()).as_str(),
                )
            }
        }
        let res = self.exec_raw_pipelined(&resp_string, commands.len())?;
        pipeline_resp_string.push_str(res.as_str());
        let valkey_value = ValkeyValue::parse_all_values(pipeline_resp_string.as_str());
        let valkey_value: Vec<String> = valkey_value.iter().map(|v| v.to_string()).collect();
        Ok(valkey_value)
    }

    pub fn exec_raw(&self, command: &str) -> Result<String, Error> {
        let mut stream = self.stream.write();
        let res = Self::read_stream(&mut stream, command, None)?;
        Ok(res)
    }

    pub fn exec_raw_pipelined(
        &self,
        command: &str,
        expected_count: usize,
    ) -> Result<String, Error> {
        let mut stream = self.stream.write();
        let res = Self::read_stream(&mut stream, command, Some(expected_count))?;
        Ok(res)
    }

    fn count_complete_resp_messages(data: &str) -> usize {
        let mut count = 0;
        let mut pos = 0;

        while pos < data.len() {
            if let Some(end_pos) = Self::find_next_complete_message(data, pos) {
                count += 1;
                pos = end_pos;
            } else {
                break;
            }
        }

        count
    }

    fn find_next_complete_message(data: &str, start_pos: usize) -> Option<usize> {
        if start_pos >= data.len() {
            return None;
        }

        data[start_pos..]
            .find("\r\n")
            .map(|crlf_pos| start_pos + crlf_pos + 2)
    }

    fn read_stream(
        stream: &mut TcpStream,
        command: &str,
        expected_count: Option<usize>,
    ) -> Result<String, Error> {
        stream.write_all(command.as_bytes())?;
        stream.flush()?;

        let mut response = Vec::new();
        let mut buffer = [0; 8192];
        let mut consecutive_would_block = 0;
        const MAX_WOULD_BLOCK: usize = 3;

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    if !response.is_empty() {
                        let response_str = String::from_utf8(response).map_err(|e| {
                            Error::from(io::Error::new(io::ErrorKind::InvalidData, e))
                        })?;
                        return Ok(response_str);
                    }
                    return Err(Error::from(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Connection closed by server without response",
                    )));
                }
                Ok(n) => {
                    consecutive_would_block = 0;
                    response.extend_from_slice(&buffer[..n]);

                    if let Ok(response_str) = String::from_utf8(response.clone()) {
                        if let Some(expected_count) = expected_count {
                            if Self::count_complete_resp_messages(&response_str) >= expected_count {
                                return Ok(response_str);
                            }
                        } else if Self::is_complete_resp_message(&response_str) {
                            return Ok(response_str);
                        }
                    }
                }
                Err(e) => match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::TimedOut => {
                        consecutive_would_block += 1;
                        if consecutive_would_block >= MAX_WOULD_BLOCK {
                            if response.is_empty() {
                                return Err(Error::from(io::Error::new(
                                    ErrorKind::TimedOut,
                                    "Server did not respond within timeout period",
                                )));
                            } else {
                                return Err(Error::from(io::Error::new(
                                    ErrorKind::TimedOut,
                                    "Server response incomplete - timeout while waiting for more data",
                                )));
                            }
                        }
                        continue;
                    }
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {
                        return Err(Error::from(io::Error::new(
                            e.kind(),
                            "Connection lost to server",
                        )));
                    }
                    _ => {
                        return Err(Error::from(e));
                    }
                },
            }
        }
    }

    fn is_complete_resp_message(data: &str) -> bool {
        let bytes = data.as_bytes();
        match Self::parse_resp_value(bytes, 0) {
            Ok((_, consumed)) => consumed == bytes.len(),
            Err(_) => false,
        }
    }

    fn parse_resp_value(data: &[u8], start: usize) -> Result<(usize, usize), &'static str> {
        if start >= data.len() {
            return Err("Incomplete data");
        }

        let first_byte = data[start];

        match first_byte {
            b'+' | b'-' | b':' | b'#' | b',' | b'(' | b'_' => {
                if let Some(end_pos) = find_crlf(data, start) {
                    Ok((end_pos + 2, end_pos + 2 - start))
                } else {
                    Err("Incomplete simple type")
                }
            }
            b'$' => {
                if let Some(header_end) = find_crlf(data, start) {
                    let length_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid bulk string header")?;
                    let length: i64 = length_str
                        .parse()
                        .map_err(|_| "Invalid bulk string length")?;

                    if length < 0 {
                        Ok((header_end + 2, header_end + 2 - start))
                    } else {
                        let data_start = header_end + 2;
                        let data_end = data_start + length as usize;

                        if data_end + 2 <= data.len()
                            && data[data_end] == b'\r'
                            && data[data_end + 1] == b'\n'
                        {
                            Ok((data_end + 2, data_end + 2 - start))
                        } else {
                            Err("Incomplete bulk string data")
                        }
                    }
                } else {
                    Err("Incomplete bulk string header")
                }
            }
            b'*' => {
                if let Some(header_end) = find_crlf(data, start) {
                    let count_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid array header")?;
                    let count: i64 = count_str.parse().map_err(|_| "Invalid array count")?;

                    if count < 0 {
                        Ok((header_end + 2, header_end + 2 - start))
                    } else {
                        let mut pos = header_end + 2;
                        for _ in 0..count {
                            let (new_pos, _) = Self::parse_resp_value(data, pos)?;
                            pos = new_pos;
                        }
                        Ok((pos, pos - start))
                    }
                } else {
                    Err("Incomplete array header")
                }
            }
            b'!' => {
                if let Some(header_end) = find_crlf(data, start) {
                    let length_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid bulk error header")?;
                    let length: i64 = length_str
                        .parse()
                        .map_err(|_| "Invalid bulk error length")?;

                    if length < 0 {
                        Ok((header_end + 2, header_end + 2 - start))
                    } else {
                        let data_start = header_end + 2;
                        let data_end = data_start + length as usize;

                        if data_end + 2 <= data.len()
                            && data[data_end] == b'\r'
                            && data[data_end + 1] == b'\n'
                        {
                            Ok((data_end + 2, data_end + 2 - start))
                        } else {
                            Err("Incomplete bulk error data")
                        }
                    }
                } else {
                    Err("Incomplete bulk error header")
                }
            }
            b'%' => {
                if let Some(header_end) = find_crlf(data, start) {
                    let len_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid map header")?;
                    let len: usize = len_str.parse().map_err(|_| "Invalid map length")?;

                    let mut pos = header_end + 2;
                    for _ in 0..len {
                        let (new_pos, _) = Self::parse_resp_value(data, pos)?;
                        pos = new_pos;
                        let (new_pos, _) = Self::parse_resp_value(data, pos)?;
                        pos = new_pos;
                    }
                    Ok((pos, pos - start))
                } else {
                    Err("Incomplete map header")
                }
            }
            b'~' | b'>' => {
                if let Some(header_end) = find_crlf(data, start) {
                    let len_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid collection header")?;
                    let len: usize = len_str.parse().map_err(|_| "Invalid collection length")?;

                    let mut pos = header_end + 2;
                    for _ in 0..len {
                        let (new_pos, _) = Self::parse_resp_value(data, pos)?;
                        pos = new_pos;
                    }
                    Ok((pos, pos - start))
                } else {
                    Err("Incomplete collection header")
                }
            }
            b'=' => {
                if let Some(header_end) = find_crlf(data, start) {
                    let length_str = std::str::from_utf8(&data[start + 1..header_end])
                        .map_err(|_| "Invalid verbatim string header")?;
                    let length: i64 = length_str
                        .parse()
                        .map_err(|_| "Invalid verbatim string length")?;

                    if length < 0 {
                        Ok((header_end + 2, header_end + 2 - start))
                    } else {
                        let data_start = header_end + 2;
                        let data_end = data_start + length as usize;

                        if data_end + 2 <= data.len()
                            && data[data_end] == b'\r'
                            && data[data_end + 1] == b'\n'
                        {
                            Ok((data_end + 2, data_end + 2 - start))
                        } else {
                            Err("Incomplete verbatim string data")
                        }
                    }
                } else {
                    Err("Incomplete verbatim string header")
                }
            }
            _ => Err("Unknown RESP type"),
        }
    }

    pub fn server_url(&self) -> String {
        (*self.url).clone()
    }

    pub fn alias(&self) -> Option<String> {
        (*self.alias).clone()
    }

    pub fn server_type(&self) -> String {
        (*self.server_type).clone()
    }

    fn split_commands(input: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\\' if in_quotes => {
                    if let Some(next_ch) = chars.next() {
                        match next_ch {
                            '"' => current_token.push('"'),
                            '\'' => current_token.push('\''),
                            '\\' => current_token.push('\\'),
                            'n' => current_token.push('\n'),
                            't' => current_token.push('\t'),
                            _ => {
                                current_token.push('\\');
                                current_token.push(next_ch);
                            }
                        }
                    }
                }
                '"' | '\'' => {
                    if in_quotes {
                        result.push(current_token.clone());
                        current_token.clear();
                        in_quotes = false;
                    } else {
                        in_quotes = true;
                    }
                }
                ' ' if !in_quotes => {
                    if !current_token.is_empty() {
                        result.push(current_token.clone());
                        current_token.clear();
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.is_empty() {
            result.push(current_token);
        }

        result
    }
}
