/*!

This module is for managing errors in the code of caminos-lib. To avoid invoking `panic!` in favor of a more graceful exit. Cases that should never happen can be kept as `panic!`.

Instead of `expect` or `unwrap_or_else` try
* `map_err` like in `.map_err(|e|Error::command_not_found(source_location!(),"squeue".to_string(),e))?;`
* `ok_or_else` like in `.ok_or_else( ||Error::new(source_location!(),ErrorKind::NonsenseCommandOutput) )?;

Instead of `panic!` try
* Return an error. E.g., by `return Err( Error::new(source_location!(),ErrorKind::NonsenseCommandOutput) );`

*/

use std::fmt::{Display,Formatter};
use std::path::PathBuf;

/// The main Error class to be used in each `Result(Whatever,Error)`.
/// It contains the code source of the error and its kind.
/// An arbitrary `String` message can be optionally attached.
#[derive(Debug)]
pub struct Error
{
	pub source_location: SourceLocation,
	pub kind: ErrorKind,
	pub message: Option<String>,
}

/// A source code location where an error occurred.
/// Contains the values of the macros `std::{file,line,column}`.
#[derive(Debug)]
pub struct SourceLocation
{
	pub file: &'static str,
	pub line: u32,
	pub column: u32,
}

#[derive(Debug)]
pub enum ErrorKind
{
	/// Some command failed because its binary was not found.
	/// Keep the path and original error.
	CommandNotFound{
		path: String,
		io_error: std::io::Error,
	},
	/// We could not make sense of the output of some command
	NonsenseCommandOutput,
	/// We looked for some environment variable, but did not find it.
	/// keep the variable name and original error
	MissingEnvironmentVariable{
		variable: String,
		var_error: std::env::VarError,
	},
	CouldNotStartSftpSession{
		error:ssh2::Error,
	},
	CouldNotOpenFile{
		filepath: PathBuf,
		error:std::io::Error,
	},
	CouldNotOpenRemoteFile{
		filepath: PathBuf,
		error:ssh2::Error,
	},
	CouldNotParseFile{
		filepath: PathBuf,
	},
	/// Any other error. Better to add new types than to use this thing.
	Undetermined,
}

// source_location!()
#[macro_export]
macro_rules! source_location{
	() => {
		SourceLocation{
			file: file!(),
			line: line!(),
			column: column!(),
		}
	}
}

use ErrorKind::*;

impl Error
{
	pub fn new(source_location:SourceLocation, kind:ErrorKind) -> Error
	{
		Error{
			source_location,
			kind,
			message:None,
		}
	}
	pub fn with_message(mut self,message:String) -> Error
	{
		self.message=Some(message);
		self
	}
	/// example call: Error::new_command_not_found(source_location!(),"squeue".to_string(),e).
	pub fn command_not_found(source_location:SourceLocation,path:String,io_error:std::io::Error)->Error
	{
		Error{
			source_location,
			kind: CommandNotFound{
				path,
				io_error,
			},
			message:None,
		}
	}
	pub fn nonsense_command_output(source_location:SourceLocation)->Error
	{
		Error{
			source_location,
			kind: NonsenseCommandOutput,
			message:None,
		}
	}
	pub fn missing_environment_variable(source_location:SourceLocation,variable:String,var_error:std::env::VarError)->Error
	{
		Error{
			source_location,
			kind: MissingEnvironmentVariable{
				variable,
				var_error,
			},
			message:None,
		}
	}
	pub fn could_not_start_sftp_session(source_location:SourceLocation,error:ssh2::Error)->Error
	{
		Error{
			source_location,
			kind: CouldNotStartSftpSession{
				error,
			},
			message:None,
		}
	}
	pub fn could_not_open_file(source_location:SourceLocation,filepath:PathBuf,error:std::io::Error)->Error
	{
		Error{
			source_location,
			kind: CouldNotOpenFile{
				filepath,
				error,
			},
			message:None,
		}
	}
	pub fn could_not_open_remote_file(source_location:SourceLocation,filepath:PathBuf,error:ssh2::Error)->Error
	{
		Error{
			source_location,
			kind: CouldNotOpenRemoteFile{
				filepath,
				error,
			},
			message:None,
		}
	}
	pub fn could_not_parse_file(source_location:SourceLocation,filepath:PathBuf)->Error
	{
		Error{
			source_location,
			kind: CouldNotParseFile{
				filepath,
			},
			message:None,
		}
	}
	pub fn undetermined(source_location:SourceLocation)->Error
	{
		Error{
			source_location,
			kind: Undetermined,
			message:None,
		}
	}
}


impl Display for Error
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error>
	{
		let Error{source_location:location,kind,message} = self;
		writeln!(formatter,"Error at file {} at line {} column {}.",location.file,location.line,location.column)?;
		if let Some(text) = message
		{
			writeln!(formatter,"{}",text)?;
		}
		kind.fmt(formatter)?;
		Ok(())
	}
}

impl Display for ErrorKind
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error>
	{
		match self
		{
			CommandNotFound{path,io_error}=>
			{
				writeln!(formatter,"CommandNotFound error: the command at {} was not found\nio_error: {}",path,io_error)?;
			},
			NonsenseCommandOutput =>
			{
				writeln!(formatter,"NonsenseCommandOutput error: the output of some command could not be understood.")?;
			},
			MissingEnvironmentVariable{variable,var_error} =>
			{
				writeln!(formatter,"MissingEnvironmentVariable error: the environment variable {} could not be accessed\nvar_error: {}",variable,var_error)?;
			},
			CouldNotStartSftpSession{error} =>
			{
				writeln!(formatter,"CouldNotStartSftpSession error: The call to ssh2_session.sftp failed.\nssh2_error: {}",error)?;
			},
			CouldNotOpenFile{filepath,error} =>
			{
				writeln!(formatter,"CouldNotOpenFile error: The file {:?} could not be opened.\nio_error: {}",filepath,error)?;
			},
			CouldNotOpenRemoteFile{filepath,error} =>
			{
				writeln!(formatter,"CouldNotOpenRemoteFile error: The file {:?} at a remote host via ssh2 could not be opened.\nssh2_error: {}",filepath,error)?;
			},
			CouldNotParseFile{filepath} =>
			{
				writeln!(formatter,"CouldNotParseFile error: The file {:?} could not be parsed.",filepath)?;
			},
			Undetermined =>
			{
				writeln!(formatter,"Undetermined error: A generic error. The concrete error should be more specified.")?;
			},
		}
		Ok(())
	}
}


