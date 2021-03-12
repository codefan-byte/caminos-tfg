
use std::fmt;
use std::fs::{self,File,OpenOptions};
use std::str::FromStr;
use std::io::prelude::*;
use std::io::{stdout,BufReader};
use std::path::{Path,PathBuf};
use std::process::Command;
use std::net::TcpStream;
use std::collections::{HashSet};

use ssh2::Session;
use indicatif::{ProgressBar,ProgressStyle};

use crate::config_parser::{self,ConfigurationValue};
use crate::{Simulation,Plugs};
use crate::output::{create_output};
use crate::config::{evaluate,flatten_configuration_value};

#[derive(Debug,Clone,Copy)]
pub enum Action
{
	///Default action of executing locally and creating the output files.
	LocalAndOutput,
	///Execute remaining runs locally and sequentially.
	Local,
	///Just generates the output with the available data
	Output,
	///Package the executions into Slurm jobs and send them to the Slurm queue system.
	Slurm,
	///Checks how many results it has.
	///TODO: implement looking at slurm error files.
	Check,
	///Bring results from the remote via sftp.
	Pull,
	///Performs a check action on the remote.
	RemoteCheck,
	///Push data into the remote.
	Push,
	///Cancel all slurm jobs owned by the experiment.
	SlurmCancel,
}

impl FromStr for Action
{
	type Err = ();
	fn from_str(s:&str) -> Result<Action,()>
	{
		match s
		{
			"default" => Ok(Action::LocalAndOutput),
			"local_and_output" => Ok(Action::LocalAndOutput),
			"local" => Ok(Action::Local),
			"output" => Ok(Action::Output),
			"slurm" => Ok(Action::Slurm),
			"check" => Ok(Action::Check),
			"pull" => Ok(Action::Pull),
			"remote_check" => Ok(Action::RemoteCheck),
			"push" => Ok(Action::Push),
			"slurm_cancel" => Ok(Action::SlurmCancel),
			_ => Err(()),
		}
	}
}

impl fmt::Display for Action
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "{:?}", *self)
		//match *self
		//{
		//	Action::LocalAndOutput=>write!(f,"LocalAndOutput"),
		//	Action::Local=>write!(f,"Local"),
		//}
	}
}

struct KeyboardInteration;

impl KeyboardInteration
{
	fn ask_password(&self, username: &str, hostname: &str) -> String
	{
		println!("Asking username {} for their password at {}.",username,hostname);
		stdout().lock().flush().unwrap();
		//let mut line = String::new();
		//stdin().lock().read_line(&mut line).unwrap();
		//line
		//XXX We do not want to have echo or similar.
		//FIXME: ^C behaves weird on rpassword
		rpassword::read_password_from_tty(Some("Password: ")).unwrap()
	}
}

impl ssh2::KeyboardInteractivePrompt for KeyboardInteration
{
	fn prompt<'a>(&mut self, username:&str, instructions: &str, prompts: &[ssh2::Prompt<'a>]) -> Vec<String>
	{
		println!("Asking username {} for its password. {}",username,instructions);
		println!("{} prompts?",prompts.len());
		stdout().lock().flush().unwrap();
		//let line = stdin.lock().lines().next().unwrap().unwrap();
		//let mut line = String::new();
		//stdin().lock().read_line(&mut line).unwrap();
		//vec![line]
		vec![rpassword::read_password_from_tty(Some("Password: ")).unwrap()]
	}
}

///Collect the output of
///		$ squeue -ho '%A'
///into a vector.
fn gather_slurm_jobs() -> Result<Vec<usize>,std::io::Error>
{
	let squeue_output=Command::new("squeue")
		//.current_dir(directory)
		.arg("-ho")
		.arg("%A")
		.output()?;
	let squeue_output=String::from_utf8_lossy(&squeue_output.stdout);
	Ok(squeue_output.lines().map(|line|
		//line.parse::<usize>().expect("squeue should give us integers")
		match line.parse::<usize>()
		{
			Ok(x) => x,
			Err(e) =>
			{
				panic!("error {} on parsing line [{}]",e,line);
			},
		}
	).collect())
}

///Simulations to be run in a slurm/other job.
struct Job
{
	execution_code_vec: Vec<String>,
	execution_id_vec: Vec<usize>,
}

impl Job
{
	fn new()->Job
	{
		Job{
			execution_code_vec:vec![],
			execution_id_vec:vec![],
		}
	}

	fn len(&self)->usize
	{
		self.execution_id_vec.len()
	}

	fn add_execution(&mut self, execution_id: usize, binary:&Path, execution_path_str: &str)
	{
		let job_line=format!("echo execution {}\n/bin/date\n{} {}/local.cfg --results={}/local.result",execution_id,binary.display(),execution_path_str,execution_path_str);
		self.execution_code_vec.push(job_line);
		self.execution_id_vec.push(execution_id);
	}

