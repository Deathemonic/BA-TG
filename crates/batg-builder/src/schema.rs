use std::fs;
use std::path::Path;

use bumpalo::Bump;
use cunnybuffers::r#gen::rust::{RustConfig, RustGenerator};
use cunnybuffers::r#gen::{GeneratedFile, Generator};
use cunnybuffers::ir::Schema;
use cunnybuffers::parser;

use crate::error::BuildError;

pub fn parse<'s>(dir: &Path, bump: &'s Bump) -> Result<Schema<'s>, BuildError> {
    let mut paths = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort_unstable();

    let mut schema = Schema::default();
    for path in paths
        .iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("fbs"))
    {
        let source = bump.alloc_str(&fs::read_to_string(path)?);
        schema.merge(parser::parse(source, Some(path), bump)?);
    }

    schema.apply_root_type();
    Ok(schema)
}

pub fn generate(schema: &Schema<'_>, imports: &[String]) -> Vec<GeneratedFile> {
    let generator = RustGenerator::new(RustConfig {
        gen_object_api: false,
        merge_table_excel: true,
        serialize: true,
        imports: imports.to_vec()
    });

    generator.generate(schema)
}
