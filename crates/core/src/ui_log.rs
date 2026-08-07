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
    pub total_info: usize,
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
            LogCommand::Error(message) => {
                self.total_errors += 1;
                self.add_entry(message);
            }
            LogCommand::Info(message) => {
                self.total_info += 1;
                self.add_entry(message);
            }
            LogCommand::Reset => {
                self.entries.clear();
                self.total_errors = 0;
                self.total_info = 0;
            }
        }
    }

    fn add_entry(&mut self, message: String) {
        if let Some(last) = self.entries.back_mut() {
            if last.message == message {
                last.count += 1;
            } else {
                self.entries.push_back(LogEntry { message, count: 1 });
            }
        } else {
            self.entries.push_back(LogEntry { message, count: 1 });
        }
        if self.entries.len() > 7 {
            self.entries.pop_front();
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum LogCommand {
    Info(String),
    Error(String),
    Reset,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_error_vs_info() {
        let mut logger = Logger::new();
        
        logger.apply_command(LogCommand::Info("Some info".to_string()));
        assert_eq!(logger.total_info, 1);
        assert_eq!(logger.total_errors, 0);
        
        logger.apply_command(LogCommand::Error("Some error".to_string()));
        assert_eq!(logger.total_info, 1);
        assert_eq!(logger.total_errors, 1);
        
        logger.apply_command(LogCommand::Info("Some info".to_string()));
        assert_eq!(logger.total_info, 2);
        assert_eq!(logger.total_errors, 1);
        
        assert_eq!(logger.entries.len(), 3);
    }
}
