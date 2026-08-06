use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr, LitInt, Token};
use syn::parse::{Parse, ParseStream};

struct IncludeLayersInput {
    start: usize,
    end: usize,
    template: String,
}

impl Parse for IncludeLayersInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start: LitInt = input.parse()?;
        input.parse::<Token![..=]>()?;
        let end: LitInt = input.parse()?;
        input.parse::<Token![,]>()?;
        let template: LitStr = input.parse()?;
        
        Ok(IncludeLayersInput {
            start: start.base10_parse()?,
            end: end.base10_parse()?,
            template: template.value(),
        })
    }
}

#[proc_macro]
pub fn include_layers(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as IncludeLayersInput);
    
    let includes = (input.start..=input.end).map(|i| {
        let path = input.template.replace("{}", &i.to_string());
        quote! { include_bytes!(#path) as &[u8] }
    });

    let expanded = quote! {
        &[
            #(#includes),*
        ]
    };

    TokenStream::from(expanded)
}
