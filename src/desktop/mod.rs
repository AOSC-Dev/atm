mod kde5;

pub trait ProgressTracker {
    fn set_percent(&mut self, percent: u32);
    fn set_general_description(&mut self, description: &str);
    fn set_message(&mut self, label: &str, message: &str);
    fn terminate(&mut self, message: &str);
}

pub use kde5::KF5Tracker;

pub struct DummyTracker;

impl ProgressTracker for DummyTracker {
    fn set_percent(&mut self, _: u32) {}

    fn set_general_description(&mut self, _: &str) {}

    fn set_message(&mut self, _: &str, _: &str) {}

    fn terminate(&mut self, _: &str) {}
}

impl DummyTracker {
    pub fn new() -> Self {
        Self {}
    }
}

pub fn select_best_tracker() -> Box<dyn ProgressTracker> {
    let kf5 = KF5Tracker::new("atm");
    match kf5 {
        Ok(t) => Box::new(t),
        Err(_) => Box::new(DummyTracker::new()),
    }
}
