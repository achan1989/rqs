use std::time::{Instant, Duration};

use parms;


pub struct Time {
    // C: curtime
    // C: lastcurtime
    cur_time: f64,
    last_cur_time: f64,

    first: bool,
    old_time: Instant,
    same_time_count: u32
}

impl Time {
    // C: Sys_InitFloatTime
    pub fn new(parms: &parms::Parms) -> Self {
        let cur_time = parms.get_parm_value("-starttime").unwrap_or(0.0);
        Time {
            cur_time,
            last_cur_time: cur_time,
            first: true,
            old_time: Instant::now(),
            same_time_count: 0
        }
    }

    pub fn float_time(&mut self) -> f64 {
        let now = Instant::now();

        if self.first {
            self.old_time = now;
            self.first = false;
        } else {
            let elapsed = now.duration_since(self.old_time);
            self.old_time = now;
            self.cur_time += duration_to_f64(elapsed);

            if self.cur_time == self.last_cur_time {
                self.same_time_count += 1;
                if self.same_time_count > 100000 {
                    self.cur_time += 1.0;
                    self.same_time_count = 0;
                }
            } else {
                self.same_time_count = 0;
            }

            self.last_cur_time = self.cur_time;
        }

        self.cur_time
    }
}

fn duration_to_f64(duration: Duration) -> f64 {
    0.0 + (duration.as_secs() as f64)
        + (f64::from(duration.subsec_nanos()) / f64::from(1_000_000_000))
}
