use std::collections::VecDeque;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub message: String,
    pub count: usize,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Logger {
    pub entries: VecDeque<LogEntry>,
    pub total_errors: usize,
}

impl Logger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_messages(&self) -> Vec<String> {
        self.entries.iter().map(|e| {
            if e.count > 1 {
                format!("{} (x{})", e.message, e.count)
            } else {
                e.message.clone()
            }
        }).collect()
    }

    pub fn apply_command(&mut self, cmd: LogCommand) {
        match cmd {
            LogCommand::Log(message) => {
                self.total_errors += 1;
                if let Some(last) = self.entries.back_mut() {
                    if last.message == message {
                        last.count += 1;
                    } else {
                        self.entries.push_back(LogEntry { message, count: 1 });
                    }
                } else {
                    self.entries.push_back(LogEntry { message, count: 1 });
                }
                if self.entries.len() > 30 {
                    self.entries.pop_front();
                }
            }
            LogCommand::Reset => {
                self.entries.clear();
                self.total_errors = 0;
            }
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum LogCommand {
    Log(String),
    Reset,
}
