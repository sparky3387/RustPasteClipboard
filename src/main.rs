// PasteClipboard – Rust/GTK port
// GUI: GTK4
// Typing: evdev-rs crate for Wayland-compatible uinput (ASCII ONLY)
// Settings: ~/.config/PasteClipboard/config.ini (compatible path)

use gtk4::prelude::*;
use gtk4::{
    glib::{self, source::timeout_add_local_once, ControlFlow, timeout_add_local},
    Application, ApplicationWindow, Button, Entry, Label, Orientation, ScrolledWindow, TextView,
};
use std::sync::mpsc;
use std::path::PathBuf;
use configparser::ini::Ini;
use directories::BaseDirs;
use std::rc::Rc;
use std::cell::RefCell;
use std::thread;
use std::time::Duration;
use anyhow::{Context, Result};
use evdev_rs::{
    enums::{EventCode, EV_KEY, EV_SYN},
    InputEvent, TimeVal, UInputDevice, UninitDevice, DeviceWrapper
};
use std::io::ErrorKind;

const APP_ID: &str = "com.example.PasteClipboard";
const APP_NAME: &str = "PasteClipboard";

fn config_path() -> Option<PathBuf> {
    BaseDirs::new().map(|base| base.config_dir().join("PasteClipboard").join("config.ini"))
}

fn save_settings(delay: &str) {
    if let Some(path) = config_path() {
        let mut conf = Ini::new();
        conf.set("settings", "delay_seconds", Some(delay.to_string()));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = conf.write(path);
    }
}

fn load_settings() -> String {
    let mut delay = "3".to_string();

    if let Some(path) = config_path() {
        let mut conf = Ini::new();
        if conf.load(path).is_ok() {
            if let Some(d) = conf.get("settings", "delay_seconds") {
                delay = d;
            }
        }
    }
    delay
}

