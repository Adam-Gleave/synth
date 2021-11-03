use dasp_graph::{Buffer, Input, Node};

pub struct Clock {
    interval: u32,
    completed: u32,
}

impl Clock {
    pub const HIGH: f32 = 5.0;
    pub const LOW: f32 = 0.0;

    pub fn new(bpm: f32, sample_rate: u32) -> Self {
        let interval_s = 60.0 / bpm;
        let interval_samples = (interval_s * sample_rate as f32).ceil() as u32;

        Self {
            interval: interval_samples,
            completed: 0,
        }
    }
}

impl Node for Clock {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if !inputs.is_empty() {
            panic!();
        }

        for i in 0..Buffer::LEN {
            let sample = if self.completed == self.interval {
                Self::HIGH
            } else {
                Self::LOW
            };

            self.completed += 1;
            if self.completed > self.interval {
                self.completed = 0;
            }

            for buffer in output.iter_mut() {
                buffer[i] = sample;
            }
        }
    }
}
