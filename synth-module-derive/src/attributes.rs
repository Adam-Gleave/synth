use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::ParseStream, punctuated::Punctuated, Attribute, Ident, LitStr, Token};

use std::collections::HashSet;

#[derive(Clone)]
pub(crate) enum AttributeImpl {
    Input(Input),
    Output(Output),
    Connection(Connection),
}

pub(crate) struct AttributeImplBuilder {
    attribute: Option<Attribute>,
    field_ident: Option<Ident>,
}

impl AttributeImplBuilder {
    pub(crate) fn new(attribute: &Attribute, field_ident: &Option<Ident>) -> Option<Self> {
        if !attribute.path.is_ident("synth_module") {
            return None;
        }

        Some(Self {
            attribute: Some(attribute.clone()),
            field_ident: field_ident.clone(),
        })
    }

    pub(crate) fn build(mut self) -> syn::Result<AttributeImpl> {
        let attribute = self.attribute.take().unwrap();
        let field_ident = self.field_ident.take();

        attribute.parse_args_with(|parse_input: ParseStream| {
            let ident = parse_input.parse::<Ident>()?;

            let field_ident = match field_ident {
                Some(field_ident) => field_ident,
                None => abort!(
                    Span::call_site(),
                    "attribute can only be used with named fields"
                ),
            };

            let parsed = match ident.to_string().as_str() {
                Input::KEY => AttributeImpl::Input(Input { field_ident }),
                Output::KEY => AttributeImpl::Output(Output { field_ident }),
                Connection::KEY => {
                    parse_input.parse::<Token![=]>()?;
                    AttributeImpl::Connection(Connection::parse_with_src(parse_input, field_ident)?)
                }
                other => {
                    let msg = format!("invalid attribute \"{}\"", other);
                    abort!(
                        field_ident,
                        "{}", msg;
                        help = "valid attributes are: \"input\", \"output\", \"connect\""
                    );
                }
            };

            Ok(parsed)
        })
    }
}

#[derive(Clone)]
pub(crate) struct Input {
    pub(crate) field_ident: Ident,
}

impl Input {
    pub(crate) const KEY: &'static str = "input";

    const ACCESSOR_SUFFIX: &'static str = "_in";
}

impl ToString for Input {
    fn to_string(&self) -> String {
        self.field_ident.to_string()
    }
}

impl ToTokens for Input {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let field_name = format_ident!("{}", self.field_ident.to_string());
        let fn_name = format_ident!("{}{}", field_name, Self::ACCESSOR_SUFFIX);

        let input_tokens = quote! {
            pub fn #fn_name(&self) -> Option<NodeIndex<u32>> {
                self.#field_name.index()
            }
        };

        input_tokens.to_tokens(tokens);
    }
}

#[derive(Clone)]
pub(crate) struct Output {
    pub(crate) field_ident: Ident,
}

impl Output {
    pub(crate) const KEY: &'static str = "output";

    const ACCESSOR_SUFFIX: &'static str = "_out";
}

impl ToString for Output {
    fn to_string(&self) -> String {
        self.field_ident.to_string()
    }
}

impl ToTokens for Output {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let field_name = format_ident!("{}", self.field_ident.to_string());
        let fn_name = format_ident!("{}{}", field_name, Self::ACCESSOR_SUFFIX);

        let output_tokens = quote! {
            pub fn #fn_name(&self) -> Option<NodeIndex<u32>> {
                self.#field_name.index()
            }
        };

        output_tokens.to_tokens(tokens);
    }
}

#[derive(Clone)]
pub(crate) struct Connection {
    pub(crate) input_ident: Ident,
    pub(crate) output_names: HashSet<LitStr>,
}

impl Connection {
    pub(crate) const KEY: &'static str = "connect";
}

impl Connection {
    pub(crate) fn parse_with_src(
        parse_input: ParseStream,
        input_ident: Ident,
    ) -> syn::Result<Self> {
        let output_names = HashSet::from_iter(
            Punctuated::<LitStr, Token![,]>::parse_separated_nonempty(parse_input)?.into_iter(),
        );

        Ok(Self {
            input_ident,
            output_names,
        })
    }
}
