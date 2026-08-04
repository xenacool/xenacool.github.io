use std::collections::VecDeque;
use std::sync::Mutex;
use lazy_static::lazy_static;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

struct LogEntry {
    message: String,
    count: usize,
}

struct Logger {
    entries: VecDeque<LogEntry>,
    total_errors: usize,
}

lazy_static! {
    static ref LOGGER: Mutex<Logger> = Mutex::new(Logger {
        entries: VecDeque::new(),
        total_errors: 0,
    });
}

pub fn get_error_count() -> usize {
    LOGGER.lock().map(|l| l.total_errors).unwrap_or(0)
}

pub fn get_log_messages() -> Vec<String> {
    LOGGER.lock().map(|l| {
        l.entries.iter().map(|e| {
            if e.count > 1 {
                format!("{} (x{})", e.message, e.count)
            } else {
                e.message.clone()
            }
        }).collect()
    }).unwrap_or_default()
}

pub fn reset_log() {
    if let Ok(mut logger) = LOGGER.lock() {
        logger.entries.clear();
        logger.total_errors = 0;
    }
}

pub fn ui_log(message: impl Into<String>) {
    let message = message.into();
    let mut logger = match LOGGER.lock() {
        Ok(l) => l,
        Err(_) => return,
    };
    
    logger.total_errors += 1;
    
    if let Some(last) = logger.entries.back_mut() {
        if last.message == message {
            last.count += 1;
        } else {
            logger.entries.push_back(LogEntry {
                message,
                count: 1,
            });
        }
    } else {
        logger.entries.push_back(LogEntry {
            message,
            count: 1,
        });
    }

    if logger.entries.len() > 30 {
        logger.entries.pop_front();
    }

    #[cfg(target_arch = "wasm32")]
    update_ui_log(&logger);
}

#[cfg(target_arch = "wasm32")]
fn update_ui_log(logger: &Logger) {
    let window = if let Some(w) = web_sys::window() { w } else { return; };
    let document = window.document().expect("should have a document on window");
    
    if let Some(display) = document.get_element_by_id("error-display") {
        let text = format!("Error: {}", logger.total_errors);
        display.set_text_content(Some(&text));
        if let Ok(display_html) = display.dyn_into::<web_sys::HtmlElement>() {
            if logger.total_errors > 0 {
                let _ = display_html.style().set_property("color", "red");
            } else {
                let _ = display_html.style().set_property("color", "white");
            }
        }
    }

    if let Some(container) = document.get_element_by_id("log-container") {
        let mut text = String::new();
        for entry in &logger.entries {
            if entry.count > 1 {
                text.push_str(&format!("{} (x{})\n", entry.message, entry.count));
            } else {
                text.push_str(&format!("{}\n", entry.message));
            }
        }
        container.set_text_content(Some(&text));
    }
}
