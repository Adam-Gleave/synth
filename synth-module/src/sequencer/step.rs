use crate::{port::ModuleIO, Graph};

use synth_node::{branch::SequentialSwitch, source::Level, util::PassOrDefault};

use dasp_graph::node::Pass;
use petgraph::graph::NodeIndex;

pub struct StepSequencer<const N: usize> {
    clock_in: ModuleIO<Pass>,
    levels: [ModuleIO<PassOrDefault<Level>>; N],
    level_switch: ModuleIO<SequentialSwitch>,
    v_oct_out: ModuleIO<Pass>,
}

impl<const N: usize> StepSequencer<N> {
    pub fn new(levels: [Level; N]) -> Self {
        let clock_in = ModuleIO::disconnected(Pass);
        let level_switch = ModuleIO::disconnected(SequentialSwitch::new(N));
        let levels = levels.map(|level| ModuleIO::disconnected(PassOrDefault::new(level)));
        let v_oct_out = ModuleIO::disconnected(Pass);

        Self {
            clock_in,
            level_switch,
            levels,
            v_oct_out,
        }
    }

    pub fn build_graph(mut self, graph: &mut Graph) -> Self {
        self.clock_in.connect(graph);
        self.level_switch.connect(graph);

        for level in self.levels.iter_mut() {
            level.connect(graph);
        }

        self.v_oct_out.connect(graph);

        for level in self.levels.iter().rev() {
            graph.add_edge(
                level.index().unwrap(),
                self.level_switch.index().unwrap(),
                (),
            );
        }

        graph.add_edge(self.clock_in.index().unwrap(), self.level_switch.index().unwrap(), ());
        graph.add_edge(self.level_switch.index().unwrap(), self.v_oct_out.index().unwrap(), ());

        self
    }

    pub fn clock_in(&self) -> Option<NodeIndex<u32>> {
        self.clock_in.index()
    }

    pub fn v_oct_in(&self, index: usize) -> Option<NodeIndex<u32>> {
        self.levels.get(index).and_then(|level| level.index())
    }

    pub fn v_oct_out(&self) -> Option<NodeIndex<u32>> {
        self.v_oct_out.index()
    }
}