	fn write_slurm_script(&self, out:&mut dyn Write,prefix:&str,slurm_time:&str, slurm_mem:Option<&str>,job_lines:&str)
	{
		// #SBATCH --mem=1000 ?? In megabytes or suffix [K|M|G|T]. See sbatch man page for more info.
		let mem_str = if let Some(s)=slurm_mem { format!("#SBATCH --mem={}\n",s) } else {"".to_string()};
		writeln!(out,"#!/bin/bash
#SBATCH --job-name=simulator
#SBATCH -D .
#SBATCH --output={prefix}-%j.out
#SBATCH --error={prefix}-%j.err
#SBATCH --cpus-per-task=1
#SBATCH --ntasks=1
#SBATCH --time={slurm_time}
{mem_str}
{job_lines}
",prefix=prefix,slurm_time=slurm_time,mem_str=mem_str,job_lines=job_lines).unwrap();
	}

	fn launch_slurm_script(&self, directory:&Path,script_name:&str) -> usize
	{
		let sbatch_output=Command::new("sbatch")
			.current_dir(directory)
			.arg(script_name)
			.output()
			.expect("sbatch failed to start");
		//Should be something like "Submitted batch job 382683"
		let mut jobids=vec![];
		//let sbatch_stdout=sbatch_output.stdout.iter().collect::<String>();
		let sbatch_stdout=String::from_utf8_lossy(&sbatch_output.stdout);
		//for word in sbatch_stdout.split(" ")
		for word in sbatch_stdout.split_whitespace()
		{
			match word.parse::<usize>()
			{
				Ok(id) => jobids.push(id),
				Err(_) => (),
			};
		}
		if jobids.len()!=1
		{
			panic!("sbatch executed but we got incorrect jobids ({:?} from {})",jobids,sbatch_stdout);
		}
		jobids[0]
	}
	
	///Creates a slurm script with the jobs and launch them. Returns a description to include in the journal.
	///internal_job_id is the one used in the script files. Currently being the id of the first experiment in the batch.
	///jobs_path is the path where the launch script is created.
	///slurm_time and slurm_mem are passed as slurm arguments.
	fn slurm(&mut self, internal_job_id:usize, jobs_path:&Path, slurm_time:&str, slurm_mem:Option<&str>) -> String
	{
		let job_lines=self.execution_code_vec.join("\n") + "\n/bin/date\necho job finished\n";
		let launch_name=format!("launch{}",internal_job_id);
		let launch_script=jobs_path.join(&launch_name);
		let mut launch_script_file=File::create(&launch_script).expect("Could not create launch file");
		self.write_slurm_script(&mut launch_script_file,&launch_name,slurm_time,slurm_mem,&job_lines);
		let slurm_job_id=self.launch_slurm_script(&jobs_path,&launch_name);
		//FIXME: we also need the execution ids inside that job.
		//let execution_id_string=self.execution_id_vec.join(",");
		//let execution_id_string=self.execution_id_vec.iter().map(|id|format!("{}",id)).zip(repeat(",")).collect::<String>();
		let execution_id_string=self.execution_id_vec.iter().map(|id|format!("{}",id)).collect::<Vec<String>>().join(",");
		format!("{}={}[{}], ",slurm_job_id,internal_job_id,execution_id_string)
	}
}

///Options that may modifiy the performed action.
#[non_exhaustive]
#[derive(Default)]
pub struct ExperimentOptions
{
	///Bring matching results from another experiment directory.
	pub external_source: Option<PathBuf>,
	///Experiment index in which to start the actions.
	pub start_index: Option<usize>,
	///Experiment index in which to end the actions (excluded).
	pub end_index: Option<usize>,
	///Expression of expriments to be included.
	pub where_clause: Option<config_parser::Expr>,
	///A message to be written into the log.
	pub message: Option<String>,
}

///An `Experiment` object encapsulates the operations that are performed over a folder containing an experiment.
pub struct Experiment<'a>
{
	//TODO: learn what happens when paths are not UNICODE.
	//TODO: perhaps it should be possible a ssh:// location. Maybe an URL.
	binary: PathBuf,
	root: PathBuf,
	//options: Matches,
	options: ExperimentOptions,
	journal: PathBuf,
	journal_index: usize,
	remote_host: Option<String>,
	remote_username: Option<String>,
	remote_binary: Option<PathBuf>,
	remote_root: Option<PathBuf>,
	ssh2_session: Option<Session>,
	#[allow(dead_code)]
	visible_slurm_jobs: Vec<usize>,
	owned_slurm_jobs: Vec<usize>,
	experiments_on_slurm: Vec<usize>,
	plugs:&'a Plugs,
}

impl<'a> Experiment<'a>
{
	///Creates a new experiment object.
	//pub fn new(binary:&Path,root:&Path,plugs:&'a Plugs,options:&Matches)->Experiment<'a>
	pub fn new(binary:&Path,root:&Path,plugs:&'a Plugs,options:ExperimentOptions)->Experiment<'a>
	{
		println!("Preparing experiment with {:?} as path",root);
		let visible_slurm_jobs:Vec<usize> = gather_slurm_jobs().unwrap_or(vec![]);
		let journal=root.join("journal");
		let journal_file=OpenOptions::new().read(true).write(true).create(true).open(&journal).expect("Something went wrong reading or creating the journal file");
		//let journal_len=journal_file.stream_len();
		//journal.file.seek(SeekFrom::End(0));
		let mut journal_index=0;
		let reader = BufReader::new(journal_file);
		let mut owned_slurm_jobs=vec![];
		let mut experiments_on_slurm=vec![];
		for rline in reader.lines()
		{
			//journal_index= rline.expect("bad line read from journal").split(":").next().expect("Not found the expected journal index").parse().expect("The journal index must be a non-negative integer");
			let line=rline.expect("bad line read from journal");
			if ! line.is_empty()
			{
				//let prefix=line.split(":").next().expect("Not found the expected journal index");
				let mut s = line.split(":");
				let prefix=s.next().expect("Not found the expected journal index");
				journal_index= 1usize+prefix.parse::<usize>().unwrap_or_else(|_|panic!("The journal index must be a non-negative integer (received {})",prefix));
				let entry = s.next().expect("No content found on the journal line");
				if entry.starts_with(" Launched jobs ")
				{
					//e.g:
					//	0: Launched jobs 457688=5[0,1,2,3,4,5], 457689=11[6,7,8,9,10,11], 457690=17[12,13,14,15,16,17], 457691=23[18,19,20,21,22,23],
					let mut slurm_items=entry.split(" ");
					slurm_items.next();//first empty space
					slurm_items.next();//Launched
					slurm_items.next();//jobs
					for slurm_item in slurm_items
					{
						if slurm_item.is_empty()
						{
							continue;
						}
						let mut slurm_pair = slurm_item.split("=");
						let slurm_job_id = slurm_pair.next().unwrap().parse::<usize>().unwrap_or_else(|_|panic!("left term on '{}' should be an integer",slurm_item));
						if visible_slurm_jobs.contains(&slurm_job_id)
						{
							owned_slurm_jobs.push(slurm_job_id);
							let slurm_job_content = slurm_pair.next().unwrap();
							let left_bracket_index = slurm_job_content.find("[").unwrap();
							let right_bracket_index = slurm_job_content.find("]").unwrap();
							let experiments=slurm_job_content[left_bracket_index+1 .. right_bracket_index].split(",").map(|item|item.parse::<usize>().unwrap_or_else(|_|panic!("failed with content={} for item {}",slurm_job_content,slurm_item)));
							experiments_on_slurm.extend(experiments);
						}
					}
				}
				if entry==" message"
				{
					println!("journal message {}",line);
				}
			}
		}
		Experiment{
			binary: binary.to_path_buf(),
			root: root.to_path_buf(),
			options,
			journal,
			journal_index,
			remote_host: None,
			remote_username: None,
			remote_binary: None,
			remote_root: None,
			ssh2_session: None,
			visible_slurm_jobs,
			owned_slurm_jobs,
			experiments_on_slurm,
			plugs,
		}
	}
	/// Appends a new entry to the journal
	fn write_journal_entry(&self, entry:&str)
	{
		let mut journal_file=OpenOptions::new().append(true).open(&self.journal).expect("Something went wrong reading or creating the journal file");
		writeln!(journal_file,"{}: {}",self.journal_index,entry).expect("Could not write to journal");
	}
	/// Executes an action over the experiment.
	pub fn execute_action(&mut self,action:Action)
	{
		let now = chrono::Utc::now();
		self.write_journal_entry(&format!("Executing action {} on {}.", action, now.format("%Y %m(%b) %0d(%a), %T (UTC%:z)").to_string()));
		let cfg=self.root.join("main.cfg");
		//TODO cfg checkum
		//let mut cfg_contents = String::new();
		//let mut cfg_file=File::open(&cfg).expect("main.cfg could not be opened");
		//cfg_file.read_to_string(&mut cfg_contents).expect("something went wrong reading main.cfg");
		let cfg_contents={
			let mut cfg_contents = String::new();
			let mut cfg_file=File::open(&cfg).expect("main.cfg could not be opened");
			cfg_file.read_to_string(&mut cfg_contents).expect("something went wrong reading main.cfg");
			cfg_contents
		};
		let mut results;
		let parsed_cfg=match config_parser::parse(&cfg_contents)
		{
			Err(x) => panic!("error parsing configuration file: {:?}",x),
			Ok(x) => x,
			//println!("parsed correctly: {:?}",x);
		};
		let experiments=match parsed_cfg
		{
			config_parser::Token::Value(ref value) =>
			{
				let flat=flatten_configuration_value(value);
				if let ConfigurationValue::Experiments(experiments)=flat
				{
					experiments
				}
				else
				{
					panic!("there are not experiments");
				}
			},
			_ => panic!("Not a value"),
		};

		let external_experiments = if let Some(ref path) = self.options.external_source
		{
			let cfg = path.join("main.cfg");
			let mut cfg_file=File::open(&cfg).unwrap_or_else(|_|panic!("main.cfg from --source={:?} could not be opened",path));
			let mut cfg_contents = String::new();
			cfg_file.read_to_string(&mut cfg_contents).unwrap_or_else(|_|panic!("something went wrong reading main.cfg from --source={:?}",path));
			let parsed_cfg=match config_parser::parse(&cfg_contents)
			{
				Err(x) => panic!("error parsing configuration file: {:?} from --source={:?}",x,path),
				Ok(x) => x,
				//println!("parsed correctly: {:?}",x);
			};
			//Some(parsed_cfg)
			let experiments = match parsed_cfg
			{
				config_parser::Token::Value(ref value) =>
				{
					let flat=flatten_configuration_value(value);
					if let ConfigurationValue::Experiments(experiments)=flat
					{
						experiments
					}
					else
					{
						panic!("there are not experiments in --source={:?}",path);
					}
				},
				_ => panic!("Not a value in --cource={:?}",path),
			};
			Some(experiments)
		} else {None};
		
		if let Some(message)=&self.options.message
		{
			self.write_journal_entry(&format!("message: {}",message));
		}

		let mut must_draw=false;
		let mut job_pack_size=1;//how many binary runs per job.
		//let mut pending_jobs=vec![];
		let mut job=Job::new();
		let mut slurm_time="0-24:00:00";
		let mut slurm_mem=None;
		let mut uses_jobs=false;
		match action
		{
			Action::LocalAndOutput =>
			{
				must_draw=true;
			},
			Action::Local =>
			{
				must_draw=false;
			},
			Action::Output =>
			{
				must_draw=true;
			},
			Action::Slurm =>
			{
				uses_jobs=true;
				if let config_parser::Token::Value(ref value)=parsed_cfg
				{
					if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=value
					{
						// Configuration {
						//  launch_configurations: [
						//    Slurm {...}
						//  ]
						//}
						if cv_name!="Configuration"
						{
							panic!("A simulation must be created from a `Configuration` object not `{}`",cv_name);
						}
						let mut maximum_jobs=None;
						let mut time =None;
						let mut mem =None;
						let mut option_job_pack_size=None;
						for &(ref name,ref value) in cv_pairs
						{
							match name.as_ref()
							{
								"launch_configurations" => match value
								{
									&ConfigurationValue::Array(ref l) =>
									{
										for lc in l.iter()
										{
											match lc
											{
												&ConfigurationValue::Object(ref launch_name, ref launch_pairs) =>
												{
													if launch_name=="Slurm"
													{
														for &(ref slurm_name,ref slurm_value) in launch_pairs
														{
															match slurm_name.as_ref()
															{
																"maximum_jobs" => match slurm_value
																{
																	&ConfigurationValue::Number(f) => maximum_jobs=Some(f as usize),
																	_ => panic!("bad value for maximum_jobs"),
																}
																"job_pack_size" => match slurm_value
																{
																	&ConfigurationValue::Number(f) => option_job_pack_size=Some(f as usize),
																	_ => panic!("bad value for option_job_pack_size"),
																}
																"time" => match slurm_value
																{
																	//&ConfigurationValue::Literal(ref s) => time=Some(&s[1..s.len()-1]),
																	&ConfigurationValue::Literal(ref s) => time=Some(s.as_ref()),
																	_ => panic!("bad value for time"),
																}
																"mem" => match slurm_value
																{
																	//&ConfigurationValue::Literal(ref s) => mem=Some(&s[1..s.len()-1]),
																	&ConfigurationValue::Literal(ref s) => mem=Some(s.as_ref()),
																	_ => panic!("bad value for mem"),
																}
																_ => (),
															}
														}
													}
												}
												_ => (),//XXX perhaps error on unknown launch configuration?
											}
										}
									},
									_ => panic!("bad value for launch_configurations"),
								}
								_ => (),//the other simulation options
							}
						}
						if let Some(value)=maximum_jobs
						{
							let new_job_pack_size=(experiments.len() + value-1 ) / value;//rounding up of experiments/maximum
							if new_job_pack_size>=job_pack_size
							{
								job_pack_size=new_job_pack_size;
							}
							else
							{
								panic!("Trying to reduce job_pack_size from {} to {}.",job_pack_size,new_job_pack_size);
							}
						}
						if let Some(value)=option_job_pack_size
						{
							if job_pack_size!=1 && value!=1
							{
								panic!("Trying to change job_pack_size unexpectedly");
							}
							job_pack_size = value;
						}
						if let Some(value)=time
						{
							slurm_time=value;
						}
						slurm_mem=mem;
					}
					else
					{
						panic!("Those are not experiments.");
					}
				}
			},
			Action::Check =>
			{
				must_draw=false;
			},
			Action::Pull =>
			{
				self.initialize_remote();
				//Bring the remote files to this machine
				let remote_root=self.remote_root.clone().unwrap();
				//Download remote main.cfg
				let remote_cfg_path = remote_root.join("main.cfg");
				let (mut remote_main_cfg_channel, _stat) = self.ssh2_session.as_ref().unwrap().scp_recv(&remote_cfg_path).unwrap();
				let mut remote_main_cfg_contents=String::new();
				remote_main_cfg_channel.read_to_string(&mut remote_main_cfg_contents).expect("Could not read remote main.cfg");
				println!("local path: {:?}",cfg);
				let host=self.remote_host.as_ref().unwrap();
				let username=self.remote_username.as_ref().unwrap();
				println!("remote ({}@{}) path: {:?}",username,host,remote_cfg_path);
				if cfg_contents == remote_main_cfg_contents
				{
					println!("The configurations match");
				}
				else
				{
					panic!("The configurations do not match.\nYou may try$ vimdiff {:?} scp://{}@{}/{:?}\n",cfg,username,host,remote_cfg_path);
				}
			},
			Action::RemoteCheck =>
			{
				self.initialize_remote();
				let remote_root=self.remote_root.clone().unwrap();
				let remote_binary=self.remote_binary.clone().unwrap();
				let mut channel = self.ssh2_session.as_ref().unwrap().channel_session().unwrap();
				let remote_command = format!("{:?} {:?} --action=check",remote_binary,remote_root);
				channel.exec(&remote_command).unwrap();
				let mut remote_command_output = String::new();
				channel.read_to_string(&mut remote_command_output).unwrap();
				channel.wait_close().expect("Could not close the channel of remote executions.");
				channel.exit_status().unwrap();
				for line in remote_command_output.lines()
				{
					println!("at remote: {}",line);
				}
			},
			Action::Push =>
			{
				self.initialize_remote();
				//Bring the remote files to this machine
				let remote_root=self.remote_root.clone().unwrap();
				//Download remote main.cfg
				let remote_cfg_path = remote_root.join("main.cfg");
				let sftp = self.ssh2_session.as_ref().unwrap().sftp().unwrap();
				match sftp.stat(&remote_root)
				{
					Ok(remote_stat) =>
					{
						if !remote_stat.is_dir()
						{
							panic!("remote {:?} exists, but is not a directory",&remote_stat);
						}
						//let (mut remote_main_cfg_channel, stat) = self.ssh2_session.as_ref().unwrap().scp_recv(&remote_cfg_path).unwrap();
						let mut remote_main_cfg =  sftp.open(&remote_cfg_path).expect("Could not open remote main.cfg");
						let mut remote_main_cfg_contents=String::new();
						remote_main_cfg.read_to_string(&mut remote_main_cfg_contents).expect("Could not read remote main.cfg.");
						println!("local path: {:?}",cfg);
						let host=self.remote_host.as_ref().unwrap();
						let username=self.remote_username.as_ref().unwrap();
						println!("remote ({}@{}) path: {:?}",username,host,remote_cfg_path);
						if cfg_contents == remote_main_cfg_contents
						{
							println!("The configurations match");
						}
						else
						{
							panic!("The configurations do not match.\nYou may try$ vimdiff {:?} scp://{}@{}/{:?}\n",cfg,username,host,remote_cfg_path);
						}
					},
					Err(_err) =>
					{
						println!("Could not open remote '{:?}', creating it",remote_root);
						sftp.mkdir(&remote_root,0o755).expect("Could not create remote directory");
						let mut remote_cfg = sftp.create(&remote_cfg_path).expect("Could not create remote main.cfg");
						write!(remote_cfg,"{}",cfg_contents).expect("Could not write into remote main.cfg");
						let mut remote_od = sftp.create(&remote_root.join("main.od")).expect("Could not create remote main.od");
						let mut local_od = File::open(self.root.join("main.od")).expect("Could not open local main.od");
						let mut od_contents = String::new();
						local_od.read_to_string(&mut od_contents).expect("something went wrong reading main.od");
						write!(remote_od,"{}",od_contents).expect("Could not write into remote main.od");
					},
				};
			},
			Action::SlurmCancel =>
			{
				//Cancel all jobs on owned_slurm_jobs
				let mut scancel=&mut Command::new("scancel");
				for jobid in self.owned_slurm_jobs.iter()
				{
					scancel = scancel.arg(jobid.to_string());
				}
				scancel.output().expect("scancel failed");
			},
		};

		//Remove mutabiity to prevent mistakes.
		let must_draw=must_draw;
		let job_pack_size=job_pack_size;
		let slurm_time=slurm_time;
		let slurm_mem=slurm_mem;
		let uses_jobs=uses_jobs;

		let runs_path=
		{
			let mut is_old=false;
			for experiment_index in 0..experiments.len()
			{
				let experiment_path=self.root.join(format!("run{}",experiment_index));
				if experiment_path.is_dir()
				{
					is_old=true;
					break;
				}
			}
			if is_old
			{
				self.root.join("")
			}
			else
			{
				let runs_path=self.root.join("runs");
				if !runs_path.is_dir()
				{
					fs::create_dir(&runs_path).expect("Something went wrong when creating the runs directory.");
				}
				runs_path
			}
		};

		//Execute or launch jobs.
		let start_index = self.options.start_index.unwrap_or(0);
		//if start_index<0 {panic!("start_index={} < 0",start_index);}
		if start_index>experiments.len() {panic!("start_index={} > experiments.len()={}",start_index,experiments.len());}
		let end_index = self.options.end_index.unwrap_or(experiments.len());
		//if end_index<0 {panic!("end_index={} < 0",end_index);}
		if end_index>experiments.len() {panic!("end_index={} > experiments.len()={}",end_index,experiments.len());}
		let jobs_path=runs_path.join(format!("jobs{}",self.journal_index));
		let mut launch_entry="".to_string();
		if uses_jobs && !jobs_path.is_dir()
		{
			fs::create_dir(&jobs_path).expect("Something went wrong when creating the jobs directory.");
		}
		let mut before_amount_completed=0;//We have a good local.result.
		let before_amount_slurm=self.experiments_on_slurm.len();//We can see the slurm job id in squeue. (and looking the journal file)
		let mut before_amount_inactive=0;//We have not done anything with the execution yet, i.e., no local.result.
		let mut before_amount_active=0;//We have a local.result with size 0, so we have done something. Perhaps some execution error.
		let mut delta_amount_slurm=0;
		let mut delta_completed=0;
		let sftp = self.ssh2_session.as_ref().map(|session|session.sftp().unwrap());
		let progress_bar = ProgressBar::new((end_index-start_index) as u64);
		progress_bar.set_style(ProgressStyle::default_bar().template("{prefix} [{elapsed_precise}] {bar:30.blue/white.dim} {pos:5}/{len:5} {msg}"));
		let mut pulled=0;
		let mut empty=0;
		let mut missing=0;
		match action
		{
			Action::Pull => progress_bar.set_prefix("pulling files"),
			_ => progress_bar.set_prefix("??"),
		};
		for (experiment_index,experiment) in experiments.iter().enumerate().skip(start_index).take(end_index-start_index)
		{
			progress_bar.inc(1);
			if let Some(ref expr) = self.options.where_clause
			{
				match evaluate(&expr,experiment)
				{
					ConfigurationValue::True => (),//good
					ConfigurationValue::False => continue,//discard this index
					x => panic!("The where clause evaluate to a non-bool type ({:?})",x),
				}
			}
			let experiment_path=runs_path.join(format!("run{}",experiment_index));
			if !experiment_path.is_dir()
			{
				fs::create_dir(&experiment_path).expect("Something went wrong when creating the run directory.");
			}
			let real_experiment_path=experiment_path.canonicalize().expect("This path cannot be resolved");
			let experiment_path_string = real_experiment_path.to_str().expect("You should use paths representable with unicode");
			let result_path=experiment_path.join("local.result");
			//FIXME: check if the run is expected to be currently inside some slurm job.
			let has_file = result_path.is_file();
			let has_content=if !has_file
			{
				before_amount_inactive+=1;
				false
			}
			else
			{
				result_path.metadata().unwrap().len()>=5
			};
			if !has_content
			{
				if let Some(ref external_experiment_list) = external_experiments
				{
					for (ext_index,ext_experiment) in external_experiment_list.iter().enumerate()
					{
						if experiment==ext_experiment
						{
							//println!("matching local experiment {} with external experiment {}",experiment_index,ext_index);
							let ext_path=self.options.external_source.as_ref().unwrap().join(format!("runs/run{}/local.result",ext_index));
							let mut ext_result_file=match File::open(&ext_path)
							{
								Ok(rf) => rf,
								Err(_error) =>
								{
									//panic!("There are problems opening results (external experiment {}).",ext_index);
									continue;
								}
							};
							let mut ext_result_contents=String::new();
							//remote_result_channel.read_to_string(&mut ext_result_contents);
							ext_result_file.read_to_string(&mut ext_result_contents).expect("Could not read remote result file.");
							if ext_result_contents.len()<5
							{
								//panic!("Exernal file does not have contents.");
								//empty+=1;
							}
							else
							{
								//create file
								let mut new_result_file=File::create(&result_path).expect("Could not create result file.");
								writeln!(new_result_file,"{}",ext_result_contents).unwrap();
								delta_completed+=1;
								//pulled+=1;
							}
						}
					}
				}
			}
			//if !result_path.is_file() || result_path.metadata().unwrap().len()==0
			if has_content
			{
				before_amount_completed+=1;
			}
			else
			{
				if has_file
				{
					before_amount_active+=1;
				}
				match action
				{
					Action::Local | Action::LocalAndOutput =>
					{
						println!("experiment {} of {} is {:?}",experiment_index,experiments.len(),experiment);
						let mut simulation=Simulation::new(&experiment,self.plugs);
						simulation.run();
						simulation.write_result(&mut File::create(&result_path).expect("Could not create the result file."));
					},
					Action::Slurm => if !self.experiments_on_slurm.contains(&experiment_index)
					{
						let local_cfg=experiment_path.join("local.cfg");
						let mut local_cfg_file=File::create(&local_cfg).expect("Could not create local.cfg file");
						writeln!(local_cfg_file,"{}",experiment).unwrap();
						//let job_line=format!("echo experiment {}\n/bin/date\n{} {}/local.cfg --results={}/local.result",experiment_index,self.binary.display(),experiment_path_string,experiment_path_string);
						//pending_jobs.push(job_line);
						job.add_execution(experiment_index,&self.binary,&experiment_path_string);
						if job.len()>=job_pack_size
						{
							delta_amount_slurm+=job.len();
							let job_id=experiment_index;
							launch_entry += &job.slurm(job_id,&jobs_path,slurm_time,slurm_mem);
							job=Job::new();
						}
					},
					Action::Pull =>
					{
						//println!("Could not open results of experiment {}, trying to pull it.",experiment_index);
						//println!("Trying to pull experiment {}.",experiment_index);
						//let session = self.ssh2_session.as_ref().unwrap();
						let remote_root=self.remote_root.clone().unwrap();
						let remote_result_path = remote_root.join(format!("runs/run{}/local.result",experiment_index));
						//let (mut remote_result_channel, stat) = match session.scp_recv(&remote_result_path)
						//{
						//	Ok( value ) => value,
						//	Err( _ ) =>
						//	{
						//		println!("Could not pull {}, skipping it",experiment_index);
						//		continue;
						//	},
						//};
						let mut remote_result_file = match sftp.as_ref().unwrap().open(&remote_result_path)
						{
							Ok(file) => file,
							Err(_err) =>
							{
								//println!("could not read remote file ({}).",err);
								missing+=1;
								continue;
							}
						};
						let mut remote_result_contents=String::new();
						//remote_result_channel.read_to_string(&mut remote_result_contents);
						remote_result_file.read_to_string(&mut remote_result_contents).expect("Could not read remote result file.");
						if remote_result_contents.len()<5
						{
							//println!("Remote file does not have contents.");
							empty+=1;
						}
						else
						{
							//create file
							let mut new_result_file=File::create(&result_path).expect("Could not create result file.");
							writeln!(new_result_file,"{}",remote_result_contents).unwrap();
							//drop(new_result_file);//ensure it closes and syncs
							delta_completed+=1;
							pulled+=1;
						}
						//File::open(&result_path).expect("did not work even after pulling it.")
						progress_bar.set_message(&format!("{} pulled, {} empty, {} missing",pulled,empty,missing));
					}
					Action::Output | Action::Check | Action::RemoteCheck | Action::Push | Action::SlurmCancel =>
					{
					},
				};
			}
		}
		progress_bar.finish();
		if job.len()>0
		{
			let job_id=experiments.len();
			launch_entry += &job.slurm(job_id,&jobs_path,slurm_time,slurm_mem);
			drop(job);
		}

		if ! launch_entry.is_empty()
		{
			self.write_journal_entry(&format!("Launched jobs {}",launch_entry));
		}

		let status_string = format!("Before: completed={} of {} slurm={} inactive={} active={} Changed: slurm=+{} completed=+{}",before_amount_completed,experiments.len(),before_amount_slurm,before_amount_inactive,before_amount_active,delta_amount_slurm,delta_completed);
		self.write_journal_entry(&status_string);
		println!("{}",status_string);
		
		if must_draw
		{
			results=Vec::with_capacity(experiments.len());
			//for (experiment_index,experiment) in experiments.iter().enumerate()
			for (experiment_index,experiment) in experiments.iter().enumerate().skip(start_index).take(end_index-start_index)
			{
				let experiment_path=runs_path.join(format!("run{}",experiment_index));
				let result_path=experiment_path.join("local.result");
				//let mut got_result = false;
				//let mut tried_local = false;
				//let max_tries=if let Action::Pull=action{2}else{1};
				//while !got_result
				//for itry in 0..max_tries
				//if itry==1
				//{
				//	if let Action::Pull=action
				//	{
				//	}
				//}
				//let mut result_file=File::open(&result_path).expect("result could not be opened.");
				let mut result_file=match File::open(&result_path)
				{
					Ok(rf) => rf,
					Err(_error) =>
					{
						//println!("There are problems opening results (experiment {}).",experiment_index);
						continue;
					}
				};
				let mut result_contents=String::new();
				result_file.read_to_string(&mut result_contents).expect("something went wrong reading the result file.");
				//let result=match config_parser::parse(&result_contents).unwrap_or_else(|_|panic!("could not parse the result file for experiment {}.",experiment_index))
				//{
				//	config_parser::Token::Value(value) => value,
				//	_ => panic!("wrong token"),
				//};
				//results.push((experiment.clone(),result));
				match config_parser::parse(&result_contents)
				{
					Ok(cv) =>
					{
						let result=match cv
						{
							config_parser::Token::Value(value) => value,
							_ => panic!("wrong token"),
						};
						results.push((experiment.clone(),result));
					}
					Err(_error)=>
					{
						println!("There are missing results (experiment {}).",experiment_index);
					}
				}
			}


			//println!("results={:?}",results);
			let od=self.root.join("main.od");
			let mut od_file=File::open(&od).expect("main.od could not be opened");
			let mut od_contents = String::new();
			od_file.read_to_string(&mut od_contents).expect("something went wrong reading main.od");
			match config_parser::parse(&od_contents)
			{
				Err(x) => panic!("error parsing output description file: {:?}",x),
				Ok(config_parser::Token::Value(ConfigurationValue::Array(ref descriptions))) => for description in descriptions.iter()
				{
					//println!("description={}",description);
					match create_output(&description,&results,experiments.len(),&self.root)
					{
						Ok(_) => (),
						Err(err) => println!("ERROR: could not create output {:?}",err),
					}
				},
				_ => panic!("The output description file does not contain a list.")
			};
		}
		let fin = format!("Finished action {} on {}.", action, now.format("%Y %m(%b) %0d(%a), %T (UTC%:z)").to_string());
		self.write_journal_entry(&fin);
		println!("{}",fin);
	}
	///Tries to initiate a ssh session with the remote host.
	///Will ask a pasword via keyboard.
	fn initialize_remote(&mut self)
	{
		let remote_path = self.root.join("remote");
		let mut remote_file = File::open(&remote_path).expect("remote could not be opened");
		let mut remote_contents = String::new();
		remote_file.read_to_string(&mut remote_contents).expect("something went wrong reading remote.");
		let parsed_remote=match config_parser::parse(&remote_contents)
		{
			Err(x) => panic!("error parsing remote file: {:?}",x),
			Ok(x) => x,
			//println!("parsed correctly: {:?}",x);
		};
		match parsed_remote
		{
			config_parser::Token::Value(ref value) =>
			{
				if let ConfigurationValue::Array(ref l)=value
				{
					for remote_value in l
					{
						let mut name:Option<String> = None;
						let mut host:Option<String> = None;
						let mut username:Option<String> = None;
						let mut root:Option<String> = None;
						let mut binary:Option<String> = None;
						if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=remote_value
						{
							if cv_name!="Remote"
							{
								panic!("A remote must be created from a `Remote` object not `{}`",cv_name);
							}
							for &(ref cvname,ref value) in cv_pairs
							{
								match cvname.as_ref()
								{
									"name" => match value
									{
										&ConfigurationValue::Literal(ref s) => name=Some(s.to_string()),
										_ => panic!("bad value for a remote name"),
									},
									"host" => match value
									{
										&ConfigurationValue::Literal(ref s) => host=Some(s.to_string()),
										_ => panic!("bad value for a remote host"),
									},
									"username" => match value
									{
										&ConfigurationValue::Literal(ref s) => username=Some(s.to_string()),
										_ => panic!("bad value for a remote username"),
									},
									"root" => match value
									{
										&ConfigurationValue::Literal(ref s) => root=Some(s.to_string()),
										_ => panic!("bad value for a remote root"),
									},
									"binary" => match value
									{
										&ConfigurationValue::Literal(ref s) => binary=Some(s.to_string()),
										_ => panic!("bad value for a remote binary"),
									},
									_ => panic!("Nothing to do with field {} in Remote",cvname),
								}
							}
						}
						else
						{
							panic!("Trying to create a remote from a non-Object");
						}
						if name==Some("default".to_string())
						{
							self.remote_host = host;
							self.remote_username = username;
							if let Some(value)=root
							{
								self.remote_root = Some(Path::new(&value).to_path_buf());
							}
							if let Some(value)=binary
							{
								self.remote_binary = Some(Path::new(&value).to_path_buf());
							}
						}
					}
				}
				else
				{
					panic!("there are not remotes");
				}
			},
			_ => panic!("Not a value"),
		};
		//remote values are initialized
		let host=self.remote_host.as_ref().expect("there is no host").to_owned();
		//See ssh2 documentation https://docs.rs/ssh2/0.8.2/ssh2/index.html
		let tcp = TcpStream::connect(format!("{}:22",host)).unwrap();
		let mut session = Session::new().unwrap();
		session.set_tcp_stream(tcp);
		session.handshake().unwrap();
		//See portable-pty crate /src/ssh.rs for a good example on using ssh2.
		//session.userauth_agent("cristobal").unwrap();//FIXME: this fails, as it does not get any password.
		//session.userauth_password("cristobal","").unwrap();//This also fails, without asking
		let prompt = KeyboardInteration;
		//session.userauth_keyboard_interactive("cristobal",&mut prompt).unwrap();
		let username = self.remote_username.as_ref().expect("there is no username").to_owned();
		let raw_methods = session.auth_methods(&username).unwrap();
		let methods: HashSet<&str> = raw_methods.split(',').collect();
		println!("{} available authentication methods ({})",methods.len(),raw_methods);
		//if !session.authenticated() && methods.contains("publickey")
		if !session.authenticated() && methods.contains("password")
		{
			let password=prompt.ask_password(&username,&host);
			session.userauth_password(&username,&password).expect("Password authentication failed.");
		}
		//if !session.authenticated() && methods.contains("publickey")
		assert!(session.authenticated());
		self.ssh2_session = Some(session);
		println!("ssh2 session created with remote host");
	}
}
