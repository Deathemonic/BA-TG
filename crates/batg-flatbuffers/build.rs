use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    batg_builder::Build::new().schema_dir("../../schema").run()?;
    Ok(())
}
