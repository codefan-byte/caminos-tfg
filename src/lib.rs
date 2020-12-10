/*!
caminos-lib
=====

This crate provides the CAMINOS simulator as a library. This is the Cantabrian Adaptable and Modular Interconnection Open Simulator.

# Usage

This crate is `caminos-lib`. To use it add `caminos-lib` to your dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
caminos-lib = "0.1"
```

Alternatively, consider whether the binary crate `caminos` fits your intended use.

# Public Interface

`caminos-lib` provides the functions `directory_main` and `file_main`, intended to use the file version when the final binary calls with a configuration file argument and the directory version when it is called with a directory argument.

The `directory_main` function receives a `&Path` assumed to contain a `main.cfg`, `main.od`, optionally `remote`, plus any generated files and subdirectories.
* `main.cfg` contains the definition of the experiment to perform, expected to unfold into multiple simulations.
* `main.od` contains the definition of what outputs are desired. For example `csv` files or (`pdf`,`latex`)-plots.
* `remote` allows to define a remote from which to pull result files.
* `journal`tracks the actions performed on the experiment. It is specially useful to track what execution are currently launched in what slurm jobs.
* `runs/job<action_index>/launch<experiment_index>` are the scripts launched to slurm. `action_index` is number of the current action. `experiment_index` is expected to be the experiment index of one of the experiments included in the slurm job.
* `runs/job<action_index>/launch<experiment_index>-<slurm_index>.{out,err}` are the outputs from scripts launched to slurm. The `slurm_index` is the job id given by slurm.
* `runs/run<experiment_index>/local.cfg` is the configuration exclusive to the simulation number `experiment_index`.
* `runs/run<experiment_index>/local.result` will contain the result values of the simulation number `experiment_index` after a successful simulation.

The `directory_main` receives also an `Action`. In the crate `caminos` this is done via its `--action=<method>` falg.
* `local_and_output` runs all the remaining simulations locally and generates the outputs.
* `local` runs all the simulations locally, without processing the results afterwards.
* `output` processes the currently available results and generates the outputs.
* `slurm` launches the remaining simulations onto the slurm system.
* `check` just shows how many results we got and how many are currently in slurm.
* `pull` brings result files from the defined remote host.
* `remote_check` performs a `check` action in the remote host.
* `push` compares the local main.cfg with the host remote.cfg. It reports discrepancies and create the remote path if missing.
* `slurm_cancel` executes a `scancel` with the job ids found in the journal file.


# Configuration Syntax

The configuration files are parsed using the `gramatica` crate. These files are parsed as a `ConfigurationValue` defined as following.

```
pub enum ConfigurationValue
{
	Literal(String),
	Number(f64),
	Object(String,Vec<(String,ConfigurationValue)>),
	Array(Vec<ConfigurationValue>),
	Experiments(Vec<ConfigurationValue>),
	True,
	False,
	Where(Rc<ConfigurationValue>,Expr),
	Expression(Expr),
}
```

* An `Object` os typed `Name { key1 : value1, key2 : value2, [...] }`.
* An `Array` is typed `[value1, value2, value3, [...]]`.
* An `Experiments` is typed `![value1, value2, value3, [...]]`. These are used to indicate several simulations in a experiment. This is, the set of simulations to be performed is the product of all lists of this kind.
* A `Number` can be written like 2 or 3.1. Stored as a `f64`.
* A `Literal` is a double-quoted string.
* `True` is written `true`a and `False` is written `false`.
* `Expression` is typed `=expr`, useful in output descriptions.
* The `Where` clause is not yet implemented.

## Experiment example

An example of `main.cfg` file is

```
Configuration
{
	random_seed: ![42,43,44],//Simulate each seed
	warmup: 20000,//Cycles to warm the network
	measured: 10000,//Cycles measured for the results
	topology: RandomRegularGraph//The topology is given as a named record
	{
		servers_per_router: 5,//Number of host connected to each router
		routers: 500,//Total number of routers in the network
		degree: 10,//Number of router ports reserved to go to other routers
		legend_name: "random 100-regular graph",//Name used on generated outputs
	},
	traffic: HomogeneousTraffic//Select a traffic. e.g., traffic repeating a pattern continously.
	{
		pattern: ![//We can make a simulation for each of several patterns.
			Uniform { legend_name:"uniform" },
			RandomPermutation { legend_name:"random server permutation" },
		],
		servers: 2500,//Servers involved in the traffic. Typically equal to the total of servers.
		//The load offered from the servers. A common case where to include many simulation values.
		load: ![0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 1.0],
		message_size: 16,//The size in phits of the messages created by the servers.
	}
	maximum_packet_size: 16,//Messages of greater length will be broken into several packets.
	router: Basic//The router is another object with a large description
	{
		//The number of virtual channels. The basic router sets a buffer for each virtual channel in each port, both at input and output.
		virtual_channels: 6,
		//Policies that filter the candidate routes given by the routing algorithm. They may be used to break deadlock or to give preference to some choices.
		virtual_channel_policy: [ WideHops{width:1}, LowestSinghWeight{extra_congestion:0, extra_distance:0, aggregate_buffers:true, use_internal_space:true}, Random ],
		delay: 0,//not actually implemted in the basic router. In the future it may be removed or actually implemented.
		buffer_size: 64,//phits available in each input buffer
		bubble: false,//to enable bubble mechanism in Cartesian topologies.
		flit_size: 16,//set to maximum_packet_size to have Virtual Cut-Through.
		intransit_priority: false,//whether to give preference to transit over injection.
		allow_request_busy_port: true,//whether to allow input buffer to make requests to ports that are transmitting
		output_buffer_size:32,//Available phits in each output_buffer.
	},
	routing: ![//Algorithm to provide candidate exit ports.
		Shortest { legend_name: "shortest" },
		Valiant {
			//The meta-routing by Valiant in which we sent shortest to a random middle router
			//And then shortest from the middle to the destination.
			first: Shortest,//We can change the sub-routing in either the first or second segment.
			second: Shortest,//If we do not have arguments we only put the object name. No need for braces.
			legend_name: "generic Valiant",
		},
	],
	link_classes: [
		//We can set the delays of different class of links. The number of classes depends on the topology.
		LinkClass {
			//The first class always correspond to the links between server and router
			delay:1,
		},
		//In random regular graphs all router--router links have the same class.
		LinkClass { delay: 1},
		//In a dragonfly topology we would have 0=server, 1=routers from same group, 2=routers from different groups.
	],
	launch_configuration: [
		//We may put here options to send to the SLURM system.
		Slurm
		{
			job_pack_size: 2,//number of simulations to go in each slurm job.
			time: "1-11:59:59",//maximum time allocated to each slurm job.
		},
	],
}
```

## Example output description

An example of output decription `main.od` is
```
[
	CSV//To generate a csv with a selection of fields
	{
		fields: [=configuration.traffic.pattern.legend_name, =configuration.traffic.load, =result.accepted_load, =result.average_message_delay, =configuration.routing.legend_name, =result.server_consumption_jain_index, =result.server_generation_jain_index, =result.average_packet_hops, =result.average_link_utilization, =result.maximum_link_utilization],
		filename: "results.csv",
	},
	Plots//To plot curves of data.
	{
		selector: =configuration.traffic.pattern.legend_name,//Make a plot for each value of the selector
		kind: [
			//We may create groups of figures.
			//In this example. For each value of pattern we draw three graphics.
			Plotkind{
				//The first one is accepted load for each offered load.
				//Simulations with same parameter, here offered load, are averaged together.
				parameter: =configuration.traffic.load,
				abscissas: =configuration.traffic.load,
				label_abscissas: "offered load",
				ordinates: =result.accepted_load,
				label_ordinates: "accepted load",
				min_ordinate: 0.0,
				max_ordinate: 1.0,
			},
			//In this example we draw message delay against accepted load, but we
			//continue to average by offered load. The offered load is also used for
			//the order in which points are joined by lines.
			Plotkind{
				parameter: =configuration.traffic.load,
				abscissas: =result.accepted_load,
				label_abscissas: "accepted load",
				ordinates: =result.average_message_delay,
				label_ordinates: "average message delay",
				min_ordinate: 0.0,
				max_ordinate: 200.0,
			},
		],
		legend: =configuration.routing.legend_name,
		backend: Tikz
		{
			//We use tikz to create the figures.
			//We generate a tex file easy to embed in latex document.
			//We also generate apdf file, using the latex in the system.
			tex_filename: "load_and_delay.tex",
			pdf_filename: "load_and_delay.pdf",
		},
	},
	Plots
	{
		selector: =configuration.traffic.pattern.legend_name,//Make a plot for each value of the selector
		//We can create histograms.
		kind: [Plotkind{
			label_abscissas: "path length",
			label_ordinates: "amount fo packets",
			histogram: =result.total_packet_per_hop_count,
			min_ordinate: 0.0,
			//max_ordinate: 1.0,
		}],
		legend: =configuration.routing.legend_name,
		backend: Tikz
		{
			tex_filename: "hop_histogram.tex",
			pdf_filename: "hop_histogram.pdf",
		},
	},
]
```

# Plugging

Both entries `directory_main` and `file_main` receive a `&Plugs` argument that may be used to provide the simulator with new implementations. This way, one can make a copy of the `main` in the `caminos` crate and declare plugs for their implemented `Router`, `Topology`, `Routing`, `Traffic`, `Pattern`, and `VirtualChannelPolicy`.

*/

