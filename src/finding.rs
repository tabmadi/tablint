use std::fmt;

/// One violation. Line 0 means the finding is about the whole file.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub path: String,
    pub line: usize,
    pub rule: &'static str,
    pub message: String,
}

impl Finding {
    pub fn new(path: &str, line: usize, rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line,
            rule,
            message: message.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}: {}", self.path, self.rule, self.message)
        } else {
            write!(
                f,
                "{}:{}: {}: {}",
                self.path, self.line, self.rule, self.message
            )
        }
    }
}
