use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemStruct};

#[proc_macro_derive(Regel)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(input);
    TokenStream::from(quote! {
        impl Regel for #ident {
            fn id(&self) -> &'static str {
                stringify!(#ident)
            }
        }
    })
}

#[proc_macro_attribute]
pub fn regel(_metadata: TokenStream, _input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(_input as ItemStruct);
    let struct_name = &input.ident; // Get the name of the struct

    // Constructing the output TokenStream using the quote! macro
    // The quote! macro allows for writing Rust code as if it were a string,
    // but with the ability to interpolate values
    TokenStream::from(quote! {
        // Derive Debug trait for #struct_name to enable formatted output with `println()`
        #[derive(Debug)]
        // Defining a new struct #struct_name with two fields: foo and bar
        struct #struct_name {
            foo: i32,
            bar: i32,
        }

        // Implementing the Default trait for #struct_name
        // This provides a default() method to create a new instance of #struct_name
        impl Default for #struct_name {
            // The default method returns a new instance of #struct_name
            // with foo set to 10 and bar set to 20
            fn default() -> Self {
                #struct_name { foo: 10, bar: 20}
            }
        }

        impl #struct_name {
            // Defining a method double_foo for #struct_name
            // This method returns double the value of foo
            fn double_foo(&self) -> i32 {
                self.foo * 2
            }
        }
    })
}
