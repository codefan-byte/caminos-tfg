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
	//println!("cargo:rerun-if-changed=build.rs");
}

