use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BindMode {
    DoubleTap,
    SinglePress,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    pub double_tap_ms: f64,
    pub crouch_hold_ms: f64,
    pub walk_delay_ms: f64,
    pub jitter_ms: f64,
    pub enabled: bool,
    pub bind_vk: u32,
    pub bind_mode: BindMode,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            double_tap_ms: 300.0,
            crouch_hold_ms: 150.0,
            walk_delay_ms: 300.0,
            jitter_ms: 3.0,
            enabled: false,
            bind_vk: 0x10,
            bind_mode: BindMode::DoubleTap,
        }
    }
}

pub type SharedConfig = Arc<Mutex<TimingConfig>>;

pub fn config_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let dir = base.join("silent_walk");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("config.json")
}

pub fn load_config() -> TimingConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => TimingConfig::default(),
    }
}

pub fn save_config(config: &TimingConfig) {
    let path = config_path();
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&path, &json);
    }
}

pub const VK_CODES: &[(&str, u32)] = &[
    ("Shift", 0x10),
    ("Ctrl", 0x11),
    ("Alt", 0x12),
    ("Caps Lock", 0x14),
    ("Tab", 0x09),
    ("Space", 0x20),
    ("Enter", 0x0D),
    ("Backspace", 0x08),
    ("Esc", 0x1B),
    ("F1", 0x70),
    ("F2", 0x71),
    ("F3", 0x72),
    ("F4", 0x73),
    ("F5", 0x74),
    ("F6", 0x75),
    ("F7", 0x76),
    ("F8", 0x77),
    ("F9", 0x78),
    ("F10", 0x79),
    ("F11", 0x7A),
    ("F12", 0x7B),
    ("0", 0x30),
    ("1", 0x31),
    ("2", 0x32),
    ("3", 0x33),
    ("4", 0x34),
    ("5", 0x35),
    ("6", 0x36),
    ("7", 0x37),
    ("8", 0x38),
    ("9", 0x39),
    ("A", 0x41),
    ("B", 0x42),
    ("C", 0x43),
    ("D", 0x44),
    ("E", 0x45),
    ("F", 0x46),
    ("G", 0x47),
    ("H", 0x48),
    ("I", 0x49),
    ("J", 0x4A),
    ("K", 0x4B),
    ("L", 0x4C),
    ("M", 0x4D),
    ("N", 0x4E),
    ("O", 0x4F),
    ("P", 0x50),
    ("Q", 0x51),
    ("R", 0x52),
    ("S", 0x53),
    ("T", 0x54),
    ("U", 0x55),
    ("V", 0x56),
    ("W", 0x57),
    ("X", 0x58),
    ("Y", 0x59),
    ("Z", 0x5A),
    ("`", 0xC0),
    ("-", 0xBD),
    ("=", 0xBB),
    ("[", 0xDB),
    ("]", 0xDD),
    (";", 0xBA),
    ("'", 0xDE),
    (",", 0xBC),
    (".", 0xBE),
    ("/", 0xBF),
    ("Mouse 4", 0x05),
    ("Mouse 5", 0x06),
];

pub fn vk_to_name(vk: u32) -> String {
    for (name, code) in VK_CODES {
        if *code == vk {
            return name.to_string();
        }
    }
    if vk >= 0x70 && vk <= 0x7B {
        return format!("F{}", vk - 0x70 + 1);
    }
    format!("0x{:02X}", vk)
}

#[allow(dead_code)]
pub fn name_to_vk(name: &str) -> Option<u32> {
    for (n, code) in VK_CODES {
        if *n == name {
            return Some(*code);
        }
    }
    None
}
