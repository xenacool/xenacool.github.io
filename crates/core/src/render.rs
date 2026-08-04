use std::sync::atomic::AtomicBool;

pub static ERROR_MODE_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct RenderError<T> {
    pub message: String,
    pub fallback: T,
}

impl<T> RenderError<T> {
    pub fn new(message: impl Into<String>, fallback: T) -> Self {
        Self {
            message: message.into(),
            fallback,
        }
    }
}
