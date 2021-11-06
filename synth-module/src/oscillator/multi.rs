use crate::{port::ModuleIO, Graph};

use synth_node::{
    source::{Level, Saw, Sine, Square, Triangle},
    util::PassOrDefault,
};

use petgraph::graph::NodeIndex;

pub struct MultiOscillator {
    v_oct: ModuleIO<PassOrDefault<Level>>,
    sine: ModuleIO<Sine>,
    square: ModuleIO<Square>,
    saw: ModuleIO<Saw>,
    triangle: ModuleIO<Triangle>,
}

impl MultiOscillator {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            v_oct: ModuleIO::disconnected(PassOrDefault::new(Level::new(0.0))),
            sine: ModuleIO::disconnected(Sine::new(freq, sample_rate)),
            square: ModuleIO::disconnected(Square::new(freq, sample_rate)),
            saw: ModuleIO::disconnected(Saw::new(freq, sample_rate)),
            triangle: ModuleIO::disconnected(Triangle::new(freq, sample_rate)),
        }
    }

    pub fn build_graph(mut self, graph: &mut Graph) -> Self {
        self.v_oct.connect(graph);
        self.sine.connect(graph);
        self.square.connect(graph);
        self.saw.connect(graph);
        self.triangle.connect(graph);

        graph.add_edge(self.v_oct.index().unwrap(), self.sine.index().unwrap(), ());
        graph.add_edge(
            self.v_oct.index().unwrap(),
            self.square.index().unwrap(),
            (),
        );
        graph.add_edge(self.v_oct.index().unwrap(), self.saw.index().unwrap(), ());
        graph.add_edge(
            self.v_oct.index().unwrap(),
            self.triangle.index().unwrap(),
            (),
        );

        self
    }

    pub fn v_oct_in(&self) -> Option<NodeIndex<u32>> {
        self.v_oct.index()
    }

    pub fn sine_out(&self) -> Option<NodeIndex<u32>> {
        self.sine.index()
    }

    pub fn square_out(&self) -> Option<NodeIndex<u32>> {
        self.square.index()
    }

    pub fn saw_out(&self) -> Option<NodeIndex<u32>> {
        self.saw.index()
    }

    pub fn triangle_out(&self) -> Option<NodeIndex<u32>> {
        self.triangle.index()
    }
}
