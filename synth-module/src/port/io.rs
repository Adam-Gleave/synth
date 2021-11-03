use dasp_graph::{Node, NodeData};
use petgraph::graph::NodeIndex;

pub struct ModuleIO<T: Node + 'static> {
    inner: Impl<T>,
}

enum Impl<T: Node + 'static> {
    Disconnected(Option<T>),
    Connected(NodeIndex<u32>),
}

impl<T: Node + 'static> ModuleIO<T> {
    pub fn connected(index: NodeIndex<u32>) -> Self {
        Self {
            inner: Impl::Connected(index),
        }
    }

    pub fn disconnected(node: T) -> Self {
        Self {
            inner: Impl::Disconnected(Some(node)),
        }
    }

    pub fn connect(&mut self, graph: &mut crate::Graph) {
        let inner = match &mut self.inner {
            Impl::Disconnected(node) => {
                if let Some(node) = node.take() {
                    let idx = graph.add_node(NodeData::boxed1(node));
                    Some(Impl::Connected(idx))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(inner) = inner {
            let _ = std::mem::replace(&mut self.inner, inner);
        }
    }

    pub fn index(&self) -> Option<NodeIndex<u32>> {
        match &self.inner {
            Impl::Disconnected(_) => None,
            Impl::Connected(idx) => Some(*idx),
        }
    }
}
