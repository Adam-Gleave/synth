use crate::source::Clock;

use dasp_graph::{Buffer, Input, Node};

pub struct SequentialSwitch {
    cycled_inputs: usize,
    current_input: usize,
}

impl SequentialSwitch {
    const CLOCK_INDEX: usize = 0;
    const FIRST_INPUT_INDEX: usize = 1;

    pub fn new(cycled_inputs: usize) -> Self {
        Self {
            cycled_inputs,
            current_input: Self::FIRST_INPUT_INDEX,
        }
    }
}

impl Node for SequentialSwitch {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        for input in inputs {
            if input.buffers().len() > 1 {
                panic!();
            }
        }

        if self.cycled_inputs == 0 {
            for output in output.iter_mut() {
                *output = Buffer::SILENT;
            }

            return;
        }

        let clock_buf = inputs
            .get(Self::CLOCK_INDEX)
            .and_then(|input| input.buffers().get(0))
            .unwrap_or_else(|| &Buffer::SILENT);

        for i in 0..Buffer::LEN {
            let clock = clock_buf[i];

            if clock >= Clock::HIGH {
                self.current_input += 1;

                if self.current_input > self.cycled_inputs {
                    self.current_input = Self::FIRST_INPUT_INDEX;
                }
            }

            let sample = inputs
                .get(self.current_input)
                .and_then(|input| input.buffers().get(0))
                .and_then(|input_buf| input_buf.get(i))
                .unwrap_or_else(|| &0.0);

            for buffer in output.iter_mut() {
                buffer[i] = *sample;
            }
        }
    }
}