pub use quantifiable_derive::Quantifiable;//the derive macro

pub mod config_parser;
pub mod topology;
pub mod traffic;
pub mod pattern;
pub mod router;
pub mod routing;
pub mod event;
pub mod matrix;
mod output;
pub mod quantify;
pub mod policies;
pub mod experiments;
pub mod config;

use std::rc::Rc;
use std::boxed::Box;
use std::cell::{RefCell};
use std::env;
use std::fs::{File};
use std::io::prelude::*;
use std::io::{stdout};
use std::collections::{VecDeque,BTreeMap};
use std::ops::DerefMut;
use std::path::{Path};
use std::mem::{size_of};
use std::fmt::Debug;
//use std::borrow::Cow;
use rand::{StdRng,SeedableRng};

use config_parser::{ConfigurationValue};
use topology::{Topology,new_topology,TopologyBuilderArgument,Location};
use traffic::{Traffic,new_traffic,TrafficBuilderArgument,TrafficError};
use router::{Router,new_router,RouterBuilderArgument,TransmissionFromServer,TransmissionMechanism,StatusAtEmissor};
use routing::{RoutingInfo,Routing,new_routing,RoutingBuilderArgument};
use event::{EventQueue,Event};
use quantify::Quantifiable;
use experiments::{Experiment,Action,ExperimentOptions};
use policies::{VirtualChannelPolicy,VCPolicyBuilderArgument};
use pattern::{Pattern,PatternBuilderArgument};
use config::flatten_configuration_value;

#[derive(Clone,Quantifiable)]
struct ServerStatistics
{
	created_phits: usize,
	consumed_phits: usize,
	consumed_messages: usize,
	total_message_delay: usize,
}

impl ServerStatistics
{
	fn new()->ServerStatistics
	{
		ServerStatistics{
			created_phits:0,
			consumed_phits:0,
			consumed_messages:0,
			total_message_delay:0,
		}
	}
	fn reset(&mut self)
	{
		self.created_phits=0;
		self.consumed_phits=0;
		self.consumed_messages=0;
		self.total_message_delay=0;
	}
}

///The objects that create and consume traffic to/from the network.
#[derive(Clone,Quantifiable)]
pub struct Server
{
	///The index of the server in the network.
	index: usize,
	///To which router the server is connected + link class index. Although we could just compute with the topology each time...
	port: (Location,usize),
	///Known available capacity in the connected router.
	router_status: router::StatusAtServer,
	///Created messages but not sent.
	stored_messages: VecDeque<Rc<Message>>,
	///The packets of the message that have not yet been sent.
	stored_packets: VecDeque<Rc<Packet>>,
	///The phits of a packet being sent.
	stored_phits: VecDeque<Rc<Phit>>,
	///For each message we store the number of consumed phits, until the whole message is consumed.
	consumed_phits: BTreeMap<*const Message,usize>,
	///Statistics local to the server.
	statistics: ServerStatistics,
}

impl Server
{
	///Consumes a phit
	fn consume(&mut self, phit:Rc<Phit>, traffic:&mut dyn Traffic, statistics:&mut Statistics, cycle:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.statistics.consumed_phits+=1;
		statistics.consumed_phits+=1;
		let message=phit.packet.message.clone();
		let message_ptr=message.as_ref() as *const Message;
		//println!("phit consumed at server {}: stats {:?}",self.index,statistics);
		let cp=match self.consumed_phits.get(&message_ptr)
		{
			None => 1,
			Some(x) => x+1,
		};
		if cp==message.size
		{
			//The whole message has been consumed
			self.statistics.consumed_messages+=1;
			statistics.consumed_messages+=1;
			self.statistics.total_message_delay+=cycle-message.creation_cycle;
			statistics.total_message_delay+=cycle-message.creation_cycle;
			self.consumed_phits.remove(&message_ptr);
			if !traffic.try_consume(message,cycle,topology,rng)
			{
				panic!("The traffic could not consume its own message.");
			}
			if !phit.is_end()
			{
				panic!("message was consumed by a non-ending phit.");
			}
		}
		else
		{
			self.consumed_phits.insert(message_ptr,cp);
			// Usually len==1
			//let n=self.consumed_phits.len();
			//if n>1
			//{
			//	println!("server.consumed_phits.len()={}",n);
			//}
		}
		if phit.is_end()
		{
			statistics.consumed_packets+=1;
			let hops=phit.packet.routing_info.borrow().hops;
			statistics.total_packet_hops+=hops;
			if statistics.total_packet_per_hop_count.len() <= hops
			{
				statistics.total_packet_per_hop_count.resize( hops+1, 0 );
			}
			statistics.total_packet_per_hop_count[hops]+=1;
			if cp < phit.packet.size
			{
				println!("phit tail has been consuming without haing consumed a whole packet.");
			}
		}
	}
}

