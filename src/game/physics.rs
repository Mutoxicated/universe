use glam::Vec3;

pub struct Physics {
    /// in milliseconds
    pub(crate) tick_rate: u64,
    pub(crate) _gravity: Vec3,
}

impl Physics {
    pub fn new(tick_rate_millis: u64, gravity: Vec3) -> Self {
        Self {
            tick_rate: tick_rate_millis,
            _gravity: gravity,
        }
    }
}
