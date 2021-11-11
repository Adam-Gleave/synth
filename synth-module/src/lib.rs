pub use synth_module_derive::SynthModule;

use dasp_graph::{BoxedNode, NodeData};
use petgraph::Directed;

pub mod oscillator;
pub mod port;
pub mod sequencer;

type Graph = petgraph::Graph<NodeData<BoxedNode>, (), Directed, u32>;

pub trait SynthModule {
    fn build_graph(self, graph: &mut Graph) -> Self;
}
