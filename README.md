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
			//In random regular graphs all router--router links have the same class.
			delay:1,
		},
		//The last class always correspond to the links between server and router
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

