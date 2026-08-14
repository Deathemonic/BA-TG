pub mod dispatch;
pub mod error;
pub mod schema;
pub mod sync;

use std::path::{Path, PathBuf};
use std::{env, fs};

use bumpalo::Bump;
use cunnybuffers::analysis::SchemaAnalysis;
pub use error::BuildError;

pub struct Build {
    schema_dir: PathBuf,
    generated_dir: PathBuf,
    module: String,
    table_namespace: String,
    row_namespace: String,
    imports: Vec<String>
}

impl Default for Build {
    fn default() -> Self {
        Self {
            schema_dir: PathBuf::from("schema"),
            generated_dir: PathBuf::from("src/flatbuffers"),
            module: "flatbuffers".into(),
            table_namespace: "FlatData".into(),
            row_namespace: "MX.Data.Excel".into(),
            imports: vec!["bacy::table_encryption_service".into()]
        }
    }
}

impl Build {
    pub fn new() -> Self { Self::default() }

    pub fn schema_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.schema_dir = dir.into();
        self
    }

    pub fn generated_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.generated_dir = dir.into();
        self
    }

    pub fn module(mut self, module: impl Into<String>) -> Self {
        self.module = module.into();
        self
    }

    pub fn table_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.table_namespace = namespace.into();
        self
    }

    pub fn row_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.row_namespace = namespace.into();
        self
    }

    pub fn imports(mut self, imports: Vec<String>) -> Self {
        self.imports = imports;
        self
    }

    pub fn run(self) -> Result<(), BuildError> {
        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", self.schema_dir.display());

        let bump = Bump::new();
        let schema = schema::parse(&self.schema_dir, &bump)?;

        sync::sync(&self.generated_dir, &schema::generate(&schema, &self.imports))?;

        let analysis = SchemaAnalysis::new(&schema);
        let tables =
            dispatch::collect(&schema, &analysis, &self.module, &self.table_namespace, |name| {
                name.ends_with("Table")
            })?;
        let rows =
            dispatch::collect(&schema, &analysis, &self.module, &self.row_namespace, |_| true)?;

        let output = Path::new(&env::var("OUT_DIR")?).join("tables.rs");
        fs::write(output, dispatch::render(&tables, &rows)?)?;
        Ok(())
    }
}
