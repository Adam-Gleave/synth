use dasp_graph::{Buffer, Input, Node};

pub struct Mul;

impl Node for Mul {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if inputs.len() == 2 && output.len() == 1 {
            let src = inputs[0].buffers();
            let scale = inputs[1].buffers();

            if src.len() == 1 && scale.len() == 1 {
                for i in 0..Buffer::LEN {
                    output[0][i] = src[0][i] * scale[0][i];
                }
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }
}
