
use chrono::{NaiveDateTime, Utc};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait TimeProvider : Send + Sync {
    fn naive_utc_start(&self) -> NaiveDateTime;
    fn naive_utc_now(&self) -> NaiveDateTime {
        Utc::now().naive_utc()
    }
}

pub struct CoreTimeProvider { start: NaiveDateTime }
impl CoreTimeProvider {
    pub fn new() -> Self {
        Self { start: Utc::now().naive_utc() }
    }
}
impl TimeProvider for CoreTimeProvider {
    fn naive_utc_start(&self) -> NaiveDateTime {
        self.start
    }
}