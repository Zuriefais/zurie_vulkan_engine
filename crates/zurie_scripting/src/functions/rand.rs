use rand::Rng;
use std::ops::Range;

use super::{zurie::engine, ScriptingState};

impl engine::rand::Host for ScriptingState {
    fn rand_f32(&mut self, start: f32, end: f32) -> f32 {
        rand::thread_rng().gen_range(Range { start, end })
    }

    fn rand_i32(&mut self, start: i32, end: i32) -> i32 {
        rand::thread_rng().gen_range(Range { start, end })
    }
}
