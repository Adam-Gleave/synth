use std::collections::HashMap;

use petgraph::{
    graph::NodeIndex,
    visit::EdgeRef,
    Graph,
};
use proc_macro_error::{abort, proc_macro_error, ResultExt};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, DataStruct, DeriveInput, Ident, LitStr, Token,
};

#[proc_macro_error]
#[proc_macro_derive(SynthModule, attributes(synth_module))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input);

    match SynthModuleImpl::new(input) {
        Ok(impl_struct) => impl_struct.into_token_stream().into(),
        Err(err) => abort!(err),
    }
}

struct SynthModuleImpl {
    name: Ident,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    connections: Vec<Connection>,
}

impl SynthModuleImpl {
    fn new(input: DeriveInput) -> syn::Result<Self> {
        match input.data {
            syn::Data::Enum(_) | syn::Data::Union(_) => Err(syn::Error::new(
                input.ident.span(),
                "Cannot derive `SynthModule` for enums or unions.",
            )),
            syn::Data::Struct(data) => Self::impl_struct(input.ident, input.attrs, data),
        }
    }

    fn impl_struct(name: Ident, _attrs: Vec<Attribute>, data: DataStruct) -> syn::Result<Self> {
        let module_attrs = data
            .fields
            .iter()
            .map(|field| {
                field.attrs.iter().map(|attr| {
                    if attr.path.is_ident("synth_module") {
                        let module_attr = attr.parse_args_with(|parse_stream: ParseStream| {
                            let ident = parse_stream.parse::<Ident>().unwrap_or_abort();
                            parse_stream.parse::<Token![=]>().unwrap_or_abort();

                            let parsed = match ident.to_string().as_str() {
                                "input" => SynthModuleAttr::Input(
                                    parse_stream.parse::<Input>().unwrap_or_abort(),
                                ),
                                "output" => SynthModuleAttr::Output(
                                    parse_stream.parse::<Output>().unwrap_or_abort(),
                                ),
                                "connect" => SynthModuleAttr::Connection(
                                    match &field.ident {
                                        Some(src_ident) => Connection::parse_with_src(src_ident.clone(), parse_stream).unwrap_or_abort(),
                                        None => abort!(ident, "\"connect\" attribute can only be used with named fields"),
                                    }
                                ),
                                other => {
                                    let msg = format!("invalid attribute \"{}\"", other);
                                    abort!(
                                        ident,
                                        "{}", msg; 
                                        help = "valid attributes are: \"input\", \"output\", \"connect\""
                                    );
                                }
                            };

                            Ok(parsed)
                        }).unwrap_or_abort();

                        Some(module_attr)
                    } else {
                        None
                    }
                })
                .filter_map(|attr| attr)
                .collect::<Vec<SynthModuleAttr>>()
            })
            .flatten()
            .collect::<Vec<SynthModuleAttr>>();

        let mut inputs = vec![];
        let mut outputs = vec![];
        let mut connections = vec![];

        for attr in module_attrs {
            match attr {
                SynthModuleAttr::Input(input) => inputs.push(input.clone()),
                SynthModuleAttr::Output(output) => outputs.push(output.clone()),
                SynthModuleAttr::Connection(connection) => connections.push(connection.clone()),
            }
        }

        Ok(Self {
            name,
            inputs,
            outputs,
            connections,
        })
    }
}

impl ToTokens for SynthModuleImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            inputs,
            outputs,
            connections,
        } = &self;

        let input_getters = inputs
            .iter()
            .map(|input| {
                let field_name = format_ident!("{}", input.name.value());
                let fn_name = format_ident!("{}_in", field_name);

                quote! {
                    pub fn #fn_name(&self) -> Option<NodeIndex<u32>> {
                        self.#field_name.index()
                    }
                }
            })
            .collect::<Vec<_>>();

        let output_getters = outputs
            .iter()
            .map(|output| {
                let field_name = format_ident!("{}", output.name.value());
                let fn_name = format_ident!("{}_out", field_name);

                quote! {
                    pub fn #fn_name(&self) -> Option<NodeIndex<u32>> {
                        self.#field_name.index()
                    }
                }
            })
            .collect::<Vec<_>>();

        let mut graph = Graph::<String, ()>::new();
        let mut nodes = HashMap::<String, NodeIndex<u32>>::new();

        for connection in connections {
            let src = connection.src.to_string();
            let dst = connection.connected_to.value();

            let src_idx = match nodes.get(&src) {
                Some(src) => *src,
                None => {
                    let idx = graph.add_node(src.clone());
                    nodes.insert(src, idx);
                    idx
                }
            };

            let dst_idx = match nodes.get(&dst) {
                Some(dst) => *dst,
                None => {
                    let idx = graph.add_node(dst.clone());
                    nodes.insert(dst, idx);
                    idx
                }
            };

            graph.add_edge(src_idx, dst_idx, ());
        }

        let mut connect_quotes = Vec::new();

        for input in inputs.iter().map(|input| input.name.value()) {
            let src_ident = format_ident!("{}", input);
            connect_quotes.push(quote! {
                self.#src_ident.connect(graph);
            });

            for output in outputs.iter().map(|output| output.name.value()) {
                for edge in graph.edges_connecting(nodes[&input], nodes[&output]) {
                    let dst_ident = format_ident!("{}", graph[edge.target()]);
                    connect_quotes.push(quote! {
                        self.#dst_ident.connect(graph);
                    });
                }
            }
        }

        for input in inputs.iter().map(|input| input.name.value()) {
            for output in outputs.iter().map(|output| output.name.value()) {
                for edge in graph.edges_connecting(nodes[&input], nodes[&output]) {
                    let src_ident = format_ident!("{}", graph[edge.source()]);
                    let dst_ident = format_ident!("{}", graph[edge.target()]);
                    connect_quotes.push(quote! {
                        graph.add_edge(self.#src_ident.index().unwrap(), self.#dst_ident.index().unwrap(), ());
                    });
                }
            }
        }

        let impl_tokens = quote! {
            impl SynthModule for #name {
                fn build_graph(mut self, graph: &mut Graph) -> Self {
                    #(#connect_quotes)*

                    self
                }
            }

            impl #name {
                #(#input_getters)*
                #(#output_getters)*
            }
        };

        impl_tokens.to_tokens(tokens);
    }
}

#[derive(Clone)]
enum SynthModuleAttr {
    Input(Input),
    Output(Output),
    Connection(Connection),
}

#[derive(Clone)]
struct Input {
    name: LitStr,
}

impl Parse for Input {
    fn parse(parse_input: ParseStream) -> syn::Result<Self> {
        let name = parse_input.parse::<LitStr>()?;
        Ok(Self { name })
    }
}

#[derive(Clone)]
struct Output {
    name: LitStr,
}

impl Parse for Output {
    fn parse(parse_input: ParseStream) -> syn::Result<Self> {
        let name = parse_input.parse::<LitStr>()?;
        Ok(Self { name })
    }
}

#[derive(Debug, Clone)]
struct Connection {
    src: Ident,
    connected_to: LitStr,
}

impl Connection {
    fn parse_with_src(src: Ident, parse_input: ParseStream) -> syn::Result<Self> {
        let connected_to = parse_input.parse::<LitStr>()?;
        Ok(Self { src, connected_to })
    }
}
