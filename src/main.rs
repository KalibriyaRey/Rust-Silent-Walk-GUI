#![windows_subsystem = "windows"]

mod config;
mod worker;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

use winapi::shared::windef::POINT;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::*;
use winapi::um::shellapi::*;

use config::*;

enum TrayCommand {
    Toggle,
    Quit,
}

struct SilentWalkApp {
    config: SharedConfig,
    tray_rx: mpsc::Receiver<TrayCommand>,
    minimized: bool,
    listening_for_key: bool,
}

impl SilentWalkApp {
    fn new(config: SharedConfig, tray_rx: mpsc::Receiver<TrayCommand>) -> Self {
        let saved = load_config();
        if let Ok(mut cfg) = config.lock() {
            *cfg = saved;
        }
        Self {
            config,
            tray_rx,
            minimized: false,
            listening_for_key: false,
        }
    }
}

impl SilentWalkApp {
    fn save_to_disk(&self) {
        if let Ok(cfg) = self.config.lock() {
            save_config(&cfg);
        }
    }
}

impl eframe::App for SilentWalkApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(cmd) = self.tray_rx.try_recv() {
            match cmd {
                TrayCommand::Toggle => {
                    if self.minimized {
                        self.minimized = false;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    } else {
                        self.minimized = true;
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                }
                TrayCommand::Quit => {
                    self.save_to_disk();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        let mut cfg = match self.config.lock() {
            Ok(c) => c,
            Err(_) => return,
        };

        ui.horizontal(|ui| {
            ui.heading("\u{1F3A7} Silent Walk Pro");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if cfg.enabled {
                    ui.colored_label(egui::Color32::from_rgb(0, 255, 100), "\u{25CF} ACTIVE");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(100, 100, 100), "\u{25CB} INACTIVE");
                }
            });
        });

        ui.separator();

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(25, 25, 35))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Toggle key:");
                    egui::ComboBox::from_id_salt("bind_key_combo")
                        .width(100.0)
                        .selected_text(vk_to_name(cfg.bind_vk))
                        .show_ui(ui, |ui| {
                            for (name, vk) in VK_CODES {
                                ui.selectable_value(&mut cfg.bind_vk, *vk, *name);
                            }
                        });

                    ui.separator();

                    ui.label("Mode:");
                    egui::ComboBox::from_id_salt("bind_mode_combo")
                        .width(120.0)
                        .selected_text(match cfg.bind_mode {
                            BindMode::DoubleTap => "Double Tap",
                            BindMode::SinglePress => "Single Press",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut cfg.bind_mode,
                                BindMode::DoubleTap,
                                "Double Tap",
                            );
                            ui.selectable_value(
                                &mut cfg.bind_mode,
                                BindMode::SinglePress,
                                "Single Press",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    if self.listening_for_key {
                        ui.label("\u{25CF} Press key or Esc...");
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.listening_for_key = false;
                        }
                        let pressed = ui.input(|i| i.keys_down.clone());
                        for key in pressed {
                            if key != egui::Key::Escape {
                                if let Some(vk) = egui_key_to_windows_vk(key) {
                                    cfg.bind_vk = vk;
                                }
                                self.listening_for_key = false;
                                break;
                            }
                        }
                    } else {
                        if ui.button("\u{23FA} Record").clicked() {
                            self.listening_for_key = true;
                        }
                    }

                    if ui.button("\u{21BA} Defaults").clicked() {
                        *cfg = TimingConfig::default();
                        save_config(&cfg);
                    }

                    if ui.button("\u{1F4BE} Save").clicked() {
                        save_config(&cfg);
                    }
                });
            });

        ui.add_space(8.0);

        ui.add(egui::Slider::new(&mut cfg.crouch_hold_ms, 20.0..=500.0)
            .text("Crouch Hold")
            .suffix(" ms"));

        ui.add(egui::Slider::new(&mut cfg.walk_delay_ms, 50.0..=1000.0)
            .text("Walk Delay")
            .suffix(" ms"));

        ui.add(egui::Slider::new(&mut cfg.double_tap_ms, 100.0..=1000.0)
            .text("Double Tap Window")
            .suffix(" ms"));

        ui.add(egui::Slider::new(&mut cfg.jitter_ms, 0.0..=50.0)
            .text("Jitter")
            .suffix(" ms"));

        ui.add_space(12.0);

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_rgb(20, 20, 30))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let w_down = unsafe { GetAsyncKeyState(0x57i32) < 0 };
                    let bind_down = unsafe { GetAsyncKeyState(cfg.bind_vk as i32) < 0 };

                    ui.label(format!("W: {}", if w_down { "\u{25A0}" } else { "\u{25A1}" }));
                    ui.separator();
                    ui.label(format!(
                        "{}: {}",
                        vk_to_name(cfg.bind_vk),
                        if bind_down { "\u{25A0}" } else { "\u{25A1}" }
                    ));
                    ui.separator();
                    ui.label(if cfg.enabled {
                        "\u{2705} Macro ON"
                    } else {
                        "\u{274C} Macro OFF"
                    });
                });
            });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(format!(
                "Hold {}ms / Walk {}ms",
                cfg.crouch_hold_ms as u64,
                cfg.walk_delay_ms as u64,
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label("v0.2.0");
            });
        });

        ui.ctx().request_repaint();
    }

    fn on_exit(&mut self) {
        self.save_to_disk();
    }
}

