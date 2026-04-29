use std::path::PathBuf;

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pi-oven").join("client.toml")
}

/// Load `font_size` from `~/.pi-oven/client.toml`, returning `default` if the
/// file is absent, unreadable, or doesn't contain a valid value.
pub fn load_font_size(default: f32) -> f32 {
    let content = match std::fs::read_to_string(config_path()) {
        Ok(s) => s,
        Err(_) => return default,
    };
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix("font_size") {
            let value = rest.trim_start_matches(|c: char| c == ' ' || c == '=').trim();
            if let Ok(v) = value.parse::<f32>() {
                return v;
            }
        }
    }
    default
}

/// Write `font_size` to `~/.pi-oven/client.toml`, creating the directory if
/// needed. Failures are silently ignored — a missing pref is not fatal.
pub fn save_font_size(font_size: f32) {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("font_size = {font_size}\n"));
}
