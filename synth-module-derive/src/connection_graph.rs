use crate::attributes::Connection;

use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use std::collections::HashMap;

pub(crate) struct ConnectionGraph {
    pub(crate) fields_graph: Graph<String, ()>,
    pub(crate) fields_to_nodes: HashMap<String, NodeIndex<u32>>,
}

impl ConnectionGraph {
    pub(crate) fn new<'a>(connections: impl Iterator<Item = &'a Connection>) -> Self {
        let mut fields_graph = Graph::<String, ()>::new();
        let mut fields_to_nodes = HashMap::<String, NodeIndex<u32>>::new();

        for connection in connections {
            let src = connection.input_ident.to_string();

            for dst in connection.output_names.iter().cloned() {
                let dst = dst.value();

                let src_idx = match fields_to_nodes.get(&src) {
                    Some(src) => *src,
                    None => {
                        let idx = fields_graph.add_node(src.clone());
                        fields_to_nodes.insert(src.clone(), idx);
                        idx
                    }
                };

                let dst_idx = match fields_to_nodes.get(&dst) {
                    Some(dst) => *dst,
                    None => {
                        let idx = fields_graph.add_node(dst.clone());
                        fields_to_nodes.insert(dst, idx);
                        idx
                    }
                };

                fields_graph.add_edge(src_idx, dst_idx, ());
            }
        }

        Self {
            fields_graph,
            fields_to_nodes,
        }
    }

    pub(crate) fn generate_node_additions(&self, inputs: &Vec<String>, outputs: &Vec<String>) -> TokenStream {
        let mut tokens = TokenStream::new();

        for input in inputs {
            let src_ident = format_ident!("{}", input);

            tokens.extend(quote! {
                self.#src_ident.connect(graph);
            });

            for output in outputs {
                let input_to_output = self
                    .fields_graph
                    .edges_connecting(self.fields_to_nodes[input], self.fields_to_nodes[output]);

                for edge in input_to_output {
                    let dst_ident = format_ident!("{}", self.fields_graph[edge.target()]);

                    tokens.extend(quote! {
                        self.#dst_ident.connect(graph);
                    });
                }
            }
        }

        tokens
    }

    pub(crate) fn generate_node_connections(&self, inputs: &Vec<String>, outputs: &Vec<String>) -> TokenStream {
        let mut tokens = TokenStream::new();

        for input in inputs {
            for output in outputs {
                let input_to_output = self
                    .fields_graph
                    .edges_connecting(self.fields_to_nodes[input], self.fields_to_nodes[output]);

                for edge in input_to_output {
                    let src_ident = format_ident!("{}", self.fields_graph[edge.source()]);
                    let dst_ident = format_ident!("{}", self.fields_graph[edge.target()]);

                    tokens.extend(quote! {
                        graph.add_edge(self.#src_ident.index().unwrap(), self.#dst_ident.index().unwrap(), ());
                    });
                }
            }
        }

        tokens
    }
}
