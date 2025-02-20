#![deny(clippy::all, clippy::pedantic)]

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, Type};

#[proc_macro_derive(Arguments)]
pub fn arguments_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let interaction_derives = from_interaction_derive(&input.data);

    let expanded = quote! {
        impl Arguments for #struct_name {
            fn from_interaction(options: &[rowifi_models::discord::application::interaction::application_command::CommandDataOption]) -> Result<Self, rowifi_framework::arguments::ArgumentError> {
                use rowifi_models::discord::application::interaction::application_command::CommandDataOption;

                let options = options.iter().map(|c| (c.name.as_str(), c)).collect::<std::collections::HashMap<&str, &CommandDataOption>>();

                #interaction_derives
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn from_interaction_derive(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_decs = fields.named.iter().map(|f| {
                        let name = f.ident.clone().unwrap();
                        let field_name = format!("{name}");
                        let ty = &f.ty;
                        let is_option = is_option(&f.ty);

                        if is_option {
                            quote_spanned! {f.span()=>
                                let #name = match options.get(#field_name).map(|s| <#ty>::from_interaction(s)) {
                                    Some(Ok(s)) => s,
                                    Some(Err(err)) => return Err(err),
                                    None => None
                                };
                            }
                        } else {
                            quote_spanned! {f.span()=>
                                let #name = match options.get(#field_name).map(|s| <#ty>::from_interaction(s)) {
                                    Some(Ok(s)) => s,
                                    Some(Err(err)) => return Err(err),
                                    None => <#ty>::default()
                                };
                            }
                        }
                    });

                let field_names = fields.named.iter().map(|f| f.ident.clone().unwrap());
                quote! {
                    #(#field_decs)*
                    Ok(Self {
                        #(#field_names),*
                    })
                }
            }
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    }
}

fn is_option(type_: &Type) -> bool {
    if let Type::Path(path) = type_ {
        let path = &path.path;
        path.leading_colon.is_none()
            && path.segments.len() == 1
            && path.segments.iter().next().unwrap().ident == "Option"
    } else {
        false
    }
}
