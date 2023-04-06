use std::env;
use std::error::Error;
use std::path::PathBuf;

use spirv_builder::{MetadataPrintout, SpirvBuilder, SpirvMetadata};

fn main() -> Result<(), Box<dyn Error>> {
	let manifest_dir = env!("CARGO_MANIFEST_DIR");
	let crate_path = [manifest_dir, "..", "example-shader"].iter().copied().collect::<PathBuf>();
	let result = SpirvBuilder::new(crate_path, "spirv-unknown-spv1.3")
		.multimodule(true)
		// this needs at least NameVariables for vulkano to like the spv, but may also be Full
		.spirv_metadata(SpirvMetadata::NameVariables)
		.print_metadata(MetadataPrintout::None)
		.build()?;

	let paths = result.module.unwrap_multi().iter()
		.map(|x| format!("{}: {}", x.0, x.1.to_str().unwrap()))
		.collect::<Vec<String>>().join("\n");
	println!("paths:\n{}", paths);
	println!("entry points: {}", result.codegen_entry_point_strings());

	// uncomment if you want to see build output
	// Err(Box::new(spirv_builder::SpirvBuilderError::BuildFailed))
	Ok(())
}
