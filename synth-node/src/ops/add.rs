use dasp_graph::{Buffer, Input, Node};

pub struct Add;

impl Node for Add {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if inputs.len() == 2 && output.len() == 1 {
            let a = inputs[0].buffers();
            let b = inputs[1].buffers();

            if a.len() == 1 && b.len() == 1 {
                for i in 0..Buffer::LEN {
                    output[0][i] = a[0][i] + b[0][i];
                }
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    }
}