//impl Quantifiable for Server
//{
//	fn total_memory(&self) -> usize
//	{
//		return size_of::<Server>() + self.stored_messages.total_memory() + self.stored_packets.total_memory() + self.stored_phits.total_memory() + self.consumed_phits.total_memory();
//	}
//	fn print_memory_breakdown(&self)
//	{
//		unimplemented!();
//	}
//	fn forecast_total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//}


///An instantiated network, with all its routers and servers.
pub struct Network
{
	///The topology defining the conectivity.
	pub topology: Box<dyn Topology>,
	//XXX The only reason to use Rc instead of Box is to make them insertable on the event queue. Perhaps the Eventful should be Box<MyRouter> instead of directly MyRouter? Or maybe storing some other kind of reference to the RefCell or the Box?
	///TThe collection of all the routers in the network.
	pub routers: Vec<Rc<RefCell<dyn Router>>>,
	//routers: Vec<Box<RefCell<dyn Router>>>,
	///TThe collection of all the servers in the network.
	pub servers: Vec<Server>,
}

impl Quantifiable for Network
{
	fn total_memory(&self) -> usize
	{
		let mut total=size_of::<Box<dyn Topology>>() + self.topology.total_memory() + self.routers.total_memory() + self.servers.total_memory();
		//let mut phit_count=0;
		for router in self.routers.iter()
		{
			total+=router.as_ref().total_memory();
			let rb=router.borrow();
			for phit in rb.iter_phits()
			{
				total+=phit.as_ref().total_memory();
				//phit_count+=1;
				if phit.is_end()
				{
					let packet=phit.packet.as_ref();
					total+=packet.total_memory();
				}
			}
		}
		for server in self.servers.iter()
		{
			for phit in server.stored_phits.iter()
			{
				total+=phit.as_ref().total_memory();
			}
			for packet in server.stored_packets.iter()
			{
				total+=packet.as_ref().total_memory();
			}
			for message in server.stored_messages.iter()
			{
				total+=message.as_ref().total_memory();
			}
			for (_message_ptr,_) in server.consumed_phits.iter()
			{
				total+=size_of::<Message>();
			}
		}
		//println!("phit_count={}",phit_count);
		total
	}
	fn print_memory_breakdown(&self)
	{
		unimplemented!();
	}
	fn forecast_total_memory(&self) -> usize
	{
		unimplemented!();
	}
}

///Minimal unit to be processed by the network.
///Not to be confused with flits.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct Phit
{
	///The packet to what this phit belongs
	pub packet: Rc<Packet>,
	///position inside the packet
	pub index: usize,
	///The virtual channel in which this phit should be inserted
	pub virtual_channel: RefCell<Option<usize>>,
}

///A portion of a message. They are divided into phits.
///All phits must go through the same queues without phits of other packets in between.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct Packet
{
	///Number of phits
	pub size: usize,
	///Information for the routing
	pub routing_info: RefCell<RoutingInfo>,
	///The message to what this packet belongs.
	pub message: Rc<Message>,
	///position inside the message
	pub index: usize,
}

///An application message, broken into packets
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct Message
{
	///Server that created the message.
	pub origin: usize,
	///Server that is the destination of the message.
	pub destination: usize,
	///Number of phits.
	pub size: usize,
	///Cycle when the message was created.
	pub creation_cycle: usize,
}

impl Phit
{
	///Whether the phit is leading a packet. Routers check this to make requests, stablish flows, etc.
	pub fn is_begin(&self) -> bool
	{
		self.index==0
	}
	///Whether this phit is the last one of a packet. Routers use this to finalize some operations.
	pub fn is_end(&self) -> bool
	{
		self.index==self.packet.size-1
	}
}

///Description of common properties of sets of links.
///For example, the links to servers could have a different delay.
///The topologies can set additional classes. For example, a mesh/torus can diffentiate horizontal/vertical links.
///And a dragonfly topology can differentiate local from global links.
pub struct LinkClass
{
	///Cycles the phit needs to move from one endpoint to the other endpoint.
	pub delay: usize,
	//(x,y) means x phits each y cycles ??
	//transference_speed: (usize,usize)
}

