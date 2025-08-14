// PasteClipboard — Rust/GTK port
// GUI: GTK4
// Typing: libxdo via minimal FFI
// Settings: ~/.config/PasteClipboard/config.ini (compatible path)

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Button, Entry, Label, Orientation, ScrolledWindow, TextView};
use glib::{source::timeout_add_local, ControlFlow};
use std::sync::mpsc;
use std::path::PathBuf;
use configparser::ini::Ini;
use directories::BaseDirs;
use std::ffi::CString;
use libc;
use std::rc::Rc;
use std::cell::RefCell;

#[allow(non_camel_case_types)]
type xdo_t = std::ffi::c_void;

#[link(name = "xdo")]
extern "C" {
    fn xdo_new(display: *const libc::c_char) -> *mut xdo_t;
    fn xdo_free(xdo: *mut xdo_t);
    fn xdo_enter_text_window(
        xdo: *mut xdo_t,
        window: libc::c_ulong,
        string: *const libc::c_char,
        delay: libc::useconds_t
    ) -> libc::c_int;
}

const APP_ID: &str = "com.example.PasteClipboard"; 
const APP_NAME: &str = "PasteClipboard";

fn config_path() -> Option<PathBuf> {
    let base = BaseDirs::new()?;
    Some(base.config_dir().join("PasteClipboard").join("config.ini"))
}


fn save_settings(text: &str, delay: &str) {
    if let Some(path) = config_path() {
        let mut conf = Ini::new();
        conf.set("settings", "text", Some(text.to_string()));
        conf.set("settings", "delay_seconds", Some(delay.to_string()));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = conf.write(path);
    }
}

fn load_settings() -> (String, String) {
    let mut text = String::new();
    let mut delay = "3".to_string();

    if let Some(path) = config_path() {
        let mut conf = Ini::new();
        if conf.load(path).is_ok() {
            if let Some(t) = conf.get("settings", "text") {
                text = t;
            }
            if let Some(d) = conf.get("settings", "delay_seconds") {
                delay = d;
            }
        }
    }
    (text, delay)
}

// Typing function brought over from main_test.rs
fn type_with_libxdo(text: &str) -> Result<(), String> {
    let c_text = CString::new(text)
        .map_err(|e| format!("Failed to create C string: {}", e))?;

    unsafe {
        let xdo = xdo_new(std::ptr::null());
        if xdo.is_null() {
            return Err("Failed to create xdo instance - is X11 running?".to_string());
        }

        // Type into the currently focused window (0) with a 12ms delay between keys.
        let result = xdo_enter_text_window(xdo, 0, c_text.as_ptr(), 12000);
        xdo_free(xdo);

        if result == 0 {
            Ok(())
        } else {
            Err(format!("xdo_enter_text_window failed with error code: {}", result))
        }
    }
}


fn build_ui(app: &Application) {
    // Widgets
    let window = ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(560)
        .default_height(420)
        .build();

    let vbox = gtk4::Box::new(Orientation::Vertical, 8);

    let lbl_text = Label::new(Some("Input text (typed after delay):"));
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
    vbox.append(&lbl_status);

    window.set_child(Some(&vbox));

    // Initialize settings
    let (saved_text, saved_delay) = load_settings();
    buffer.set_text(&saved_text);
    entry_delay.set_text(&saved_delay);

    // --- Button Click Logic ---
    btn_start.connect_clicked(glib::clone!(
        #[weak] buffer,
        #[weak] entry_delay,
        #[weak] lbl_status,
        move |btn_start| {
            // Extract current text and delay from widgets
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            let text = buffer.text(&start, &end, true).to_string();
            let delay_str = entry_delay.text().to_string();

            // Save settings immediately
            save_settings(&text, &delay_str);

            // Parse delay
            let delay_sec = match delay_str.parse::<u64>() {
                Ok(d) if d <= 86400 => d, // Max 24 hours
                _ => {
                    lbl_status.set_text("Invalid delay (must be a number from 0–86400).");
                    return;
                }
            };

            // --- MODIFICATION START ---

            // Disable button and set initial status
            btn_start.set_sensitive(false);
            lbl_status.set_text(&format!("Typing in {} second{}... focus the target window.", delay_sec, if delay_sec == 1 {""} else {"s"}));

            // Use Rc<RefCell> for a shared, mutable countdown timer state
            let remaining_seconds = Rc::new(RefCell::new(delay_sec));

            // A. Create a recurring timer to UPDATE THE LABEL every second.
            if delay_sec > 0 {
                let lbl_status_clone = lbl_status.clone();
                timeout_add_local(std::time::Duration::from_secs(1), move || {
                    let mut current = remaining_seconds.borrow_mut();
                    *current -= 1;

                    if *current > 0 {
                        lbl_status_clone.set_text(&format!("Typing in {} second{}... focus the target window.", *current, if *current == 1 {""} else {"s"}));
                        ControlFlow::Continue // Keep the timer running
                    } else {
                        lbl_status_clone.set_text("Typing now...");
                        ControlFlow::Break // Stop this countdown timer
                    }
                });
            }


            // B. The existing logic to schedule the TYPING ACTION after the full delay.
            // This remains mostly unchanged, but we've already set the label and disabled the button.
            let (sender, receiver) = mpsc::channel();

            // Check for the result from the typing thread (polling)
            let lbl_status_clone = lbl_status.clone();
            let btn_start_clone = btn_start.clone();
            timeout_add_local(std::time::Duration::from_millis(100), move || {
                match receiver.try_recv() {
                    Ok(result) => {
                        match result {
                            Ok(()) => lbl_status_clone.set_text("✓ Done typing."),
                            Err(e) => lbl_status_clone.set_text(&format!("Typing failed: {}", e)),
                        }
                        btn_start_clone.set_sensitive(true);
                        ControlFlow::Break // Stop polling for the result
                    }
                    Err(_) => ControlFlow::Continue, // Continue polling
                }
            });

            // Use the original one-shot timer to trigger the background thread
            glib::timeout_add_seconds_local_once(delay_sec as u32, move || {
                std::thread::spawn(move || {
                    let res = type_with_libxdo(&text);
                    let _ = sender.send(res); // Send result back to the main thread
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

