use syn::DeriveInput;

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(note_transaction))]
struct NoteTransactionStructAttributes {
    result_type: syn::Type
}

fn note_transaction_derive_macro2(item: proc_macro2::TokenStream) -> deluxe::Result<proc_macro2::TokenStream> {
    // parse
    let mut ast: DeriveInput = syn::parse2(item)?;

    // extract attribute
    let NoteTransactionStructAttributes{ result_type } = deluxe::extract_attributes(&mut ast)?;

    // define imple variables
    let ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    // generate
    Ok(quote::quote! {
        impl #impl_generics NoteTransaction for #ident #type_generics #where_clause {
            type NoteResult = #result_type;
        }
    })
}

#[proc_macro_derive(NoteTransaction, attributes(note_transaction))]
pub fn note_transaction_derive_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    note_transaction_derive_macro2(item.into()).unwrap().into()
}
