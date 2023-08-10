use std::env;
use std::error::Error;
use std::path::PathBuf;

use spirv_builder::{MetadataPrintout, SpirvBuilder, SpirvMetadata};

const TARGET: &str = "spirv-unknown-vulkan1.2";
const SHADER_PATH: &str = "example-shader";

fn main() -> Result<(), Box<dyn Error>> {
	// main compile
	let manifest_dir = env!("CARGO_MANIFEST_DIR");
	let crate_path = [manifest_dir, "..", SHADER_PATH].iter().copied().collect::<PathBuf>();
	let result = SpirvBuilder::new(crate_path, TARGET)
		.multimodule(true)
		// this needs at least NameVariables for vulkano to like the spv, but may also be Full
		.spirv_metadata(SpirvMetadata::NameVariables)
		.print_metadata(MetadataPrintout::None)
		.build()?;

	// create SHADER_OUT_DIR env var to be read by vulkano shader macro
	let out_dir = env::var("OUT_DIR").unwrap();
	let shader_path_folder = SHADER_PATH.replace('-', "_");
	let shader_out_dir = format!("{out_dir}/../../../../spirv-builder/{TARGET}/release/deps/{shader_path_folder}.spvs/");
	println!("cargo:rustc-env=SHADER_OUT_DIR={shader_out_dir}");

	// print compile results, feel free to add some codegen here
	let paths = result.module.unwrap_multi().iter()
		.map(|x| format!("{}: {}", x.0, x.1.to_str().unwrap()))
		.collect::<Vec<String>>().join("\n");
	println!("OUT_DIR: {}", env::var("OUT_DIR").unwrap());
	println!("paths:\n{}", paths);
	println!("entry points: {}", result.codegen_entry_point_strings());

	// uncomment if you want to see build output
	// Err(Box::new(spirv_builder::SpirvBuilderError::BuildFailed))
	Ok(())
}