/// Maps an ASCII character to its corresponding evdev::Key and whether Shift is needed.
fn char_to_key_event(c: char) -> (EV_KEY, bool) {
    // This exhaustive match is the correct and only reliable way to map chars to keycodes.
    match c {
        'a' => (EV_KEY::KEY_A, false), 'b' => (EV_KEY::KEY_B, false), 'c' => (EV_KEY::KEY_C, false),
        'd' => (EV_KEY::KEY_D, false), 'e' => (EV_KEY::KEY_E, false), 'f' => (EV_KEY::KEY_F, false),
        'g' => (EV_KEY::KEY_G, false), 'h' => (EV_KEY::KEY_H, false), 'i' => (EV_KEY::KEY_I, false),
        'j' => (EV_KEY::KEY_J, false), 'k' => (EV_KEY::KEY_K, false), 'l' => (EV_KEY::KEY_L, false),
        'm' => (EV_KEY::KEY_M, false), 'n' => (EV_KEY::KEY_N, false), 'o' => (EV_KEY::KEY_O, false),
        'p' => (EV_KEY::KEY_P, false), 'q' => (EV_KEY::KEY_Q, false), 'r' => (EV_KEY::KEY_R, false),
        's' => (EV_KEY::KEY_S, false), 't' => (EV_KEY::KEY_T, false), 'u' => (EV_KEY::KEY_U, false),
        'v' => (EV_KEY::KEY_V, false), 'w' => (EV_KEY::KEY_W, false), 'x' => (EV_KEY::KEY_X, false),
        'y' => (EV_KEY::KEY_Y, false), 'z' => (EV_KEY::KEY_Z, false),
        'A' => (EV_KEY::KEY_A, true), 'B' => (EV_KEY::KEY_B, true), 'C' => (EV_KEY::KEY_C, true),
        'D' => (EV_KEY::KEY_D, true), 'E' => (EV_KEY::KEY_E, true), 'F' => (EV_KEY::KEY_F, true),
        'G' => (EV_KEY::KEY_G, true), 'H' => (EV_KEY::KEY_H, true), 'I' => (EV_KEY::KEY_I, true),
        'J' => (EV_KEY::KEY_J, true), 'K' => (EV_KEY::KEY_K, true), 'L' => (EV_KEY::KEY_L, true),
        'M' => (EV_KEY::KEY_M, true), 'N' => (EV_KEY::KEY_N, true), 'O' => (EV_KEY::KEY_O, true),
        'P' => (EV_KEY::KEY_P, true), 'Q' => (EV_KEY::KEY_Q, true), 'R' => (EV_KEY::KEY_R, true),
        'S' => (EV_KEY::KEY_S, true), 'T' => (EV_KEY::KEY_T, true), 'U' => (EV_KEY::KEY_U, true),
        'V' => (EV_KEY::KEY_V, true), 'W' => (EV_KEY::KEY_W, true), 'X' => (EV_KEY::KEY_X, true),
        'Y' => (EV_KEY::KEY_Y, true), 'Z' => (EV_KEY::KEY_Z, true),
        '1' => (EV_KEY::KEY_1, false), '2' => (EV_KEY::KEY_2, false), '3' => (EV_KEY::KEY_3, false),
        '4' => (EV_KEY::KEY_4, false), '5' => (EV_KEY::KEY_5, false), '6' => (EV_KEY::KEY_6, false),
        '7' => (EV_KEY::KEY_7, false), '8' => (EV_KEY::KEY_8, false), '9' => (EV_KEY::KEY_9, false),
        '0' => (EV_KEY::KEY_0, false),
        '!' => (EV_KEY::KEY_1, true), '@' => (EV_KEY::KEY_2, true), '#' => (EV_KEY::KEY_3, true),
        '$' => (EV_KEY::KEY_4, true), '%' => (EV_KEY::KEY_5, true), '^' => (EV_KEY::KEY_6, true),
        '&' => (EV_KEY::KEY_7, true), '*' => (EV_KEY::KEY_8, true), '(' => (EV_KEY::KEY_9, true),
        ')' => (EV_KEY::KEY_0, true),
        '-' => (EV_KEY::KEY_MINUS, false), '_' => (EV_KEY::KEY_MINUS, true),
        '=' => (EV_KEY::KEY_EQUAL, false), '+' => (EV_KEY::KEY_EQUAL, true),
        '[' => (EV_KEY::KEY_LEFTBRACE, false), '{' => (EV_KEY::KEY_LEFTBRACE, true),
        ']' => (EV_KEY::KEY_RIGHTBRACE, false), '}' => (EV_KEY::KEY_RIGHTBRACE, true),
        '\\' => (EV_KEY::KEY_BACKSLASH, false), '|' => (EV_KEY::KEY_BACKSLASH, true),
        ';' => (EV_KEY::KEY_SEMICOLON, false), ':' => (EV_KEY::KEY_SEMICOLON, true),
        '\'' => (EV_KEY::KEY_APOSTROPHE, false), '"' => (EV_KEY::KEY_APOSTROPHE, true),
        '`' => (EV_KEY::KEY_GRAVE, false), '~' => (EV_KEY::KEY_GRAVE, true),
        ',' => (EV_KEY::KEY_COMMA, false), '<' => (EV_KEY::KEY_COMMA, true),
        '.' => (EV_KEY::KEY_DOT, false), '>' => (EV_KEY::KEY_DOT, true),
        '/' => (EV_KEY::KEY_SLASH, false), '?' => (EV_KEY::KEY_SLASH, true),
        ' ' => (EV_KEY::KEY_SPACE, false),
        '\n' => (EV_KEY::KEY_ENTER, false),
        '\t' => (EV_KEY::KEY_TAB, false),
        _ => (EV_KEY::KEY_RESERVED, false),
    }
}


/// Simulates typing the given text using the evdev-rs crate and uinput.
fn simulate_typing_with_uinput(text: &str) -> Result<()> {
    // Explicitly filter for ASCII characters
    let ascii_text: String = text.chars().filter(|c| c.is_ascii()).collect();

    let dev = UninitDevice::new().context("Failed to create uninit evdev device")?;
    dev.set_name("PasteClipboard-Virtual-Keyboard");

    // Define the set of ASCII keys we support
    let supported_keys = "abcdefghijklmnopqrstuvwxyz1234567890!@#$%^&*()-_=+[{]};:'\",<.>/?`~\\| \n\t";
    for char_code in supported_keys.chars() {
        let (key, _) = char_to_key_event(char_code);
        if key != EV_KEY::KEY_RESERVED {
            dev.enable(EventCode::EV_KEY(key)).with_context(|| format!("Failed to enable key {:?}", key))?;
        }
    }
    dev.enable(EventCode::EV_KEY(EV_KEY::KEY_LEFTSHIFT)).context("Failed to enable Shift key")?;

    let device = UInputDevice::create_from_device(&dev).map_err(|err| {
        let context_msg = match err.kind() {
            ErrorKind::NotFound => "Failed to create UInput device. Is the 'uinput' kernel module loaded?",
            ErrorKind::PermissionDenied => "Failed to create UInput device. Do you have permissions for /dev/uinput?",
            _ => "Failed to create UInput device.",
        };
        anyhow::Error::new(err).context(context_msg)
    })?;

    thread::sleep(Duration::from_millis(200));

    let time = TimeVal::new(0, 0);

    for c in ascii_text.chars() {
        let (key, needs_shift) = char_to_key_event(c);
        if key == EV_KEY::KEY_RESERVED {
            continue;
        }

        if needs_shift {
            device.write_event(&InputEvent::new(&time, &EventCode::EV_KEY(EV_KEY::KEY_LEFTSHIFT), 1))?;
            device.write_event(&InputEvent::new(&time, &EventCode::EV_SYN(EV_SYN::SYN_REPORT), 0))?;
        }

        device.write_event(&InputEvent::new(&time, &EventCode::EV_KEY(key), 1))?;
        device.write_event(&InputEvent::new(&time, &EventCode::EV_SYN(EV_SYN::SYN_REPORT), 0))?;
        
        device.write_event(&InputEvent::new(&time, &EventCode::EV_KEY(key), 0))?;
        device.write_event(&InputEvent::new(&time, &EventCode::EV_SYN(EV_SYN::SYN_REPORT), 0))?;

        if needs_shift {
            device.write_event(&InputEvent::new(&time, &EventCode::EV_KEY(EV_KEY::KEY_LEFTSHIFT), 0))?;
            device.write_event(&InputEvent::new(&time, &EventCode::EV_SYN(EV_SYN::SYN_REPORT), 0))?;
        }

        thread::sleep(Duration::from_millis(20));
    }

    Ok(())
}


fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(560)
        .default_height(420)
        .build();

    let vbox = gtk4::Box::new(Orientation::Vertical, 8);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    let lbl_text = Label::new(Some("Input text (typed after delay):"));
    lbl_text.set_xalign(0.0);
    vbox.append(&lbl_text);

    let scrolled = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();
    let text_view = TextView::new();
    text_view.set_wrap_mode(gtk4::WrapMode::WordChar);
    scrolled.set_child(Some(&text_view));
    vbox.append(&scrolled);
    let buffer = text_view.buffer();

    let row = gtk4::Box::new(Orientation::Horizontal, 6);
    let lbl_delay = Label::new(Some("Delay (seconds):"));
    let entry_delay = Entry::new();
    entry_delay.set_max_length(6);
    entry_delay.set_placeholder_text(Some("e.g., 3"));
    row.append(&lbl_delay);
    row.append(&entry_delay);
    vbox.append(&row);

    let btn_start = Button::with_label("Type After Delay");
    vbox.append(&btn_start);

    let lbl_status = Label::new(None);
    lbl_status.set_xalign(0.0);
    vbox.append(&lbl_status);

    window.set_child(Some(&vbox));

    let saved_delay = load_settings();
    entry_delay.set_text(&saved_delay);

    btn_start.connect_clicked(glib::clone!(
        @weak buffer,
        @weak entry_delay,
        @weak lbl_status,
        @weak btn_start,
        => move |_| {
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true).to_string();
            let delay_str = entry_delay.text().to_string();

            save_settings(&delay_str);

            let delay_sec = match delay_str.parse::<u64>() {
                Ok(d) if d <= 86400 => d,
                _ => {
                    lbl_status.set_text("Invalid delay (must be a number from 0–86400).");
                    return;
                }
            };

            btn_start.set_sensitive(false);
            lbl_status.set_text(&format!("Typing in {} second{}... focus the target window.", delay_sec, if delay_sec == 1 { "" } else { "s" }));

            let remaining_seconds = Rc::new(RefCell::new(delay_sec));

            if delay_sec > 0 {
                let lbl_status_clone = lbl_status.clone();
                let remaining_seconds_clone = remaining_seconds.clone();
                timeout_add_local(Duration::from_secs(1), move || {
                    let mut current = remaining_seconds_clone.borrow_mut();
                    *current -= 1;
                    if *current > 0 {
                        lbl_status_clone.set_text(&format!("Typing in {} second{}... focus the target window.", *current, if *current == 1 { "" } else { "s" }));
                        ControlFlow::Continue
                    } else {
                        lbl_status_clone.set_text("Typing now...");
                        ControlFlow::Break
                    }
                });
            }

            let (sender, receiver) = mpsc::channel::<Result<()>>();
            timeout_add_local(Duration::from_millis(100), glib::clone!(
                @weak btn_start,
                @weak lbl_status
                => @default-return ControlFlow::Break,
                move || {
                    match receiver.try_recv() {
                        Ok(result) => {
                            match result {
                                Ok(()) => lbl_status.set_text("✓ Done typing."),
                                Err(e) => lbl_status.set_text(&format!("Typing failed: {:?}", e)),
                            }
                            btn_start.set_sensitive(true);
                            ControlFlow::Break
                        }
                        Err(_) => ControlFlow::Continue,
                    }
                }
            ));

            timeout_add_local_once(Duration::from_secs(delay_sec), move || {
                thread::spawn(move || {
                    let res = simulate_typing_with_uinput(&text);
                    let _ = sender.send(res);
                });
            });
        }
    ));

    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}
