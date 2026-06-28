use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use std::thread;

use winapi::um::winuser::*;

use crate::config::{BindMode, SharedConfig};

struct MiniRng {
    state: u64,
}

impl MiniRng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_i64(&mut self) -> i64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x as i64
    }

    fn gen_range(&mut self, min: i64, max: i64) -> i64 {
        min + (self.next_i64().abs()) % (max - min + 1)
    }
}

unsafe fn send_ctrl(down: bool) {
    let mut input: INPUT = std::mem::zeroed();
    input.type_ = INPUT_KEYBOARD;

    let mut flags = KEYEVENTF_SCANCODE;
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }

    *input.u.ki_mut() = KEYBDINPUT {
        wVk: 0,
        wScan: 0x1D,
        dwFlags: flags,
        time: 0,
        dwExtraInfo: 0,
    };

    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
}

pub fn run(config: SharedConfig, running: Arc<AtomicBool>) {
    let mut rng = MiniRng::new();
    let mut last_toggle_time = Instant::now();
    let mut toggle_key_was_down = false;

    let mut cached_enabled = false;
    let mut cached_bind_vk = 0x10u32;
    let mut cached_bind_mode = BindMode::DoubleTap;
    let mut cached_crouch = 150_i64;
    let mut cached_walk = 300_i64;
    let mut cached_jitter = 3_i64;
    let mut cached_double_tap = 300_u64;

    macro_rules! read_config {
        () => {
            if let Ok(cfg) = config.lock() {
                cached_enabled = cfg.enabled;
                cached_bind_vk = cfg.bind_vk;
                cached_bind_mode = cfg.bind_mode;
                cached_crouch = cfg.crouch_hold_ms as i64;
                cached_walk = cfg.walk_delay_ms as i64;
                cached_jitter = cfg.jitter_ms as i64;
                cached_double_tap = cfg.double_tap_ms as u64;
            }
        };
    }

    read_config!();

    while running.load(Ordering::Relaxed) {
        read_config!();

        let now = Instant::now();
        let key_is_down = unsafe { GetAsyncKeyState(cached_bind_vk as i32) < 0 };

        match cached_bind_mode {
            BindMode::DoubleTap => {
                if key_is_down && !toggle_key_was_down {
                    if now.duration_since(last_toggle_time).as_millis() < cached_double_tap as u128
                    {
                        if let Ok(mut cfg) = config.lock() {
                            cfg.enabled = !cfg.enabled;
                            cached_enabled = cfg.enabled;
                            let on = cfg.enabled;
                            if !on {
                                unsafe { send_ctrl(false) };
                            }
                            println!("[{}] Silent Walk (toggle: double-tap)", if on { "ON" } else { "OFF" });
                        }
                    }
                    last_toggle_time = now;
                }
            }
            BindMode::SinglePress => {
                if key_is_down && !toggle_key_was_down {
                    if let Ok(mut cfg) = config.lock() {
                        cfg.enabled = !cfg.enabled;
                        cached_enabled = cfg.enabled;
                        let on = cfg.enabled;
                        if !on {
                            unsafe { send_ctrl(false) };
                        }
                        println!("[{}] Silent Walk (toggle: single)", if on { "ON" } else { "OFF" });
                    }
                }
            }
        }
        toggle_key_was_down = key_is_down;

        if cached_enabled {
            let w_down = unsafe { GetAsyncKeyState(0x57i32) < 0 };

            if w_down {
                let j_c = rng.gen_range(-cached_jitter, cached_jitter);
                let j_w = rng.gen_range(-cached_jitter, cached_jitter);

                let hold = (cached_crouch + j_c).max(20) as u64;
                let delay = (cached_walk + j_w).max(100) as u64;

                unsafe { send_ctrl(true) };
                thread::sleep(Duration::from_millis(hold));
                unsafe { send_ctrl(false) };
                thread::sleep(Duration::from_millis(delay));
                continue;
            } else {
                unsafe { send_ctrl(false) };
            }
        }

        thread::sleep(Duration::from_millis(1));
    }
}