fn egui_key_to_windows_vk(key: egui::Key) -> Option<u32> {
    use egui::Key::*;
    Some(match key {
        A => 0x41, B => 0x42, C => 0x43, D => 0x44, E => 0x45, F => 0x46,
        G => 0x47, H => 0x48, I => 0x49, J => 0x4A, K => 0x4B, L => 0x4C,
        M => 0x4D, N => 0x4E, O => 0x4F, P => 0x50, Q => 0x51, R => 0x52,
        S => 0x53, T => 0x54, U => 0x55, V => 0x56, W => 0x57, X => 0x58,
        Y => 0x59, Z => 0x5A,
        Num0 => 0x30, Num1 => 0x31, Num2 => 0x32, Num3 => 0x33,
        Num4 => 0x34, Num5 => 0x35, Num6 => 0x36, Num7 => 0x37,
        Num8 => 0x38, Num9 => 0x39,
        F1 => 0x70, F2 => 0x71, F3 => 0x72, F4 => 0x73, F5 => 0x74,
        F6 => 0x75, F7 => 0x76, F8 => 0x77, F9 => 0x78, F10 => 0x79,
        F11 => 0x7A, F12 => 0x7B,
        Space => 0x20, Enter => 0x0D, Escape => 0x1B, Tab => 0x09,
        Backspace => 0x08, Minus => 0xBD, Equals => 0xBB,
        _ => return None,
    })
}

fn setup_tray_icon(
    event_tx: mpsc::Sender<TrayCommand>,
    running: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null_mut());

            let class_name: Vec<u16> = "SilentWalkTray\0".encode_utf16().collect();

            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(DefWindowProcW),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null_mut(),
                lpszClassName: class_name.as_ptr(),
            };

            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                std::ptr::null(),
                0,
                0, 0, 0, 0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                instance,
                std::ptr::null_mut(),
            );

            if hwnd.is_null() {
                return;
            }

            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_APP + 1;
            nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);

            let tip: Vec<u16> = "Silent Walk Pro\0".encode_utf16().collect();
            let mut i = 0;
            while i < tip.len() && i < 128 {
                nid.szTip[i] = tip[i];
                i += 1;
            }
            while i < 128 {
                nid.szTip[i] = 0;
                i += 1;
            }

            Shell_NotifyIconW(NIM_ADD, &mut nid);

            let mut msg: MSG = std::mem::zeroed();

            loop {
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if msg.message == WM_QUIT {
                        running.store(false, Ordering::Relaxed);
                        Shell_NotifyIconW(NIM_DELETE, &mut nid);
                        DestroyWindow(hwnd);
                        UnregisterClassW(class_name.as_ptr(), instance);
                        return;
                    }
                    if msg.hwnd == hwnd && msg.message == WM_APP + 1 {
                        let lp = msg.lParam as u32;
                        match lp {
                            WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                                let _ = event_tx.send(TrayCommand::Toggle);
                            }
                            WM_RBUTTONUP => {
                                let hmenu = CreatePopupMenu();
                                let show_item: Vec<u16> =
                                    "Show/Hide\0".encode_utf16().collect();
                                let quit_item: Vec<u16> = "Quit\0".encode_utf16().collect();
                                AppendMenuW(hmenu, MF_STRING, 1001, show_item.as_ptr());
                                AppendMenuW(hmenu, MF_STRING, 1002, quit_item.as_ptr());
                                SetForegroundWindow(hwnd);
                                let mut pt: POINT = std::mem::zeroed();
                                GetCursorPos(&mut pt);
                                let cmd = TrackPopupMenu(
                                    hmenu,
                                    TPM_RIGHTBUTTON | TPM_RETURNCMD,
                                    pt.x, pt.y,
                                    0, hwnd,
                                    std::ptr::null_mut(),
                                );
                                PostMessageW(hwnd, WM_NULL, 0, 0);
                                DestroyMenu(hmenu);

                                match cmd {
                                    1001 => {
                                        let _ = event_tx.send(TrayCommand::Toggle);
                                    }
                                    1002 => {
                                        let _ = event_tx.send(TrayCommand::Quit);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    } else {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                if !running.load(Ordering::Relaxed) {
                    Shell_NotifyIconW(NIM_DELETE, &mut nid);
                    DestroyWindow(hwnd);
                    UnregisterClassW(class_name.as_ptr(), instance);
                    return;
                }

                thread::sleep(Duration::from_millis(50));
            }
        }
    });
}

fn main() -> Result<(), eframe::Error> {
    let config: SharedConfig = Arc::new(Default::default());
    let running = Arc::new(AtomicBool::new(true));

    let w_config = Arc::clone(&config);
    let w_running = Arc::clone(&running);
    thread::spawn(move || {
        worker::run(w_config, w_running);
    });

    let (tray_tx, tray_rx) = mpsc::channel();
    setup_tray_icon(tray_tx, Arc::clone(&running));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([340.0, 460.0])
            .with_min_inner_size([320.0, 400.0])
            .with_always_on_top()
            .with_resizable(false),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Silent Walk Pro",
        options,
        Box::new(|_cc| Ok(Box::new(SilentWalkApp::new(config, tray_rx)))),
    );

    running.store(false, Ordering::Relaxed);
    result
}
