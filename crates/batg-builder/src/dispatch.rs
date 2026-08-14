use std::iter;

use cunnybuffers::analysis::SchemaAnalysis;
use cunnybuffers::ir::Schema;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Path as TypePath;

use crate::error::BuildError;

pub struct Entry {
    name: String,
    path: TypePath
}

pub fn collect(
    schema: &Schema<'_>,
    analysis: &SchemaAnalysis<'_>,
    module: &str,
    namespace: &str,
    filter: fn(&str) -> bool
) -> Result<Vec<Entry>, BuildError> {
    let mut names = schema
        .tables
        .iter()
        .filter(|table| table.namespace.as_deref() == Some(namespace))
        .filter(|table| analysis.has_finish(table.name, table.namespace.as_deref()))
        .map(|table| table.name)
        .filter(|name| filter(name))
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();

    names
        .into_iter()
        .map(|name| {
            Ok(Entry {
                name: name.to_owned(),
                path: type_path(module, namespace, name)?
            })
        })
        .collect()
}

fn type_path(module: &str, namespace: &str, name: &str) -> Result<TypePath, BuildError> {
    let modules = namespace.split('.').filter(|part| !part.is_empty()).map(str::to_snake_case);
    let path = iter::once("crate".to_owned())
        .chain(iter::once(module.to_owned()))
        .chain(modules)
        .chain(iter::once(name.to_owned()))
        .collect::<Vec<_>>()
        .join("::");

    Ok(syn::parse_str(&path)?)
}

pub fn render(tables: &[Entry], rows: &[Entry]) -> Result<String, BuildError> {
    let table_names = tables.iter().map(|entry| &entry.name);
    let row_names = rows.iter().map(|entry| &entry.name);
    let table_arms = tables.iter().map(arm);
    let row_arms = rows.iter().map(arm);

    let tokens = quote! {
        pub const TABLE_TYPES: &[&str] = &[#(#table_names),*];

        fn dispatch_table(
            type_name: &str,
            bytes: &[u8],
            out: &mut Vec<u8>
        ) -> Option<Result<(), FlatBufferError>> {
            match type_name {
                #(#table_arms)*
                _ => None
            }
        }

        pub const ROW_TYPES: &[&str] = &[#(#row_names),*];

        fn dispatch_row(
            type_name: &str,
            bytes: &[u8],
            out: &mut Vec<u8>
        ) -> Option<Result<(), FlatBufferError>> {
            match type_name {
                #(#row_arms)*
                _ => None
            }
        }
    };

    let file = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&file))
}

fn arm(entry: &Entry) -> TokenStream {
    let name = &entry.name;
    let path = &entry.path;

    quote! {
        #name => Some(to_json::<#path<'_>>(bytes, out)),
    }
}
