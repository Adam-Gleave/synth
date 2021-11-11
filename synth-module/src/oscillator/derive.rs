use crate::{port::ModuleIO, Graph, SynthModule};

use synth_node::{
    source::{Level, Saw, Sine, Square, Triangle},
    util::PassOrDefault,
};

use petgraph::graph::NodeIndex;

#[derive(SynthModule)]
pub struct DeriveOscillator {
    #[synth_module(input)]
    #[synth_module(connect = "sine", "square", "saw", "triangle")]
    v_oct: ModuleIO<PassOrDefault<Level>>,

    #[synth_module(output)]
    sine: ModuleIO<Sine>,

    #[synth_module(output)]
    square: ModuleIO<Square>,

    #[synth_module(output)]
    saw: ModuleIO<Saw>,

    #[synth_module(output)]
    triangle: ModuleIO<Triangle>,
}

impl DeriveOscillator {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            v_oct: ModuleIO::new(PassOrDefault::new(Level::new(0.0))),
            sine: ModuleIO::new(Sine::new(freq, sample_rate)),
            square: ModuleIO::new(Square::new(freq, sample_rate)),
            saw: ModuleIO::new(Saw::new(freq, sample_rate)),
            triangle: ModuleIO::new(Triangle::new(freq, sample_rate)),
        }
    }
}
