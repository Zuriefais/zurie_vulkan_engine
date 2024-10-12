pub struct SimClock {
    simulate: bool,
    simulate_ui_togle: bool,
    sim_rate: u16,
    cur_sim: u16,
}

impl Default for SimClock {
    fn default() -> Self {
        SimClock {
            simulate: true,
            simulate_ui_togle: true,
            sim_rate: 0,
            cur_sim: 0,
        }
    }
}

impl SimClock {
    pub fn clock(&mut self) {
        if self.cur_sim == self.sim_rate {
            self.simulate = true;
            self.sim_rate = 0;
        } else if self.simulate_ui_togle {
            self.simulate = false;
            self.sim_rate += 1;
        }
        if !self.simulate_ui_togle {
            self.simulate = false;
        }
    }

    pub fn ui_togles(&mut self) -> (&mut bool, &mut u16, &mut u16) {
        (
            &mut self.simulate_ui_togle,
            &mut self.cur_sim,
            &mut self.sim_rate,
        )
    }

    pub fn simulate(&mut self) -> bool {
        self.simulate
    }
}
