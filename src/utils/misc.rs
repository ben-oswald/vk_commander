use crate::APP_NAME;
use crate::errors::Error;
use egui::Color32;
use std::path::PathBuf;

pub struct PathProvider;

impl PathProvider {
    pub fn get_config_path() -> Result<PathBuf, Error> {
        if cfg!(target_os = "windows") {
            let path = PathBuf::from(std::env::var("APP_DATA")?);
            if !path.exists() {
                std::fs::create_dir_all(&path)?;
            }
            Ok(path.join(APP_NAME))
        } else {
            let path = PathBuf::from(std::env::var_os("XDG_CONFIG_HOME").unwrap_or_else(|| {
                let home = std::env::var_os("HOME").unwrap_or_default();
                std::path::Path::new(&home).join(".config").into_os_string()
            }));
            let path = path.join(APP_NAME);
            if !path.exists() {
                std::fs::create_dir_all(&path)?;
            }
            Ok(path)
        }
    }
}

pub fn random_string(len: usize) -> Result<String, Error> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos() as u64;

    let mut state = seed;

    let letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut result = String::with_capacity(len);

    for _ in 0..len {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let idx = (state >> 32) as usize % letters.len();
        result.push(letters[idx] as char);
    }

    Ok(result)
}

pub fn text_float_filter_less_than_one(input: &str) -> String {
    let filtered = text_float_filter(input);

    if filtered.is_empty() || filtered == "-" {
        return filtered;
    }

    if let Ok(value) = filtered.parse::<f64>() {
        if value < 1.0 { filtered } else { String::new() }
    } else if filtered.starts_with("0") || filtered.starts_with("-") || filtered == "." {
        filtered
    } else {
        String::new()
    }
}

pub fn format_size(s: u64) -> String {
    if s < 1024 {
        format!("{s}B")
    } else if s < 1024 * 1024 {
        format!("{:.1}KiB", s as f64 / 1024.0)
    } else if s < 1024 * 1024 * 1024 {
        format!("{:.1}MiB", s as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GiB", s as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn type_color(key_type: &str) -> Color32 {
    match key_type {
        "string" => Color32::from_rgb(255, 99, 132),
        "hash" => Color32::from_rgb(54, 162, 235),
        "list" => Color32::from_rgb(255, 206, 86),
        "set" => Color32::from_rgb(75, 192, 192),
        "zset" => Color32::from_rgb(153, 102, 255),
        _ => Color32::from_rgb(201, 203, 207),
    }
}

pub fn text_float_filter(input: &str) -> String {
    let mut result = String::new();
    let mut has_dot = false;
    let mut has_minus = false;

    for (index, c) in input.chars().enumerate() {
        if c.is_ascii_digit() {
            result.push(c);
        } else if c == '.' && !has_dot {
            result.push(c);
            has_dot = true;
        } else if c == '-' && !has_minus && index == 0 {
            result.push(c);
            has_minus = true;
        }
    }
    result
}
