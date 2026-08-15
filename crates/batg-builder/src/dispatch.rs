use std::iter;

use cunnybuffers::analysis::SchemaAnalysis;
use cunnybuffers::ir::Schema;
use heck::ToSnakeCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Path as TypePath;

use crate::error::BuildError;

pub struct Entry {
    name: String,
    path: TypePath
}

struct Dispatcher<'a> {
    function: Ident,
    call: Ident,
    output: &'a TokenStream,
    entries: &'a [Entry]
}

impl<'a> Dispatcher<'a> {
    fn new(function: &str, call: &str, output: &'a TokenStream, entries: &'a [Entry]) -> Self {
        Self {
            function: format_ident!("{function}"),
            call: format_ident!("{call}"),
            output,
            entries
        }
    }
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

    let json = quote! { &mut Vec<u8> };
    let sink = quote! { &crate::sink::SinkRef };

    let dispatchers = [
        Dispatcher::new("dispatch_table", "to_json", &json, tables),
        Dispatcher::new("dispatch_row", "to_json", &json, rows),
        Dispatcher::new("visit_table_type", "to_sink_table", &sink, tables),
        Dispatcher::new("visit_row_type", "to_sink_row", &sink, rows)
    ];
    let functions = dispatchers.iter().map(function);

    let tokens = quote! {
        pub const TABLE_TYPES: &[&str] = &[#(#table_names),*];

        pub const ROW_TYPES: &[&str] = &[#(#row_names),*];

        #(#functions)*
    };

    let file = syn::parse2(tokens)?;
    Ok(prettyplease::unparse(&file))
}

fn function(dispatcher: &Dispatcher<'_>) -> TokenStream {
    let Dispatcher {
        function,
        call,
        output,
        entries
    } = dispatcher;
    let arms = entries.iter().map(|entry| arm(entry, call));

    quote! {
        fn #function(
            type_name: &str,
            bytes: &[u8],
            out: #output
        ) -> Option<Result<(), FlatBufferError>> {
            match type_name {
                #(#arms)*
                _ => None
            }
        }
    }
}

fn arm(entry: &Entry, call: &Ident) -> TokenStream {
    let name = &entry.name;
    let path = &entry.path;

    quote! {
        #name => Some(#call::<#path<'_>>(bytes, out)),
    }
}
