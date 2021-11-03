use dasp_graph::{Buffer, Input, Node};

pub struct Saw {
    freq: f32,
    phase: f32,
    sample_rate: f32,
}

impl Saw {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            freq,
            phase: 0.0,
            sample_rate: sample_rate as f32,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate as f32;
    }

    pub fn get_sample(&mut self, v_oct: Option<f32>) -> f32 {
        let freq = self.freq * 2_f32.powf(v_oct.unwrap_or_default());

        let t = 1.0 / self.sample_rate;
        self.phase = (self.phase + freq * t) % 1.0;

        (self.phase * 2.0) - 1.0
    }
}

impl Node for Saw {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match inputs.len() {
            0 => {
                for i in 0..Buffer::LEN {
                    for buffer in output.iter_mut() {
                        let sample = self.get_sample(None);
                        buffer[i] = sample;
                    }
                }
            }
            1 => {
                for i in 0..Buffer::LEN {
                    if inputs[0].buffers().len() != 1 {
                        panic!();
                    }

                    let v_oct_buf = &inputs[0].buffers()[0];

                    for buffer in output.iter_mut() {
                        let v_oct = v_oct_buf[i];
                        let sample = self.get_sample(Some(v_oct));
                        buffer[i] = sample;
                    }
                }
            }
            _ => panic!(),
        }
    }
}
