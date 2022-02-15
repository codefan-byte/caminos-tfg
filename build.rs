/// --- Build script ---
///To be executed before compiling sources.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;


fn main()
{
	let out_dir= env::var_os("OUT_DIR").unwrap();
	//let dest_path = Path::new(&out_dir).join("automatic_example_file");
	//fs::write(&dest_path,"Hello\n");
	//e.g. ./target/release/build/simulator-9503fa252f4d438d/out/automatic_example_file
	//To get current commit$ git rev-parse --verify HEAD
	//To get current state$ git describe --dirty --always --all
	let git_describe=Command::new("git")
		//.current_dir(directory)
		.arg("describe")
		.arg("--dirty")
		.arg("--always")
		.arg("--all")
		.output();
	let git_rev=Command::new("git")
		//.current_dir(directory)
		.arg("rev-parse")
		.arg("--verify")
		.arg("HEAD")
		.output();
	let id_str=vec![git_describe,git_rev].iter().filter_map(|x|match x{
		//Ok(output) => Some(String::from_utf8_lossy(&output.stdout)),
		Ok(output) => Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
		Err(_) => None,
	}).collect::<Vec<String>>().join("-");
	let git_id_path = Path::new(&out_dir).join("generated_git_id");
	fs::write(&git_id_path,id_str).expect("failed to write git_id");

	// alternatively use https://stackoverflow.com/questions/27840394/how-can-a-rust-program-access-metadata-from-its-cargo-package
	// This code is incorrect when the Cargo.toml has several `version=XXX` lines. And it may the first, last, or whatever.
	// Better to use option_env!("CARGO_PKG_VERSION")
	//let grep_version=Command::new("grep")
	//	.arg("version")
	//	.arg("Cargo.toml")
	//	.output();
	//let version : String = grep_version.ok().and_then(|output|{
	//	let version_line = String::from_utf8_lossy(&output.stdout).trim().to_string();
	//	let version_line = version_line.lines().next().expect("The first line should be the version of caminos-lib");
	//	if let Some( (_left,right) ) = version_line.split_once('=') {
	//		Some(right.trim().trim_matches(|c| c=='\"' || c=='\'').to_string())
	//	} else { None }
	//}).unwrap_or_else(||"?".to_string());
	//let version_number_path = Path::new(&out_dir).join("generated_version_number");
	//fs::write(&version_number_path,version).expect("failed to write version number");

	//println!("cargo:rerun-if-changed=build.rs");
}

