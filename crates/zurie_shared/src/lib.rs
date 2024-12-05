pub use hashbrown;
pub use slotmap;
use slotmap::new_key_type;

pub mod sim_clock;

pub static mut DELTA_TIME: f32 = 0.0;
