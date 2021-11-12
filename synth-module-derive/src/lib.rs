mod attributes;
mod connection_graph;
mod fields;

use attributes::{AttributeImpl, Input, Output, Connection};
use fields::FieldImpl;
use connection_graph::ConnectionGraph;

use proc_macro2::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Attribute, DataStruct, DeriveInput, Ident};


#[proc_macro_error]
#[proc_macro_derive(SynthModule, attributes(synth_module))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input);

    match StructImpl::new(input) {
        Ok(impl_struct) => impl_struct.into_token_stream().into(),
        Err(err) => abort!(err),
    }
}

struct StructImpl {
    name: Ident,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    connections: Vec<Connection>,
}

impl StructImpl {
    fn new(input: DeriveInput) -> syn::Result<Self> {
        match input.data {
            syn::Data::Enum(_) | syn::Data::Union(_) => Err(syn::Error::new(
                input.ident.span(),
                "cannot derive `SynthModule` for enums or unions.",
            )),
            syn::Data::Struct(data) => Self::impl_struct(input.ident, input.attrs, data),
        }
    }

    fn impl_struct(name: Ident, _attrs: Vec<Attribute>, data: DataStruct) -> syn::Result<Self> {
        let fields = data
            .fields
            .iter()
            .filter_map(|field| FieldImpl::new(field).ok())
            .collect::<Vec<_>>();

        let mut inputs = vec![];
        let mut outputs = vec![];
        let mut connections = vec![];

        for field in fields {
            for attribute in field.attributes {
                match attribute {
                    AttributeImpl::Input(input) => inputs.push(input.clone()),
                    AttributeImpl::Output(output) => outputs.push(output.clone()),
                    AttributeImpl::Connection(connection) => connections.push(connection.clone()),
                }
            }
        }

        Ok(Self {
            name,
            inputs,
            outputs,
            connections,
        })
    }

    fn input_field_names(&self) -> impl Iterator<Item = String> + '_ {
        self.inputs.iter().map(ToString::to_string)
    }

    fn output_field_names(&self) -> impl Iterator<Item = String> + '_ {
        self.outputs.iter().map(ToString::to_string)
    }

    fn generate_input_accessors(&self) -> TokenStream {
        let inputs_accessors = self
            .inputs
            .iter()
            .map(|input| input.to_token_stream())
            .collect::<Vec<_>>();

        quote! { #(#inputs_accessors)* }
    }

    fn generate_output_accessors(&self) -> TokenStream {
        let outputs_accessors = self
            .outputs
            .iter()
            .map(|output| output.to_token_stream())
            .collect::<Vec<_>>();

        quote! { #(#outputs_accessors)* }
    }
}

impl ToTokens for StructImpl {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let input_accessors = self.generate_input_accessors();
        let output_accessors = self.generate_output_accessors();

        let input_field_names = self.input_field_names().collect::<Vec<_>>();
        let output_field_names = self.output_field_names().collect::<Vec<_>>();

        let connection_graph = ConnectionGraph::new(self.connections.iter());
        let add_audio_graph_nodes = connection_graph
            .generate_node_additions(&input_field_names, &output_field_names);
        let connect_audio_graph_nodes = connection_graph
            .generate_node_connections(&input_field_names, &output_field_names);

        let impl_tokens = quote! {
            impl SynthModule for #name {
                fn build_graph(mut self, graph: &mut Graph) -> Self {
                    #add_audio_graph_nodes
                    #connect_audio_graph_nodes
                    self
                }
            }

            impl #name {
                #input_accessors
                #output_accessors
            }
        };

        impl_tokens.to_tokens(tokens);
    }
}