impl LinkClass
{
	fn new(cv:&ConfigurationValue) -> LinkClass
	{
		let mut delay=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=cv
		{
			if cv_name!="LinkClass"
			{
				panic!("A LinkClass must be created from a `LinkClass` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match name.as_ref()
				{
					"delay" => match value
					{
						&ConfigurationValue::Number(f) => delay=Some(f as usize),
						_ => panic!("bad value for delay"),
					},
					"transference_speed" => (),//FIXME
					_ => panic!("Nothing to do with field {} in LinkClass",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a LinkClass from a non-Object");
		}
		let delay=delay.expect("There were no delay");
		LinkClass{
			delay,
		}
	}
}

///Statistics captured for each link.
#[derive(Debug)]
struct LinkStatistics
{
	phit_arrivals: usize,
}

impl LinkStatistics
{
	fn new() -> LinkStatistics
	{
		LinkStatistics{
			phit_arrivals: 0,
		}
	}
	fn reset(&mut self)
	{
		self.phit_arrivals=0;
	}
}

///All the global statistics captured.
#[derive(Debug)]
pub struct Statistics
{
	///The number of the first cycle included in the statistics.
	begin_cycle: usize,
	///The number of phits that servers have sent to routers.
	created_phits: usize,
	///Number of phits that have reached their destination server (called consume).
	consumed_phits: usize,
	///Number of phit tails consumed.
	consumed_packets: usize,
	///Number of messages for which all their phits have beeen consumed.
	consumed_messages: usize,
	///Accumulated delay of al messages. From message creation (in traffic.rs) to server consumption.
	total_message_delay: usize,
	///Accumulated count of hops made for all consumed packets.
	total_packet_hops: usize,
	///Count of consumed packets indexed by the number of hops it made.
	total_packet_per_hop_count: Vec<usize>,
	///Specific statistics of the links. Indexed by router and port.
	link_statistics: Vec<Vec<LinkStatistics>>,
	///The columns to print in the periodic reports.
	columns: Vec<ReportColumn>,
}

impl Statistics
{
	fn new(topology: &dyn Topology)->Statistics
	{
		Statistics{
			begin_cycle:0,
			created_phits:0,
			consumed_phits:0,
			consumed_packets:0,
			consumed_messages:0,
			total_message_delay:0,
			total_packet_hops:0,
			total_packet_per_hop_count:Vec::new(),
			link_statistics: (0..topology.num_routers()).map(|i| (0..topology.ports(i)).map(|_|LinkStatistics::new()).collect() ).collect(),
			columns: vec![
				ReportColumnKind::BeginEndCycle.into(),
				ReportColumnKind::InjectedLoad.into(),
				ReportColumnKind::AcceptedLoad.into(),
				ReportColumnKind::AveragePacketHops.into(),
				ReportColumnKind::AverageLinkUtilization.into(),
				//ReportColumnKind::MaximumLinkUtilization.into(),
				ReportColumnKind::AverageMessageDelay.into(),
				//ReportColumnKind::ServerGenerationJainIndex.into(),
				ReportColumnKind::ServerConsumptionJainIndex.into(),
				],
		}
	}
	fn jain_server_created_phits(&self, network:&Network) -> f64
	{
		//double rcvd_count_total=0.0;
		//double rcvd_count2_total=0.0;
		let mut count=0.0;
		let mut count2=0.0;
		for server in network.servers.iter()
		{
			//double x=(double)(network[i].rcvd_count_from);
			let x=server.statistics.created_phits as f64;
			count+=x;
			count2+=x*x;
		}
		//double Jain_fairness=rcvd_count_total*rcvd_count_total/rcvd_count2_total/(double)nprocs;
		//printf("OUT_Jain_fairness=%f%s",Jain_fairness,sep);
		count*count/count2/network.servers.len() as f64
	}
	fn jain_server_consumed_phits(&self, network:&Network) -> f64
	{
		//double rcvd_count_total=0.0;
		//double rcvd_count2_total=0.0;
		let mut count=0.0;
		let mut count2=0.0;
		for server in network.servers.iter()
		{
			//double x=(double)(network[i].rcvd_count_from);
			let x=server.statistics.consumed_phits as f64;
			count+=x;
			count2+=x*x;
		}
		//double Jain_fairness=rcvd_count_total*rcvd_count_total/rcvd_count2_total/(double)nprocs;
		//printf("OUT_Jain_fairness=%f%s",Jain_fairness,sep);
		count*count/count2/network.servers.len() as f64
	}
	///Print in stdout a header showing the statistical columns to be periodically printed.
	fn print_header(&self)
	{
		//println!("cycle_begin-cycle_end injected_load accepted_load server_generation_jain_index server_consumption_jain_index");
		let report:String = self.columns.iter().map(|c|c.header()).collect();
		println!("{}",report);
	}
	///Print in stdout the current values of the statistical columns indicated to be periodically printed.
	fn print(&self, next_cycle:usize, network:&Network)
	{
		//let cycles=next_cycle-self.begin_cycle+1;
		//let injected_load=self.created_phits as f32/cycles as f32/network.servers.len() as f32;
		//let accepted_load=self.consumed_phits as f32/cycles as f32/network.servers.len() as f32;
		//let jsgp=self.jain_server_created_phits(network);
		//let jscp=self.jain_server_consumed_phits(network);
		//println!("{:>11}-{:<9} {:<13} {:<13} {:<17} {:<12}",self.begin_cycle,next_cycle-1,injected_load,accepted_load,jsgp,jscp);
		let report:String = self.columns.iter().map(|c|c.format(self,next_cycle,network)).collect();
		println!("{}",report);
	}
	///Forgets all captured statistics and began capturing again.
	fn reset(&mut self,next_cycle:usize, network:&mut Network)
	{
		self.begin_cycle=next_cycle;
		self.created_phits=0;
		self.consumed_phits=0;
		self.consumed_packets=0;
		self.consumed_messages=0;
		self.total_message_delay=0;
		self.total_packet_hops=0;
		self.total_packet_per_hop_count=Vec::new();
		for server in network.servers.iter_mut()
		{
			server.statistics.reset();
		}
		for router in network.routers.iter()
		{
			router.borrow_mut().reset_statistics(next_cycle);
		}
		for router_links in self.link_statistics.iter_mut()
		{
			for link in router_links.iter_mut()
			{
				link.reset();
			}
		}
	}
}

///The available statistical columns. Each column has a string for the header and a way to compute what to print each period.
#[derive(Debug)]
#[allow(dead_code)]
enum ReportColumnKind
{
	BeginEndCycle,
	InjectedLoad,
	AcceptedLoad,
	ServerGenerationJainIndex,
	ServerConsumptionJainIndex,
	AverageMessageDelay,
	AveragePacketHops,
	AverageLinkUtilization,
	MaximumLinkUtilization,
}

impl ReportColumnKind
{
	fn name(&self) -> &str
	{
		match self
		{
			ReportColumnKind::BeginEndCycle => "cycle_begin-cycle_end",
			ReportColumnKind::InjectedLoad => "injected_load",
			ReportColumnKind::AcceptedLoad => "accepted_load",
			ReportColumnKind::ServerGenerationJainIndex => "server_generation_jain_index",
			ReportColumnKind::ServerConsumptionJainIndex => "server_consumption_jain_index",
			ReportColumnKind::AverageMessageDelay => "average_message_delay",
			ReportColumnKind::AveragePacketHops => "average_packet_hops",
			ReportColumnKind::AverageLinkUtilization => "average_link_utilization",
			ReportColumnKind::MaximumLinkUtilization => "maximum_link_utilization",
		}
	}
}

///A statistical column with extra formatting information.
#[derive(Debug)]
struct ReportColumn
{
	kind: ReportColumnKind,
	width: usize,
}

impl ReportColumn
{
	fn header(&self) -> String
	{
		//let base = match self.kind
		//{
		//	ReportColumnKind::BeginEndCycle => "cycle_begin-cycle_end",
		//	ReportColumnKind::InjectedLoad => "injected_load",
		//	ReportColumnKind::AcceptedLoad => "accepted_load",
		//	ReportColumnKind::ServerGenerationJainIndex => "server_generation_jain_index",
		//	ReportColumnKind::ServerConsumptionJainIndex => "server_consumption_jain_index",
		//};
		let base = self.kind.name();
		format!("{name:width$}",name=base,width=self.width)
	}
	fn format(&self, statistics: &Statistics, next_cycle:usize, network:&Network) -> String
	{
		let cycles=next_cycle-statistics.begin_cycle+1;
		let value = match self.kind
		{
			ReportColumnKind::BeginEndCycle => format!("{:>11}-{}",statistics.begin_cycle,next_cycle-1),
			ReportColumnKind::InjectedLoad => format!{"{}",statistics.created_phits as f32/cycles as f32/network.servers.len() as f32},
			ReportColumnKind::AcceptedLoad =>  format!{"{}",statistics.consumed_phits as f32/cycles as f32/network.servers.len() as f32},
			ReportColumnKind::ServerGenerationJainIndex => format!{"{}",statistics.jain_server_created_phits(network)},
			ReportColumnKind::ServerConsumptionJainIndex => format!{"{}",statistics.jain_server_consumed_phits(network)},
			ReportColumnKind::AverageMessageDelay => format!("{}",statistics.total_message_delay as f64/statistics.consumed_messages as f64),
			ReportColumnKind::AveragePacketHops => format!("{}",statistics.total_packet_hops as f64 / statistics.consumed_packets as f64),
			ReportColumnKind::AverageLinkUtilization =>
			{
				let total_arrivals:usize = (0..network.topology.num_routers()).map(|i|(0..network.topology.degree(i)).map(|j|statistics.link_statistics[i][j].phit_arrivals).sum::<usize>()).sum();
				let total_links: usize = (0..network.topology.num_routers()).map(|i|network.topology.degree(i)).sum();
				format!("{}",total_arrivals as f64 / cycles as f64 / total_links as f64)
			},
			ReportColumnKind::MaximumLinkUtilization =>
			{
				let maximum_arrivals:usize = statistics.link_statistics.iter().map(|rls|rls.iter().map(|ls|ls.phit_arrivals).max().unwrap()).max().unwrap();
				format!("{}",maximum_arrivals as f64 / cycles as f64)
			},
		};
		format!("{value:width$}",value=value,width=self.width)
	}
}

///From putting default values for each kind.
impl From<ReportColumnKind> for ReportColumn
{
	fn from(kind:ReportColumnKind) -> ReportColumn
	{
		let width = 1+kind.name().len();
		ReportColumn{
			kind,
			width,
		}
	}
}

///The object represeting the whole simulation.
pub struct Simulation<'a>
{
	///The whole parsed configuration.
	#[allow(dead_code)]
	pub configuration: ConfigurationValue,
	///The seed of the random number generator.
	#[allow(dead_code)]
	pub seed: usize,
	///The random number generator itself, with its current state.
	pub rng: RefCell<StdRng>,
	///Cycles of preparation before the actual measured execution
	pub warmup: usize,
	///Cycles of measurement
	pub measured: usize,
	///The instantiated network. It constains the routers and servers connected according to the topology.
	pub network: Network,
	///The traffic being generated/consumed by the servers.
	pub traffic: Box<dyn Traffic>,
	///The maximum size in phits that network packets can have. Any message greater than this is broken into several packets.
	pub maximum_packet_size: usize,
	///The routing algorithm that the network router will employ to set candidate routes.
	pub routing: Box<dyn Routing>,
	///The properties associated to each link class.
	pub link_classes: Vec<LinkClass>,
	///The queue of events guiding the simulation.
	pub event_queue: EventQueue,
	///The current cycle, i.e, the current discrete time.
	pub cycle:usize,
	///The statistics being collected.
	pub statistics: Statistics,
	///Information abut how to launch simulations to different systems.
	#[allow(dead_code)]
	pub launch_configurations: Vec<ConfigurationValue>,
	///Plugged functions to build traffics, routers, etc.
	pub plugs: &'a Plugs,
}

impl<'a> Simulation<'a>
{
	fn new(cv: &ConfigurationValue, plugs:&'a Plugs) -> Simulation<'a>
	{
		let mut seed: Option<usize> = None;
		let mut topology =None;
		let mut traffic =None;
		let mut router_cfg: Option<&ConfigurationValue> =None;
		let mut warmup = None;
		let mut measured = None;
		let mut maximum_packet_size=None;
		let mut routing=None;
		let mut link_classes = None;
		let mut launch_configurations: Vec<ConfigurationValue> = vec![];
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=cv
		{
			if cv_name!="Configuration"
			{
				panic!("A simulation must be created from a `Configuration` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match name.as_ref()
				{
					"random_seed" => match value
					{
						&ConfigurationValue::Number(f) => seed=Some(f as usize),
						_ => panic!("bad value for random_seed"),
					}
					"warmup" => match value
					{
						&ConfigurationValue::Number(f) => warmup=Some(f as usize),
						_ => panic!("bad value for warmup"),
					}
					"measured" => match value
					{
						&ConfigurationValue::Number(f) => measured=Some(f as usize),
						_ => panic!("bad value for measured"),
					}
					//"topology" => topology=Some(new_topology(value)),
					"topology" => topology=Some(value),
					"traffic" =>
					{
						//traffic=Some(new_traffic(value,self.rng));
						traffic=Some(value);
					},
					"maximum_packet_size" => match value
					{
						&ConfigurationValue::Number(f) => maximum_packet_size=Some(f as usize),
						_ => panic!("bad value for maximum_packet_size"),
					}
					"router" => router_cfg=Some(&value),
					"routing" => routing=Some(new_routing(RoutingBuilderArgument{cv:value,plugs})),
					"link_classes" => match value
					{
						&ConfigurationValue::Array(ref l) => link_classes=Some(l.iter().map(|v|LinkClass::new(v)).collect()),
						_ => panic!("bad value for link_classes"),
					}
					"launch_configurations" => match value
					{
						&ConfigurationValue::Array(ref l) => launch_configurations=l.clone(),
						_ => panic!("bad value for launch_configurations"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Configuration",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a simulation from a non-Object");
		}
		let seed=seed.expect("There were no random_seed");
		let warmup=warmup.expect("There were no warmup");
		let measured=measured.expect("There were no measured");
		let topology=topology.expect("There were no topology");
		let traffic=traffic.expect("There were no traffic");
		let maximum_packet_size=maximum_packet_size.expect("There were no maximum_packet_size");
		let router_cfg=router_cfg.expect("There were no router");
		let mut routing=routing.expect("There were no routing");
		let link_classes:Vec<LinkClass>=link_classes.expect("There were no link_classes");
		let rng=RefCell::new(StdRng::from_seed(&[seed]));
		let topology=new_topology(TopologyBuilderArgument{
			cv:topology,
			plugs,
			rng:&rng,
		});
		topology.check_adjacency_consistency(Some(link_classes.len()));
		routing.initialize(&topology,&rng);
		let num_routers=topology.num_routers();
		let num_servers=topology.num_servers();
		//let routers: Vec<Rc<RefCell<dyn Router>>>=(0..num_routers).map(|index|new_router(index,router_cfg,plugs,topology.as_ref(),maximum_packet_size)).collect();
		let routers: Vec<Rc<RefCell<dyn Router>>>=(0..num_routers).map(|index|new_router(router::RouterBuilderArgument{
			router_index:index,
			cv:router_cfg,
			plugs,
			topology:topology.as_ref(),
			maximum_packet_size
		})).collect();
		let servers=(0..num_servers).map(|index|{
			let port=topology.server_neighbour(index);
			let router_status=match port.0
			{
				Location::RouterPort{
					router_index,
					router_port
				} => {
					let router=routers[router_index].borrow();
					let nvc=router.num_virtual_channels();
					let buffer_amount=nvc;
					let buffer_size=(0..nvc).map(|vc|router.virtual_port_size(router_port,vc)).sum::<usize>();
					let size_to_send=maximum_packet_size;
					let from_server_mechanism = TransmissionFromServer::new(buffer_amount,buffer_size,size_to_send);
					from_server_mechanism.new_status_at_emissor()
				}
				_ => panic!("Server is not connected to router"),
			};
			Server{
				index,
				port,
				router_status,
				stored_messages:VecDeque::new(),
				stored_packets:VecDeque::new(),
				stored_phits:VecDeque::new(),
				consumed_phits: BTreeMap::new(),
				statistics: ServerStatistics::new(),
			}
		}).collect();
		let traffic=new_traffic(TrafficBuilderArgument{
			cv:traffic,
			plugs,
			topology:&topology,
			rng:&rng,
		});
		let statistics=Statistics::new(topology.as_ref());
		Simulation{
			configuration: cv.clone(),
			seed,
			rng,
			warmup,
			measured,
			network: Network{
				topology,
				routers,
				servers,
			},
			traffic,
			maximum_packet_size,
			routing,
			link_classes,
			event_queue: EventQueue::new(1000),
			cycle:0,
			statistics,
			launch_configurations,
			plugs,
		}
	}
	///Run the simulations until it finishes.
	fn run(&mut self)
	{
		self.print_memory_breakdown();
		self.statistics.print_header();
		while self.cycle < self.warmup+self.measured
		{
			self.advance();
			if self.cycle==self.warmup
			{
				self.statistics.reset(self.cycle,&mut self.network);
				self.routing.reset_statistics(self.cycle);
			}
			if self.traffic.is_finished()
			{
				println!("Traffic consumed before cycle {}",self.cycle);
				break;
			}
		}
	}
	///Execute a single cycle of the simulation.
	fn advance(&mut self)
	{
		let mut ievent=0;
		//println!("Begin advance");
		//while let Some(event) = self.event_queue.access_begin(ievent)
		loop
		{
			let event=if let Some(event) = self.event_queue.access_begin(ievent)
			{
				event.clone()
			}
			else
			{
				break;
			};
			//if self.cycle>=3122
			//{
			//	println!("Processing begin event at position {}",ievent);
			//}
			match event
			{
				Event::PhitToLocation{
					ref phit,
					ref previous,
					//router,
					//port,
					ref new,
				} =>
				{
					match new
					{
						&Location::RouterPort{router_index:router,router_port:port} =>
						{
							self.statistics.link_statistics[router][port].phit_arrivals+=1;
							let mut brouter=self.network.routers[router].borrow_mut();
							brouter.insert(phit.clone(),port,&self.rng);
							if brouter.pending_events()==0
							{
								brouter.add_pending_event();
								//self.event_queue.enqueue_end(Event::Generic(self.network.routers[router]),0);
								//self.event_queue.enqueue_end(Event::Generic(self.network.routers[router] as Rc<RefCell<Eventful>>),0);
								self.event_queue.enqueue_end(Event::Generic(brouter.as_eventful().upgrade().expect("missing router")),0);
							}
							match previous
							{
								&Location::ServerPort(_server_index) => if phit.is_begin()
								{
									self.routing.initialize_routing_info(&phit.packet.routing_info, self.network.topology.as_ref(), router, phit.packet.message.destination,&self.rng);
								},
								&Location::RouterPort{../*router_index,router_port*/} => if phit.is_begin()
								{
									phit.packet.routing_info.borrow_mut().hops+=1;
									self.routing.update_routing_info(&phit.packet.routing_info, self.network.topology.as_ref(), router, port, phit.packet.message.destination,&self.rng);
								},
								_ => (),
							};
						},
						&Location::ServerPort(server) =>
						{
							if server!=phit.packet.message.destination
							{
								panic!("Packet reached wrong server, {} instead of {}!\n",server,phit.packet.message.destination);
							}
							self.network.servers[server].consume(phit.clone(),self.traffic.deref_mut(),&mut self.statistics,self.cycle,&self.network.topology,&self.rng);
						}
						&Location::None => panic!("Phit went nowhere previous={:?}",previous),
					};
				},
				//Event::PhitClearAcknowledge
				Event::Acknowledge{
					location,
					//virtual_channel,
					message: ack_message,
				} => match location
				{
					Location::RouterPort{
						router_index,
						router_port,
					} =>
					{
						let mut brouter=self.network.routers[router_index].borrow_mut();
						//brouter.acknowledge(router_port,virtual_channel);
						brouter.acknowledge(router_port,ack_message);
						if brouter.pending_events()==0
						{
							brouter.add_pending_event();
							self.event_queue.enqueue_end(Event::Generic(brouter.as_eventful().upgrade().expect("missing router")),0);
						}
					},
					Location::ServerPort(server) => self.network.servers[server].router_status.acknowledge(ack_message),
					//&Location::ServerPort(server) => TransmissionFromServer::acknowledge(self.network.servers[server].router_status,ack_message),
					_ => (),
				},
				Event::Generic(ref _element) => unimplemented!(),
			};
			ievent+=1;
		}
		//println!("Done cycle-begin events");
		ievent=0;
		//while let Some(event) = self.event_queue.access_end(ievent)
		loop
		{
			let event=if let Some(event) = self.event_queue.access_end(ievent)
			{
				event.clone()
			}
			else
			{
				break;
			};
			//if self.cycle>=3122
			//{
			//	println!("Processing end event at position {}",ievent);
			//}
			match event
			{
				Event::PhitToLocation{
					..
					//ref phit,
					//ref previous,
					//ref new,
				} => panic!("Phits should not arrive at the end of a cycle"),
				//Event::PhitClearAcknowledge
				Event::Acknowledge{
					..
					//ref location,
					//virtual_channel,
				} => panic!("Phit Acknowledgements should not arrive at the end of a cycle"),
				Event::Generic(ref element) =>
				{
					let new_events=element.borrow_mut().process(self);
					//element.borrow_mut().clear_pending_events();//now done by process itself
					for ge in new_events.into_iter()
					{
						self.event_queue.enqueue(ge);
					}
				},
			};
			ievent+=1;
		}
		//println!("Done cycle-end events");
		let num_servers=self.network.servers.len();
		for (iserver,server) in self.network.servers.iter_mut().enumerate()
		{
			//println!("credits of {} = {}",iserver,server.credits);
			if let (Location::RouterPort{router_index: index,router_port: port},link_class)=server.port
			{
				//FIXME: magic value
				if server.stored_messages.len()<20 && self.traffic.should_generate(iserver,&self.rng)
				{
					//server.stored_messages.push_back(Rc::new(self.traffic.generate_message(iserver,&self.rng)));
					//if let Some(message) = self.traffic.generate_message(iserver,&self.rng)
					//{
					//	server.stored_messages.push_back(Rc::new(message));
					//}
					match self.traffic.generate_message(iserver,self.cycle,&self.network.topology,&self.rng)
					{
						Ok(message) =>
						{
							if message.destination>=num_servers
							{
								panic!("Message sent to outside the network unexpectedly.");
							}
							if message.destination==iserver
							{
								panic!("Generated message to self unexpectedly.");
							}
							server.stored_messages.push_back(message);
						},
						Err(TrafficError::OriginOutsideTraffic) => (),
						Err(TrafficError::SelfMessage) => (),
						//Err(error) => panic!("An error happened when generating traffic: {:?}",error),
					};
				}
				if server.stored_packets.len()==0 && server.stored_messages.len()>0
				{
					let message=server.stored_messages.pop_front().expect("There are not messages in queue");
					let mut size=message.size;
					while size>0
					{
						let ps=if size>self.maximum_packet_size
						{
							self.maximum_packet_size
						}
						else
						{
							size
						};
						server.stored_packets.push_back(Rc::new(Packet{
							size:ps,
							routing_info: RefCell::new(RoutingInfo::new()),
							message:message.clone(),
							index:0,
						}));
						size-=ps;
					}
				}
				if server.stored_phits.len()==0 && server.stored_packets.len()>0
				{
					let packet=server.stored_packets.pop_front().expect("There are not packets in queue");
					for index in 0..packet.size
					{
						server.stored_phits.push_back(Rc::new(Phit{
							packet:packet.clone(),
							index,
							virtual_channel: RefCell::new(None),
						}));
					}
				}
				//if server.stored_phits.len()>0 && server.credits>0
				//{
				//	let phit=server.stored_phits.pop_front().expect("There are not phits");
				//	let event=Event::PhitToLocation{
				//		phit,
				//		previous: Location::ServerPort(iserver),
				//		new: Location::RouterPort{router_index:index,router_port:port},
				//	};
				//	self.statistics.created_phits+=1;
				//	server.statistics.created_phits+=1;
				//	self.event_queue.enqueue_begin(event,self.link_classes[link_class].delay);
				//	server.credits-=1;
				//}
				if server.stored_phits.len()>0
				{
					//Do not extract the phit until we know whether we can transmit it.
					let phit=server.stored_phits.front().expect("There are not phits");
					if server.router_status.can_transmit(&phit,0)
					{
						let phit=server.stored_phits.pop_front().expect("There are not phits");
						let event=Event::PhitToLocation{
							phit,
							previous: Location::ServerPort(iserver),
							new: Location::RouterPort{router_index:index,router_port:port},
						};
						self.statistics.created_phits+=1;
						server.statistics.created_phits+=1;
						self.event_queue.enqueue_begin(event,self.link_classes[link_class].delay);
						server.router_status.notify_outcoming_phit(0,self.cycle);
					}
				}
			}
			else
			{
				panic!("Where goes this port?");
			}
		}
		//println!("Done generation");
		//if self.cycle%1000==999
		//{
		//	self.print_memory_breakdown();
		//}
		self.event_queue.advance();
		self.cycle+=1;
		if self.cycle%1000==0
		{
			//println!("Statistics up to cycle {}: {:?}",self.cycle,self.statistics);
			self.statistics.print(self.cycle,&self.network);
			//self.print_memory_breakdown();
		}
	}
	///Write the result of the simulation somewhere, typically to a 'result' file in a 'run*' directory.
	fn write_result(&self,output:&mut dyn Write)
	{
		// https://stackoverflow.com/questions/22355273/writing-to-a-file-or-stdout-in-rust
		//output.write(b"Hello from the simulator\n").unwrap();
		//Result
		//{
		//	accepted_load: 0.9,
		//	average_message_delay: 100,
		//}
		let cycles=self.cycle-self.statistics.begin_cycle;
		let num_servers=self.network.servers.len();
		let injected_load=self.statistics.created_phits as f64/cycles as f64/num_servers as f64;
		let accepted_load=self.statistics.consumed_phits as f64/cycles as f64/num_servers as f64;
		let average_message_delay=self.statistics.total_message_delay as f64/self.statistics.consumed_messages as f64;
		let jscp=self.statistics.jain_server_consumed_phits(&self.network);
		let jsgp=self.statistics.jain_server_created_phits(&self.network);
		let average_packet_hops=self.statistics.total_packet_hops as f64 / self.statistics.consumed_packets as f64;
		let total_packet_per_hop_count=self.statistics.total_packet_per_hop_count.iter().map(|&count|ConfigurationValue::Number(count as f64)).collect();
		//let total_arrivals:usize = self.statistics.link_statistics.iter().map(|rls|rls.iter().map(|ls|ls.phit_arrivals).sum::<usize>()).sum();
		//let total_links:usize = self.statistics.link_statistics.iter().map(|rls|rls.len()).sum();
		let total_arrivals:usize = (0..self.network.topology.num_routers()).map(|i|(0..self.network.topology.degree(i)).map(|j|self.statistics.link_statistics[i][j].phit_arrivals).sum::<usize>()).sum();
		let total_links: usize = (0..self.network.topology.num_routers()).map(|i|self.network.topology.degree(i)).sum();
		let average_link_utilization = total_arrivals as f64 / cycles as f64 / total_links as f64;
		let maximum_arrivals:usize = self.statistics.link_statistics.iter().map(|rls|rls.iter().map(|ls|ls.phit_arrivals).max().unwrap()).max().unwrap();
		let maximum_link_utilization = maximum_arrivals as f64 / cycles as f64;
		let git_id=get_git_id();
		let mut result_content = vec![
			(String::from("injected_load"),ConfigurationValue::Number(injected_load)),
			(String::from("accepted_load"),ConfigurationValue::Number(accepted_load)),
			(String::from("average_message_delay"),ConfigurationValue::Number(average_message_delay)),
			(String::from("server_generation_jain_index"),ConfigurationValue::Number(jsgp)),
			(String::from("server_consumption_jain_index"),ConfigurationValue::Number(jscp)),
			(String::from("average_packet_hops"),ConfigurationValue::Number(average_packet_hops)),
			(String::from("total_packet_per_hop_count"),ConfigurationValue::Array(total_packet_per_hop_count)),
			(String::from("average_link_utilization"),ConfigurationValue::Number(average_link_utilization)),
			(String::from("maximum_link_utilization"),ConfigurationValue::Number(maximum_link_utilization)),
			//(String::from("git_id"),ConfigurationValue::Literal(format!("\"{}\"",git_id))),
			(String::from("git_id"),ConfigurationValue::Literal(format!("{}",git_id))),
		];
		if let Some(content)=self.routing.statistics(self.cycle)
		{
			result_content.push((String::from("routing_statistics"),content));
		}
		if let Some(content) = self.network.routers.iter().enumerate().fold(None,|maybe_stat,(index,router)|router.borrow().aggregate_statistics(maybe_stat,index,self.network.routers.len(),self.cycle))
		{
			result_content.push((String::from("router_aggregated_statistics"),content));
		}
		let result=ConfigurationValue::Object(String::from("Result"),result_content);
		writeln!(output,"{}",result).unwrap();
	}
}

impl<'a> Quantifiable for Simulation<'a>
{
	fn total_memory(&self) -> usize
	{
		unimplemented!();
	}
	fn print_memory_breakdown(&self)
	{
		println!("\nBegin memory report");
		println!("self : {}",size_of::<Self>());
		//println!("phits on statistics : {}",self.statistics.created_phits-self.statistics.consumed_phits);
		println!("phit : {}",size_of::<Phit>());
		println!("packet : {}",size_of::<Packet>());
		println!("message : {}",size_of::<Message>());
		//println!("topology : {}",size_of::<dyn Topology>());
		//println!("router : {}",size_of::<dyn Router>());
		println!("server : {}",size_of::<Server>());
		println!("event : {}",size_of::<Event>());
		//self.event_queue.print_memory();
		println!("network total : {}",quantify::human_bytes(self.network.total_memory()));
		println!("traffic total : {}",quantify::human_bytes(self.traffic.total_memory()));
		println!("event_queue total : {}",quantify::human_bytes(self.event_queue.total_memory()));
		//println!("topology total : {}",quantify::human_bytes(self.network.topology.total_memory()));
		println!("End memory report\n");
	}
	fn forecast_total_memory(&self) -> usize
	{
		unimplemented!();
	}
}


#[derive(Default)]
pub struct Plugs
{
	//routers: BTreeMap<String, fn(usize,&ConfigurationValue,&Plugs, &dyn Topology, usize) -> Rc<RefCell<dyn Router>>  >,
	routers: BTreeMap<String, fn(RouterBuilderArgument) -> Rc<RefCell<dyn Router>>  >,
	//topologies: BTreeMap<String, fn(&ConfigurationValue, &Plugs, &RefCell<StdRng>) -> Box<dyn Topology> >,
	topologies: BTreeMap<String, fn(TopologyBuilderArgument) -> Box<dyn Topology> >,
	//routings: BTreeMap<String,fn(&ConfigurationValue, &Plugs) -> Box<dyn Routing>>,
	routings: BTreeMap<String,fn(RoutingBuilderArgument) -> Box<dyn Routing>>,
	//traffics: BTreeMap<String,fn(&ConfigurationValue, &Plugs, &Box<dyn Topology>, &RefCell<StdRng>) -> Box<dyn Traffic> >,
	traffics: BTreeMap<String,fn(TrafficBuilderArgument) -> Box<dyn Traffic> >,
	patterns: BTreeMap<String, fn(PatternBuilderArgument) -> Box<dyn Pattern> >,
	policies: BTreeMap<String, fn(VCPolicyBuilderArgument) -> Box<dyn VirtualChannelPolicy> >,
}

impl Plugs
{
	//pub fn add_router(&mut self, key:String, builder:fn(usize,&ConfigurationValue,&Plugs, &dyn Topology, usize) -> Rc<RefCell<dyn Router>>)
	pub fn add_router(&mut self, key:String, builder:fn(RouterBuilderArgument) -> Rc<RefCell<dyn Router>>)
	{
		self.routers.insert(key,builder);
	}
	pub fn add_topology(&mut self, key:String, builder:fn(TopologyBuilderArgument) -> Box<dyn Topology>)
	{
		self.topologies.insert(key,builder);
	}
	pub fn add_traffic(&mut self, key:String, builder:fn(TrafficBuilderArgument) -> Box<dyn Traffic>)
	{
		self.traffics.insert(key,builder);
	}
	pub fn add_routing(&mut self, key:String, builder:fn(RoutingBuilderArgument) -> Box<dyn Routing>)
	{
		self.routings.insert(key,builder);
	}
	pub fn add_policy(&mut self, key:String, builder: fn(VCPolicyBuilderArgument) -> Box<dyn VirtualChannelPolicy>)
	{
		self.policies.insert(key,builder);
	}
	pub fn add_pattern(&mut self, key:String, builder: fn(PatternBuilderArgument) -> Box<dyn Pattern>)
	{
		self.patterns.insert(key,builder);
	}
}

impl Debug for Plugs
{
	fn fmt(&self,f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error>
	{
		write!(f,"{};",self.routers.keys().map(|s|s.to_string()).collect::<Vec<String>>().join(","))?;
		write!(f,"{};",self.topologies.keys().map(|s|s.to_string()).collect::<Vec<String>>().join(","))?;
		write!(f,"{};",self.routings.keys().map(|s|s.to_string()).collect::<Vec<String>>().join(","))?;
		write!(f,"{};",self.traffics.keys().map(|s|s.to_string()).collect::<Vec<String>>().join(","))?;
		write!(f,"{};",self.patterns.keys().map(|s|s.to_string()).collect::<Vec<String>>().join(","))?;
		write!(f,"{};",self.policies.keys().map(|s|s.to_string()).collect::<Vec<String>>().join(","))?;
		Ok(())
	}
}

/// Main when passed a configuration file as path
/// `file` must be a configuration file with the experiment to simulate.
/// `plugs` constains the plugged builder functions.
/// `result_file` indicates where to write the results.
//pub fn file_main(file:&mut File, plugs:&Plugs, option_matches:&Matches)
pub fn file_main(file:&mut File, plugs:&Plugs, mut results_file:Option<File>)
{
	let mut contents = String::new();
	file.read_to_string(&mut contents).expect("something went wrong reading the file");

	//println!("With text:\n{}", contents);
	match config_parser::parse(&contents)
	{
		Err(x) => println!("error parsing configuration file: {:?}",x),
		Ok(x) =>
		{
			println!("parsed correctly: {:?}",x);
			match x
			{
				config_parser::Token::Value(ref value) =>
				{
					let flat=flatten_configuration_value(value);
					if let ConfigurationValue::Experiments(ref experiments)=flat
					{
						for (i,experiment) in experiments.iter().enumerate()
						{
							println!("experiment {} of {} is {:?}",i,experiments.len(),experiment);
							let mut simulation=Simulation::new(&experiment,plugs);
							simulation.run();
							match results_file
							{
								Some(ref mut f) => simulation.write_result(f),
								None => simulation.write_result(&mut stdout()),
							};
						}
					}
					else
					{
						panic!("there are not experiments");
					}
				},
				_ => panic!("Not a value"),
			};
		},
	};
}


/// Main when passed a directory as path
/// `path` must be a directory containing a `main.cfg`.
/// `plugs` constains the plugged builder functions.
/// `action` is the action to be performed in the experiment. For example running the simulations or drawing graphics.
/// `options` encapsulate other parameters such as restricting the performed action to a range of simulations.
//pub fn directory_main(path:&Path, binary:&str, plugs:&Plugs, option_matches:&Matches)
pub fn directory_main(path:&Path, binary:&str, plugs:&Plugs, action:Action, options: ExperimentOptions)
{
	let binary_path=Path::new(binary);
	//let mut experiment=Experiment::new(binary_path,path,plugs,option_matches);
	let mut experiment=Experiment::new(binary_path,path,plugs,options);
	//let action=if option_matches.opt_present("action")
	//{
	//	Action::from_str(&option_matches.opt_str("action").unwrap()).expect("Illegal action")
	//}
	//else
	//{
	//	Action::LocalAndOutput
	//};
	experiment.execute_action(action);
	//println!("{:?} is a path",path);
}

/// Get a identifier of the git commit. It is of little use to someone using a forzen public version.
/// The value is fixed in the build script.
pub fn get_git_id() -> &'static str
{
	include_str!(concat!(env!("OUT_DIR"), "/generated_git_id"))
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
