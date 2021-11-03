use dasp_graph::{node::Pass, Buffer, Input, Node};

pub struct PassOrDefault<T: Node> {
    pass: Pass,
    default: T,
}

impl<T: Node> PassOrDefault<T> {
    pub fn new(default: T) -> Self {
        Self {
            pass: Pass,
            default,
        }
    }
}

impl<T: Node> Node for PassOrDefault<T> {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        if inputs.is_empty() {
            self.default.process(inputs, output);
        } else {
            self.pass.process(inputs, output);
        }
    }
}
