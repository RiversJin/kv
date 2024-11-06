use std::{error::Error, sync::atomic::AtomicIsize, time::Duration};

#[derive(Debug)]
pub struct Context {
    pub timeout: Option<Duration>,
    pub retries: AtomicIsize,
    pub start_time: std::time::Instant,
}

impl Context {
    pub fn new(timeout: Option<Duration>, retries: isize) -> Self {
        Context {
            timeout,
            retries: retries.into(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn is_timeout(&self) -> Result<(), Box<dyn Error>> {
        if let Some(timeout) = self.timeout {
            if self.start_time.elapsed() > timeout {
                return Err("Timeout".into());
            }
        }
        Ok(())
    }

    pub fn is_retriable(&self) -> bool {
        self.retries.load(std::sync::atomic::Ordering::Relaxed) > 0
    }

    pub fn decrease_retries(&self) -> Result<(), Box<dyn Error>> {
        let retries = self.retries.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        if retries <= 0 {
            return Err("No more retries".into());
        }
        Ok(())
    }
}

