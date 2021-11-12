use crate::attributes::{AttributeImpl, AttributeImplBuilder};

use syn::Field;

pub(crate) struct FieldImpl {
    pub(crate) attributes: Vec<AttributeImpl>,
}

impl FieldImpl {
    pub(crate) fn new(field: &Field) -> syn::Result<Self> {
        let attributes = field
            .attrs
            .iter()
            .map(|attribute| {
                AttributeImplBuilder::new(attribute, &field.ident)
                    .and_then(|builder| builder.build().ok())
            })
            .filter_map(|opt| opt)
            .collect();

        Ok(Self { attributes })
    }
}
