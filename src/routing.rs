
/*!

A Routing defines the ways to select a next router to eventually reach the destination.

see [`new_routing`](fn.new_routing.html) for documentation on the configuration syntax of predefined routings.

*/

use crate::config_parser::ConfigurationValue;
use crate::topology::cartesian::{DOR,O1TURN,ValiantDOR,OmniDimensionalDeroute};
use crate::topology::{Topology,Location,NeighbourRouterIteratorItem};
use crate::matrix::Matrix;
use std::cell::RefCell;
use ::rand::{StdRng,Rng};
use quantifiable_derive::Quantifiable;//the derive macro
use crate::Plugs;
use std::fmt::Debug;
use std::convert::TryFrom;

///Information stored in the packet for the `Routing` algorithms to operate.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct RoutingInfo
{
	///Number of edges traversed (Router--Router). It is computed by the advance routine of the simulator.
	pub hops: usize,

	//All the remaining fields are used and computed by the Routing employed.
	///Difference in coordinates from origin to destination
	pub routing_record: Option<Vec<i32>>,
	///List of router indexes in the selected path from origin to destination
	pub selected_path: Option<Vec<usize>>,
	///Some selections made by the routing
	pub selections: Option<Vec<i32>>,
	///List of router indexes that have been visited already.
	pub visited_routers: Option<Vec<usize>>,
	///Mostly for the generic Valiant scheme.
	pub meta: Option<Vec<RefCell<RoutingInfo>>>,
}

impl RoutingInfo
{
	pub fn new() -> RoutingInfo
	{
		RoutingInfo{
			hops: 0,
			routing_record: None,
			selected_path: None,
			selections: None,
			visited_routers: None,
			meta: None,
		}
	}
}

///Annotations by the routing to keep track of the candidates.
#[derive(Clone,Debug,Default)]
pub struct RoutingAnnotation
{
	values: Vec<i32>,
	meta: Vec<Option<RoutingAnnotation>>,
}

///Represent a port plus additional information that a routing algorithm can determine on how a packet must advance to the next router or server.
#[derive(Clone)]
#[derive(Debug,Default)]
pub struct CandidateEgress
{
	///Candidate exit port
	pub port: usize,
	///Candidate virtual channel in which being inserted.
	pub virtual_channel: usize,
	///Value used to indicate priorities. Semantics defined per routing and policy. Routing should use low values for more priority.
	pub label: i32,
	///An estimation of the number of hops pending. This include the hop we are requesting.
	pub estimated_remaining_hops: Option<usize>,

	///The routing must set this to None.
	///The `Router` can set it to `Some(true)` when it satisfies all flow-cotrol criteria and to `Some(false)` when it fails any criterion.
	pub router_allows: Option<bool>,

	///Annotations for the routing to know to what candidate the router refers.
	///It should be preserved by the policies.
	pub annotation: Option<RoutingAnnotation>,
}

impl CandidateEgress
{
	pub fn new(port:usize, virtual_channel:usize)->CandidateEgress
	{
		CandidateEgress{
			port,
			virtual_channel,
			label: 0,
			estimated_remaining_hops: None,
			router_allows: None,
			annotation: None,
		}
	}
}

///A routing algorithm to provide candidate routes when the `Router` requires.
///It may store/use information in the RoutingInfo.
///A `Routing` does not receive information about the state of buffers or similar. Such a mechanism should be given as a `VirtualChannelPolicy`.
pub trait Routing : Debug
{
	///Compute the list of allowed exits.
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>;
	//fn initialize_routing_info(&self, routing_info:&mut RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize);
	///Initialize the routing info of the packet. Called when the first phit of the packet leaves the server and enters a router.
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>);
	///Updates the routing info of the packet. Called when the first phit of the packet leaves a router and enters another router. Values are of the router being entered into.
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize,rng: &RefCell<StdRng>);
	///Prepares the routing to be utilized. Perhaps by precomputing routing tables.
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>);
	///To be called by the router when one of the candidates is requested.
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng:&RefCell<StdRng>);
	///To optionally write routing statistics into the simulation output.
	fn statistics(&self,cycle:usize) -> Option<ConfigurationValue>;
	///Clears all collected statistics
	fn reset_statistics(&mut self,next_cycle:usize);
}

///The argument of a builder function for `Routings`.
#[derive(Debug)]
pub struct RoutingBuilderArgument<'a>
{
	///A ConfigurationValue::Object defining the routing.
	pub cv: &'a ConfigurationValue,
	///The user defined plugs. In case the routing needs to create elements.
	pub plugs: &'a Plugs,
}

/**Build a new routing.

## Generic routings

```
Shortest{
	legend_name: "minimal routing",
}
```

```
Valiant{
	first: Shortest,
	second: Shortest,
	legend_name: "Using Valiant scheme, shortest to intermediate and shortest to destination",
	//selection_exclude_indirect_routers: false,//optional parameter
}
```

For topologies that define global links:
```
WeighedShortest{
	class_weight: [1,100],
	legend_name: "Shortest avoiding using several global links",
}
```

For multi-stage topologies we may use
```
UpDown{
	legend_name: "up/down routing",
}
```

## Operations

### Sum
To use some of two routings depending on whatever. virtual channels not on either list can be used freely. The extra label field can be used to set the priorities. Check the router policies for that.
```
Sum{
	policy: TryBoth,//or Random
	first_routing: Shortest,
	second_routing: Valiant{first:Shortest,second:Shortest},
	first_allowed_virtual_channels: [0,1],
	second_allowed_virtual_channels: [2,3,4,5],
	first_extra_label:0,//optional
	second_extra_label:10,//optiona
	legend_name: "minimal with high priority and Valiant with low priority",
}
```

### ChannelsPerHop
Modify a routing to use a given list of virtual channels each hop.
```
ChannelsPerHop{
	routing: Shortest,
	channels: [
		[0],//the first hop from a router to another router
		[1],
		[2],
		[0,1,2],//the last hop, to the server
	],
}
```

### ChannelsPerHopPerLinkClass
Modify a routing to use a given list of virtual channels each hop.
```
ChannelsPerHopPerLinkClass{
	routing: Shortest,
	channels: [
		[ [0],[1] ],//links in class 0.
		[ [0],[1] ],//links in class 1.
		[ [0,1] ],//links in class 2. Last class is towards servers. 
	],
}
```

### ChannelMap
```
ChannelMap{
	routing: Shortest,
	map: [
		[1],//map the virtual channel 0 into the vc 1
		[2,3],//the vc 1 is doubled into 2 and 3
		[4],
	],
}
```

### AscendantChannelsWithLinkClass
Virtual channels are used in ascent way. With higher classes meaning higher digits.
```
AscendantChannelsWithLinkClass{
	routing: Shortest,
	bases: [2,1],//allow two consecutive hops of class 0 before a hop of class 1
}
```

### Stubborn makes a routing to calculate candidates just once. If that candidate is not accepted is trying again every cycle.
```
Stubborn{
	routing: Shortest,
	legend_name: "stubborn minimal",
}
```

## Cartesian-specific routings

### DOR

The dimensional ordered routing. Packets will go minimal along the first dimension as much possible and then on the next.

```
DOR{
	order: [0,1],
	legend_name: "dimension ordered routing, 0 before 1",
}
```


### O1TURN
O1TURN is a pair of DOR to balance the usage of the links.

```
O1TURN{
	reserved_virtual_channels_order01: [0],
	reserved_virtual_channels_order10: [1],
	legend_name: "O1TURN",
}
```

### OmniDimensional

McDonal OmniDimensional routing for HyperX. it is a shortest with some allowed deroutes. It does not allow deroutes on unaligned dimensions.

```
OmniDimensionalDeroute{
	allowed_deroutes: 3,
	include_labels: true,//deroutes are given higher labels, implying lower priority. Check router policies.
	legend_name: "McDonald OmniDimensional routing allowing 3 deroutes",
}
```

### ValiantDOR

A proposal by Valiant for Cartesian topologies. It randomizes all-but-one coordinates, followed by a DOR starting by the non-randomized coordinate.

```
ValiantDOR{
	randomized: [2,1],
	shortest: [0,1,2],
	randomized_reserved_virtual_channels: [1],
	shortest_reserved_virtual_channels: [0],
	legend_name: "The less-known proposal of Valiant for Cartesian topologies",
}
```

*/
pub fn new_routing(arg: RoutingBuilderArgument) -> Box<dyn Routing>
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=arg.cv
	{
		match arg.plugs.routings.get(cv_name)
		{
			Some(builder) => return builder(arg),
			_ => (),
		};
		match cv_name.as_ref()
		{
			"DOR" => Box::new(DOR::new(arg)),
			"O1TURN" => Box::new(O1TURN::new(arg)),
			"OmniDimensionalDeroute" => Box::new(OmniDimensionalDeroute::new(arg)),
			"Shortest" => Box::new(Shortest::new(arg)),
			"Valiant" => Box::new(Valiant::new(arg)),
			"ValiantDOR" => Box::new(ValiantDOR::new(arg)),
			"Sum" => Box::new(SumRouting::new(arg)),
			"Mindless" => Box::new(Mindless::new(arg)),
			"WeighedShortest" => Box::new(WeighedShortest::new(arg)),
			"Stubborn" => Box::new(Stubborn::new(arg)),
			"UpDown" => Box::new(UpDown::new(arg)),
			"UpDownStar" => Box::new(ExplicitUpDown::new(arg)),
			"ChannelsPerHop" => Box::new(ChannelsPerHop::new(arg)),
			"ChannelsPerHopPerLinkClass" => Box::new(ChannelsPerHopPerLinkClass::new(arg)),
			"AscendantChannelsWithLinkClass" => Box::new(AscendantChannelsWithLinkClass::new(arg)),
			"ChannelMap" => Box::new(ChannelMap::new(arg)),
			_ => panic!("Unknown Routing {}",cv_name),
		}
	}
	else
	{
		panic!("Trying to create a Routing from a non-Object");
	}
}

///Use the shortest path from origin to destination
#[derive(Debug)]
pub struct Shortest
{
}

impl Routing for Shortest
{
	fn next(&self, _routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let distance=topology.distance(current_router,target_router);
		if distance==0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
			{
				if distance-1==topology.distance(router_index,target_router)
				{
					//r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
					r.extend((0..num_virtual_channels).map(|vc|{
						let mut egress = CandidateEgress::new(i,vc);
						egress.estimated_remaining_hops = Some(distance);
						egress
					}));
				}
			}
		}
		//println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		r
	}
	fn initialize_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn update_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn initialize(&mut self, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl Shortest
{
	pub fn new(arg: RoutingBuilderArgument) -> Shortest
	{
		//let mut order=None;
		//let mut servers_per_router=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Shortest"
			{
				panic!("A Shortest must be created from a `Shortest` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"order" => match value
					//{
					//	&ConfigurationValue::Array(ref a) => order=Some(a.iter().map(|v|match v{
					//		&ConfigurationValue::Number(f) => f as usize,
					//		_ => panic!("bad value in order"),
					//	}).collect()),
					//	_ => panic!("bad value for order"),
					//}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Shortest",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a Shortest from a non-Object");
		}
		//let order=order.expect("There were no order");
		Shortest{
		}
	}
}

#[derive(Debug)]
pub struct Valiant
{
	first: Box<dyn Routing>,
	second: Box<dyn Routing>,
	///Whether to avoid selecting routers without attached servers. This helps to apply it to indirect networks.
	selection_exclude_indirect_routers: bool,
	first_reserved_virtual_channels: Vec<usize>,
	second_reserved_virtual_channels: Vec<usize>,
}

impl Routing for Valiant
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let distance=topology.distance(current_router,target_router);
		if distance==0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let meta=routing_info.meta.as_ref().unwrap();
		match routing_info.selections
		{
			None =>
			{
				//self.second.next(&meta[1].borrow(),topology,current_router,target_server,num_virtual_channels,rng)
				self.second.next(&meta[1].borrow(),topology,current_router,target_server,num_virtual_channels,rng).into_iter().filter(|egress|!self.first_reserved_virtual_channels.contains(&egress.virtual_channel)).collect()
			}
			Some(ref s) =>
			{
				let middle=s[0] as usize;
				let middle_server=
				{
					let mut x=None;
					for i in 0..topology.ports(middle)
					{
						if let (Location::ServerPort(server),_link_class)=topology.neighbour(middle,i)
						{
							x=Some(server);
							break;
						}
					}
					x.unwrap()
				};
				let second_distance=topology.distance(middle,target_router);//Only exact if the base routing is shortest.
				//self.first.next(&meta[0].borrow(),topology,current_router,middle_server,num_virtual_channels,rng).into_iter().filter(|egress|!self.second_reserved_virtual_channels.contains(&egress.virtual_channel)).collect()
				self.first.next(&meta[0].borrow(),topology,current_router,middle_server,num_virtual_channels,rng).into_iter().filter_map(|mut egress|{
					if self.second_reserved_virtual_channels.contains(&egress.virtual_channel) { None } else {
						if let Some(ref mut eh)=egress.estimated_remaining_hops
						{
							*eh += second_distance;
						}
						Some(egress)
					}
				}).collect()
			}
		}
		// let num_ports=topology.ports(current_router);
		// let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		// for i in 0..num_ports
		// {
		// 	//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
		// 	if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
		// 	{
		// 		if distance-1==topology.distance(router_index,target_router)
		// 		{
		// 			r.extend((0..num_virtual_channels).map(|vc|(i,vc)));
		// 		}
		// 	}
		// }
		// //println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		// r
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let n=topology.num_routers();
		let middle = if self.selection_exclude_indirect_routers
		{
			let available : Vec<usize> = (0..n).filter(|&index|{
				for i in 0..topology.ports(index)
				{
					if let (Location::ServerPort(_),_) = topology.neighbour(index,i)
					{
						return true;
					}
				}
				false//there is not server in this router, hence it is excluded
			}).collect();
			if available.len()==0
			{
				panic!("There are not legal middle routers to select in Valiant from router {} towards router {}",current_router,target_router);
			}
			let r = rng.borrow_mut().gen_range(0,available.len());
			available[r]
		} else {
			rng.borrow_mut().gen_range(0,n)
		};
		let mut bri=routing_info.borrow_mut();
		bri.meta=Some(vec![RefCell::new(RoutingInfo::new()),RefCell::new(RoutingInfo::new())]);
		if middle==current_router || middle==target_router
		{
			self.second.initialize_routing_info(&bri.meta.as_ref().unwrap()[1],topology,current_router,target_server,rng);
		}
		else
		{
			bri.selections=Some(vec![middle as i32]);
			//FIXME: what do we do when we are not excluding indirect routers?
			let middle_server=
			{
				let mut x=None;
				for i in 0..topology.ports(middle)
				{
					if let (Location::ServerPort(server),_link_class)=topology.neighbour(middle,i)
					{
						x=Some(server);
						break;
					}
				}
				x.unwrap()
			};
			self.first.initialize_routing_info(&bri.meta.as_ref().unwrap()[0],topology,current_router,middle_server,rng)
		}
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let mut bri=routing_info.borrow_mut();
		let middle=match bri.selections
		{
			None => None,
			Some(ref s) => Some(s[0] as usize),
		};
		match middle
		{
			None =>
			{
				//Already towards true destination
				let meta=bri.meta.as_mut().unwrap();
				meta[1].borrow_mut().hops+=1;
				self.second.update_routing_info(&meta[1],topology,current_router,current_port,target_server,rng);
			}
			Some(middle) =>
			{
				if current_router==middle
				{
					bri.selections=None;
					let meta=bri.meta.as_ref().unwrap();
					self.second.initialize_routing_info(&meta[1],topology,current_router,target_server,rng);
				}
				else
				{
					//FIXME: that target_server
					let meta=bri.meta.as_mut().unwrap();
					meta[0].borrow_mut().hops+=1;
					self.first.update_routing_info(&meta[0],topology,current_router,current_port,target_server,rng);
				}
			}
		};
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.first.initialize(topology,rng);
		self.second.initialize(topology,rng);
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
		//TODO: recurse over routings
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl Valiant
{
	pub fn new(arg: RoutingBuilderArgument) -> Valiant
	{
		//let mut order=None;
		//let mut servers_per_router=None;
		let mut first=None;
		let mut second=None;
		let mut selection_exclude_indirect_routers=false;
		let mut first_reserved_virtual_channels=vec![];
		let mut second_reserved_virtual_channels=vec![];
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Valiant"
			{
				panic!("A Valiant must be created from a `Valiant` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"order" => match value
					//{
					//	&ConfigurationValue::Array(ref a) => order=Some(a.iter().map(|v|match v{
					//		&ConfigurationValue::Number(f) => f as usize,
					//		_ => panic!("bad value in order"),
					//	}).collect()),
					//	_ => panic!("bad value for order"),
					//}
					"first" =>
					{
						first=Some(new_routing(RoutingBuilderArgument{cv:value,..arg}));
					}
					"second" =>
					{
						second=Some(new_routing(RoutingBuilderArgument{cv:value,..arg}));
					}
					"selection_exclude_indirect_routers" => match value
					{
						&ConfigurationValue::True => selection_exclude_indirect_routers=true,
						&ConfigurationValue::False => selection_exclude_indirect_routers=false,
						_ => panic!("bad value for selection_exclude_indirect_routers"),
					},
					"first_reserved_virtual_channels" => match value
					{
						&ConfigurationValue::Array(ref a) => first_reserved_virtual_channels=a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in first_reserved_virtual_channels"),
						}).collect(),
						_ => panic!("bad value for first_reserved_virtual_channels"),
					}
					"second_reserved_virtual_channels" => match value
					{
						&ConfigurationValue::Array(ref a) => second_reserved_virtual_channels=a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in second_reserved_virtual_channels"),
						}).collect(),
						_ => panic!("bad value for first_reserved_virtual_channels"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Valiant",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a Valiant from a non-Object");
		}
		let first=first.expect("There were no first");
		let second=second.expect("There were no second");
		//let first_reserved_virtual_channels=first_reserved_virtual_channels.expect("There were no first_reserved_virtual_channels");
		//let second_reserved_virtual_channels=second_reserved_virtual_channels.expect("There were no second_reserved_virtual_channels");
		Valiant{
			first,
			second,
			selection_exclude_indirect_routers,
			first_reserved_virtual_channels,
			second_reserved_virtual_channels,
		}
	}
}


///Trait for `Routing`s that build the whole route at source.
///This includes routings such as K-shortest paths. But I have all my implementations depending on a private algorithm, so they are not yet here.
///They will all be released when the dependency is formally published.
pub trait SourceRouting
{
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>);
	fn get_paths(&self, source:usize, target:usize) -> &Vec<Vec<usize>>;
}

pub trait InstantiableSourceRouting : SourceRouting + Debug {}
impl<R:SourceRouting + Debug> InstantiableSourceRouting for R {}

impl<R:SourceRouting+Debug> Routing for R
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let distance=topology.distance(current_router,target_router);
		if distance==0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		let next_router=routing_info.selected_path.as_ref().unwrap()[routing_info.hops+1];
		let length =routing_info.selected_path.as_ref().unwrap().len() - 1;//substract source router
		let remain = length - routing_info.hops;
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
			{
				//if distance-1==topology.distance(router_index,target_router)
				if router_index==next_router
				{
					//r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
					r.extend((0..num_virtual_channels).map(|vc|{
						let mut egress = CandidateEgress::new(i,vc);
						egress.estimated_remaining_hops = Some(remain);
						egress
					}));
				}
			}
		}
		//println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		r
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		if current_router!=target_router
		{
			//let path_collection = &self.paths[current_router][target_router];
			let path_collection = self.get_paths(current_router,target_router);
			//println!("path_collection.len={} for source={} target={}\n",path_collection.len(),current_router,target_router);
			if path_collection.is_empty()
			{
				panic!("No path found from router {} to router {}",current_router,target_router);
			}
			let r=rng.borrow_mut().gen_range(0,path_collection.len());
			routing_info.borrow_mut().selected_path=Some(path_collection[r].clone());
		}
	}
	fn update_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
		//Nothing to do on update
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.initialize(topology,rng);
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}





///A policy for the `SumRouting` about how to select among the two `Routing`s.
#[derive(Debug)]
pub enum SumRoutingPolicy
{
	Random,
	TryBoth,
	Stubborn,
	StubbornWhenSecond,
}

pub fn new_sum_routing_policy(cv: &ConfigurationValue) -> SumRoutingPolicy
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=cv
	{
		match cv_name.as_ref()
		{
			"Random" => SumRoutingPolicy::Random,
			"TryBoth" => SumRoutingPolicy::TryBoth,
			"Stubborn" => SumRoutingPolicy::Stubborn,
			"StubbornWhenSecond" => SumRoutingPolicy::StubbornWhenSecond,
			//"Shortest" => SumRoutingPolicy::Shortest,
			//"Hops" => SumRoutingPolicy::Hops,
			_ => panic!("Unknown sum routing policy {}",cv_name),
		}
	}
	else
	{
		panic!("Trying to create a SumRoutingPolicy from a non-Object");
	}
}

/// To employ two different routings. It will use either `first_routing` or `second_routing` according to policy.
#[derive(Debug)]
pub struct SumRouting
{
	policy:SumRoutingPolicy,
	//first_routing:Box<dyn Routing>,
	//second_routing:Box<dyn Routing>,
	routing: [Box<dyn Routing>;2],
	//first_allowed_virtual_channels: Vec<usize>,
	//second_allowed_virtual_channels: Vec<usize>,
	allowed_virtual_channels: [Vec<usize>;2],
	//first_extra_label: i32,
	//second_extra_label: i32,
	extra_label: [i32;2],
}

//routin_info.selections uses
//* [a] if a specific routing a has been decided
//* [a,b] if the two routings are available
//* [a,b,c] if a request by routing c has been made, but the two routing are still available.
impl Routing for SumRouting
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let distance=topology.distance(current_router,target_router);
		if distance==0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let meta=routing_info.meta.as_ref().unwrap();
		match routing_info.selections
		{
			None =>
			{
				unreachable!();
			}
			Some(ref s) =>
			{
				//let both = if let &SumRoutingPolicy::TryBoth=&self.policy { routing_info.hops==0 } else { false };
				//if both
				if s.len()>=2
				{
					//let avc0=&self.first_allowed_virtual_channels;
					let avc0=&self.allowed_virtual_channels[0];
					//let el0=self.first_extra_label;
					let el0=self.extra_label[0];
					//let r0=self.first_routing.next(&meta[0].borrow(),topology,current_router,target_server,avc0.len(),rng).into_iter().map( |candidate| CandidateEgress{virtual_channel:avc0[candidate.virtual_channel],label:candidate.label+el0,annotation:Some(RoutingAnnotation{values:vec![0],meta:vec![candidate.annotation]}),..candidate} );
					let r0=self.routing[0].next(&meta[0].borrow(),topology,current_router,target_server,avc0.len(),rng).into_iter().map( |candidate| CandidateEgress{virtual_channel:avc0[candidate.virtual_channel],label:candidate.label+el0,annotation:Some(RoutingAnnotation{values:vec![0],meta:vec![candidate.annotation]}),..candidate} );
					//let avc1=&self.second_allowed_virtual_channels;
					let avc1=&self.allowed_virtual_channels[1];
					//let el1=self.second_extra_label;
					let el1=self.extra_label[1];
					//let r1=self.second_routing.next(&meta[1].borrow(),topology,current_router,target_server,avc1.len(),rng).into_iter().map( |candidate| CandidateEgress{virtual_channel:avc1[candidate.virtual_channel],label:candidate.label+el1,annotation:Some(RoutingAnnotation{values:vec![1],meta:vec![candidate.annotation]}),..candidate} );
					let r1=self.routing[1].next(&meta[1].borrow(),topology,current_router,target_server,avc1.len(),rng).into_iter().map( |candidate| CandidateEgress{virtual_channel:avc1[candidate.virtual_channel],label:candidate.label+el1,annotation:Some(RoutingAnnotation{values:vec![1],meta:vec![candidate.annotation]}),..candidate} );
					r0.chain(r1).collect()
				}
				else
				{
					let index=s[0] as usize;
					//let routing=if s[0]==0 { &self.first_routing } else { &self.second_routing };
					let routing = &self.routing[index];
					//let allowed_virtual_channels=if s[0]==0 { &self.first_allowed_virtual_channels } else { &self.second_allowed_virtual_channels };
					let allowed_virtual_channels = &self.allowed_virtual_channels[index];
					//let extra_label = if s[0]==0 { self.first_extra_label } else { self.second_extra_label };
					let extra_label = self.extra_label[index];
					let r=routing.next(&meta[index].borrow(),topology,current_router,target_server,allowed_virtual_channels.len(),rng);
					//r.into_iter().map( |(x,c)| (x,allowed_virtual_channels[c]) ).collect()
					r.into_iter()
					//.map( |CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}| CandidateEgress{port,virtual_channel:allowed_virtual_channels[virtual_channel],label,estimated_remaining_hops} ).collect()
					.map( |candidate| CandidateEgress{virtual_channel:allowed_virtual_channels[candidate.virtual_channel],label:candidate.label+extra_label,..candidate} ).collect()
				}
			}
		}
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let all:Vec<i32> = match self.policy
		{
			SumRoutingPolicy::Random => vec![rng.borrow_mut().gen_range(0,2)],
			SumRoutingPolicy::TryBoth | SumRoutingPolicy::Stubborn | SumRoutingPolicy::StubbornWhenSecond => vec![0,1],
		};
		let mut bri=routing_info.borrow_mut();
		//bri.meta=Some(vec![RefCell::new(RoutingInfo::new()),RefCell::new(RoutingInfo::new())]);
		bri.meta=Some(vec![RefCell::new(RoutingInfo::new()),RefCell::new(RoutingInfo::new())]);
		for &s in all.iter()
		{
			//let routing=if s==0 { &self.first_routing } else { &self.second_routing };
			let routing = &self.routing[s as usize];
			routing.initialize_routing_info(&bri.meta.as_ref().unwrap()[s as usize],topology,current_router,target_server,rng)
		}
		bri.selections=Some(all);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let mut bri=routing_info.borrow_mut();
		let s=match bri.selections
		{
			None => unreachable!(),
			Some(ref t) => if t.len()==1 {
				t[0] as usize
			} else {
				let s=t[2];
				bri.selections=Some(vec![s]);
				s as usize
			},
		};
		//let routing=if s==0 { &self.first_routing } else { &self.second_routing };
		let routing = &self.routing[s];
		let meta=bri.meta.as_mut().unwrap();
		meta[s].borrow_mut().hops+=1;
		routing.update_routing_info(&meta[s],topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		//self.first_routing.initialize(topology,rng);
		//self.second_routing.initialize(topology,rng);
		self.routing[0].initialize(topology,rng);
		self.routing[1].initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
		let mut bri=routing_info.borrow_mut();
		//if let SumRoutingPolicy::TryBoth=self.policy
		//if let SumRoutingPolicy::Stubborn | SumRoutingPolicy::StubbornWhenSecond =self.policy
		if bri.selections.as_ref().unwrap().len()>1
		{
			let &CandidateEgress{ref annotation,..} = requested;
			if let Some(annotation) = annotation.as_ref()
			{
				let s = annotation.values[0];
				match self.policy
				{
					SumRoutingPolicy::Stubborn => bri.selections=Some(vec![s]),
					SumRoutingPolicy::StubbornWhenSecond => bri.selections = if s==1 {
						Some(vec![1])
					} else {
						Some( vec![ bri.selections.as_ref().unwrap()[0],bri.selections.as_ref().unwrap()[1],s ] )
					},
					_ => bri.selections = Some( vec![ bri.selections.as_ref().unwrap()[0],bri.selections.as_ref().unwrap()[1],s ] ),
				}
			}
		}
		//TODO: recurse over subroutings
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl SumRouting
{
	pub fn new(arg: RoutingBuilderArgument) -> SumRouting
	{
		let mut policy=None;
		let mut first_routing=None;
		let mut second_routing=None;
		let mut first_allowed_virtual_channels=None;
		let mut second_allowed_virtual_channels=None;
		let mut first_extra_label=0i32;
		let mut second_extra_label=0i32;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Sum"
			{
				panic!("A SumRouting must be created from a `Sum` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"policy" => policy=Some(new_sum_routing_policy(value)),
					"first_routing" => first_routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg})),
					"second_routing" => second_routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg})),
					"first_allowed_virtual_channels" => match value
					{
						&ConfigurationValue::Array(ref a) => first_allowed_virtual_channels=Some(a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in first_allowed_virtual_channels"),
						}).collect()),
						_ => panic!("bad value for first_allowed_virtual_channels"),
					}
					"second_allowed_virtual_channels" => match value
					{
						&ConfigurationValue::Array(ref a) => second_allowed_virtual_channels=Some(a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in second_allowed_virtual_channels"),
						}).collect()),
						_ => panic!("bad value for first_allowed_virtual_channels"),
					}
					"first_extra_label" => match value
					{
						&ConfigurationValue::Number(x) => first_extra_label=x as i32,
						_ => panic!("bad value for first_extra_label"),
					},
					"second_extra_label" => match value
					{
						&ConfigurationValue::Number(x) => second_extra_label=x as i32,
						_ => panic!("bad value for second_extra_label"),
					},
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in SumRouting",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a SumRouting from a non-Object");
		}
		let policy=policy.expect("There were no policy");
		let first_routing=first_routing.expect("There were no first_routing");
		let second_routing=second_routing.expect("There were no second_routing");
		let first_allowed_virtual_channels=first_allowed_virtual_channels.expect("There were no first_allowed_virtual_channels");
		let second_allowed_virtual_channels=second_allowed_virtual_channels.expect("There were no second_allowed_virtual_channels");
		SumRouting{
			policy,
			//first_routing,
			//second_routing,
			routing: [first_routing,second_routing],
			//first_allowed_virtual_channels,
			//second_allowed_virtual_channels,
			allowed_virtual_channels: [first_allowed_virtual_channels, second_allowed_virtual_channels],
			//first_extra_label,
			//second_extra_label,
			extra_label: [first_extra_label, second_extra_label],
		}
	}
}


///Mindless routing
///Employ any path until reaching a router with the server atached.
///The interested may read a survey of random walks on graphs to try to predict the time to reach the destination. For example "Random Walks on Graphs: A Survey" by L. Lovász.
///Note that every cycle the request is made again. Hence, the walk is not actually unform random when there is network contention.
#[derive(Debug)]
pub struct Mindless
{
}

impl Routing for Mindless
{
	fn next(&self, _routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		if target_router==current_router
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index:_,router_port:_},_link_class)=topology.neighbour(current_router,i)
			{
				//r.extend((0..num_virtual_channels).map(|vc|(i,vc)));
				r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
			}
		}
		r
	}
	fn initialize_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn update_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn initialize(&mut self, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl Mindless
{
	pub fn new(arg: RoutingBuilderArgument) -> Mindless
	{
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Mindless"
			{
				panic!("A Mindless must be created from a `Mindless` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Mindless",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a Mindless from a non-Object");
		}
		Mindless{
		}
	}
}

///Use the shortest path from origin to destination, giving a weight to each link class.
///Note that it uses information based on BFS and not on Dijkstra, which may cause discrepancies in some topologies.
///See the `Topology::compute_distance_matrix` and its notes on weights for more informations.
#[derive(Debug)]
pub struct WeighedShortest
{
	///The weights used for each link class. Only relevant links between routers.
	class_weight:Vec<usize>,
	///The distance matrix computed, including weights.
	distance_matrix: Matrix<usize>,
}

impl Routing for WeighedShortest
{
	fn next(&self, _routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		//let distance=topology.distance(current_router,target_router);
		let distance=*self.distance_matrix.get(current_router,target_router);
		//let valid = vec![0,1,2,100,101,102];
		//if !valid.contains(&distance){ panic!("distance={}",distance); }
		if distance==0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index,router_port:_},link_class)=topology.neighbour(current_router,i)
			{
				let link_weight = self.class_weight[link_class];
				//if distance>*self.distance_matrix.get(router_index,target_router)
				let new_distance = *self.distance_matrix.get(router_index,target_router);
				if new_distance + link_weight == distance
				{
					//if ![(102,1),(1,1),(101,100),(100,100),(101,1)].contains(&(distance,link_weight)){
					//	println!("distance={} link_weight={}",distance,link_weight);
					//}
					//println!("distance={} link_weight={} hops={}",distance,link_weight,routing_info.hops);
					//r.extend((0..num_virtual_channels).map(|vc|(i,vc)));
					r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
				}
			}
		}
		//println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		r
	}
	//fn initialize_routing_info(&self, routing_info:&mut RoutingInfo, toology:&dyn Topology, current_router:usize, target_server:usize)
	fn initialize_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn update_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
		self.distance_matrix=topology.compute_distance_matrix(Some(&self.class_weight));
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl WeighedShortest
{
	pub fn new(arg: RoutingBuilderArgument) -> WeighedShortest
	{
		//let mut order=None;
		//let mut servers_per_router=None;
		let mut class_weight=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="WeighedShortest"
			{
				panic!("A WeighedShortest must be created from a `WeighedShortest` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"class_weight" => match value
					{
						&ConfigurationValue::Array(ref a) => class_weight=Some(a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in class_weight"),
						}).collect()),
						_ => panic!("bad value for class_weight"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in WeighedShortest",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a WeighedShortest from a non-Object");
		}
		let class_weight=class_weight.expect("There were no class_weight");
		WeighedShortest{
			class_weight,
			distance_matrix:Matrix::constant(0,0,0),
		}
	}
}


///Stubborn routing
///Wraps a routing so that only one request is made in every router.
///The first time the router make a port request, that request is stored and repeated in further calls to `next` until reaching a new router.
///Stores port, virtual_channel, label into routing_info.selections.
#[derive(Debug)]
pub struct Stubborn
{
	routing: Box<dyn Routing>,
}

impl Routing for Stubborn
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		if target_router==current_router
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		if let Some(ref sel)=routing_info.selections
		{
			return vec![CandidateEgress{port:sel[0] as usize,virtual_channel:sel[1] as usize,label:sel[2],..Default::default()}]
		}
		//return self.routing.next(&routing_info.meta.as_ref().unwrap()[0].borrow(),topology,current_router,target_server,num_virtual_channels,rng)
		return self.routing.next(&routing_info.meta.as_ref().unwrap()[0].borrow(),topology,current_router,target_server,num_virtual_channels,rng).into_iter().map(|candidate|CandidateEgress{annotation:Some(RoutingAnnotation{values:vec![candidate.label],meta:vec![candidate.annotation]}),..candidate}).collect()
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let meta_routing_info=RefCell::new(RoutingInfo::new());
		self.routing.initialize_routing_info(&meta_routing_info, topology, current_router, target_server, rng);
		routing_info.borrow_mut().meta = Some(vec![meta_routing_info]);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let mut bri=routing_info.borrow_mut();
		bri.selections=None;
		self.routing.update_routing_info(&bri.meta.as_mut().unwrap()[0],topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.routing.initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng:&RefCell<StdRng>)
	{
		let &CandidateEgress{port,virtual_channel,ref annotation,..} = requested;
		if let Some(annotation) = annotation.as_ref()
		{
			let label = annotation.values[0];
			//routing_info.borrow_mut().selections=Some(vec![port as i32, virtual_channel as i32, label]);
			let mut bri=routing_info.borrow_mut();
			bri.selections=Some(vec![port as i32, virtual_channel as i32, label]);
			//recurse over routing
			let meta_requested = CandidateEgress{annotation:annotation.meta[0].clone(),..*requested};
			//let meta_info = &routing_info.borrow().meta.as_ref().unwrap()[0];
			let meta_info = &bri.meta.as_ref().unwrap()[0];
			self.routing.performed_request(&meta_requested,meta_info,topology,current_router,target_server,num_virtual_channels,rng);
		}
		//otherwise it is direct to server
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl Stubborn
{
	pub fn new(arg: RoutingBuilderArgument) -> Stubborn
	{
		let mut routing=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Stubborn"
			{
				panic!("A Stubborn must be created from a `Stubborn` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"routing" =>
					{
						routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg}));
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Stubborn",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a Stubborn from a non-Object");
		}
		let routing=routing.expect("There were no routing");
		Stubborn{
			routing,
		}
	}
}

///Use a shortest up/down path from origin to destination.
///The up/down paths are understood as provided by `Topology::up_down_distance`.
#[derive(Debug)]
pub struct UpDown
{
}

impl Routing for UpDown
{
	fn next(&self, _routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let (up_distance, down_distance) = topology.up_down_distance(current_router,target_router).unwrap_or_else(||panic!("The topology does not provide an up/down path from {} to {}",current_router,target_router));
		if up_distance + down_distance == 0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
			{
				if let Some((new_u, new_d)) = topology.up_down_distance(router_index,target_router)
				{
					if (new_u<up_distance && new_d<=down_distance) || (new_u<=up_distance && new_d<down_distance)
					{
						r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
					}
				}
			}
		}
		//println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		r
	}
	fn initialize_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn update_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn initialize(&mut self, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl UpDown
{
	pub fn new(arg: RoutingBuilderArgument) -> UpDown
	{
		//let mut order=None;
		//let mut servers_per_router=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="UpDown"
			{
				panic!("A UpDown must be created from a `UpDown` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"order" => match value
					//{
					//	&ConfigurationValue::Array(ref a) => order=Some(a.iter().map(|v|match v{
					//		&ConfigurationValue::Number(f) => f as usize,
					//		_ => panic!("bad value in order"),
					//	}).collect()),
					//	_ => panic!("bad value for order"),
					//}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in UpDown",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a UpDown from a non-Object");
		}
		//let order=order.expect("There were no order");
		UpDown{
		}
	}
}

///Use a shortest up/down path from origin to destination.
///But in contrast with UpDown this uses explicit table instead of querying the topology.
///Used to define Up*/Down* (UpDownStar), see Autonet, where it is build from some spanning tree.
#[derive(Debug)]
pub struct ExplicitUpDown
{
	//defining factors to be kept up to initialization
	root: Option<usize>,
	//computed at initialization
	up_down_distances: Matrix<Option<(u8,u8)>>,
}

impl Routing for ExplicitUpDown
{
	fn next(&self, _routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let (up_distance, down_distance) = self.up_down_distances.get(current_router,target_router).unwrap_or_else(||panic!("Missing up/down path from {} to {}",current_router,target_router));
		if up_distance + down_distance == 0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
			{
				if let &Some((new_u, new_d)) = self.up_down_distances.get(router_index,target_router)
				{
					if (new_u<up_distance && new_d<=down_distance) || (new_u<=up_distance && new_d<down_distance)
					{
						r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
					}
				}
			}
		}
		//println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		r
	}
	fn initialize_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn update_routing_info(&self, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
		let n = topology.num_routers();
		if let Some(root) = self.root
		{
			self.up_down_distances = Matrix::constant(None,n,n);
			//First perform a single BFS at root.
			let mut distance_to_root=vec![None;n];
			distance_to_root[root]=Some(0);
			//The updwards BFS.
			dbg!(root,"upwards");
			for current in 0..n
			{
				if let Some(current_distance) = distance_to_root[current]
				{
					let alternate_distance = current_distance + 1;
					for NeighbourRouterIteratorItem{neighbour_router:neighbour,..} in topology.neighbour_router_iter(current)
					{
						if let None = distance_to_root[neighbour]
						{
							distance_to_root[neighbour]=Some(alternate_distance);
						}
					}
				}
			}
			//Second fill assuming going through root
			dbg!(root,"fill");
			for origin in 0..n
			{
				if let Some(origin_to_root) = distance_to_root[origin]
				{
					for target in 0..n
					{
						if let Some(target_to_root) = distance_to_root[target]
						{
							*self.up_down_distances.get_mut(origin,target) = Some((origin_to_root,target_to_root));
						}
					}
				}
			}
			//Now fix all little segments that do not reach the root.
			dbg!(root,"segments");
			for origin in 0..n
			{
				//Start towards root annotating those that require only upwards.
				if let Some(_origin_to_root) = distance_to_root[origin]
				{
					let mut upwards=Vec::with_capacity(n);
					upwards.push((origin,0));
					let mut read_index = 0;
					while read_index < upwards.len()
					{
						let (current,distance) = upwards[read_index];
						if let Some(current_to_root) = distance_to_root[current]
						{
							read_index+=1;
							*self.up_down_distances.get_mut(origin,current)=Some((distance,0));
							*self.up_down_distances.get_mut(current,origin)=Some((0,distance));
							for NeighbourRouterIteratorItem{neighbour_router:neighbour,..} in topology.neighbour_router_iter(current)
							{
								if let Some(neighbour_to_root) = distance_to_root[neighbour]
								{
									if neighbour_to_root +1 == current_to_root
									{
										upwards.push((neighbour,distance+1));
									}
								}
							}
						}
					}
				}
			}
			dbg!(root,"finished table");
		}
		if n!=self.up_down_distances.get_columns()
		{
			panic!("ExplicitUpDown has not being properly initialized");
		}
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}

impl ExplicitUpDown
{
	pub fn new(arg: RoutingBuilderArgument) -> ExplicitUpDown
	{
		//let mut order=None;
		//let mut servers_per_router=None;
		let mut root = None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="UpDownStar"
			{
				panic!("A UpDownStar must be created from a `UpDownStar` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"root" => match value
					{
						&ConfigurationValue::Number(f) => root=Some(f as usize),
						_ => panic!("bad value for root"),
					},
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in ExplicitUpDown",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ExplicitUpDown from a non-Object");
		}
		//let order=order.expect("There were no order");
		ExplicitUpDown{
			root,
			up_down_distances: Matrix::constant(None,0,0),
		}
	}
}

///Set the virtual channels to use in each hop.
///Sometimes the same can be achieved by the router policy `Hops`.
#[derive(Debug)]
pub struct ChannelsPerHop
{
	///The base routing to use.
	routing: Box<dyn Routing>,
	///channels[k] is the list of available VCs to use in the k-th hop.
	///This includes the last hop towards the server.
	channels: Vec<Vec<usize>>,
}

impl Routing for ChannelsPerHop
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//println!("{}",topology.diameter());
		let vcs = &self.channels[routing_info.hops];
		let candidates = self.routing.next(routing_info,topology,current_router,target_server,num_virtual_channels,rng);
		candidates.into_iter().filter(|c|vcs.contains(&c.virtual_channel)).collect()
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		self.routing.initialize_routing_info(routing_info,topology,current_router,target_server,rng);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		self.routing.update_routing_info(routing_info,topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.routing.initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng:&RefCell<StdRng>)
	{
		self.routing.performed_request(requested,routing_info,topology,current_router,target_server,num_virtual_channels,rng);
	}
	fn statistics(&self, cycle:usize) -> Option<ConfigurationValue>
	{
		self.routing.statistics(cycle)
	}
	fn reset_statistics(&mut self, next_cycle:usize)
	{
		self.routing.reset_statistics(next_cycle)
	}
}

impl ChannelsPerHop
{
	pub fn new(arg: RoutingBuilderArgument) -> ChannelsPerHop
	{
		let mut routing =None;
		let mut channels =None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="ChannelsPerHop"
			{
				panic!("A ChannelsPerHop must be created from a `ChannelsPerHop` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"routing" => routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg})),
					"channels" => match value
					{
						&ConfigurationValue::Array(ref hoplist) => channels=Some(hoplist.iter().map(|v|match v{
							&ConfigurationValue::Array(ref vcs) => vcs.iter().map(|v|match v{
								&ConfigurationValue::Number(f) => f as usize,
								_ => panic!("bad value in channels"),
							}).collect(),
							_ => panic!("bad value in channels"),
						}).collect()),
						_ => panic!("bad value for channels"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in ChannelsPerHop",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ChannelsPerHop from a non-Object");
		}
		let routing=routing.expect("There were no routing");
		let channels=channels.expect("There were no channels");
		ChannelsPerHop{
			routing,
			channels,
		}
	}
}

///Set the virtual channels to use in each hop for each link class.
///See also the simpler transformation by ChannelsPerHop.
#[derive(Debug)]
pub struct ChannelsPerHopPerLinkClass
{
	///The base routing to use.
	routing: Box<dyn Routing>,
	///channels[class][k] is the list of available VCs to use in the k-th hop given in links of the given `class`.
	channels: Vec<Vec<Vec<usize>>>,
}

impl Routing for ChannelsPerHopPerLinkClass
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//println!("{}",topology.diameter());
		let candidates = self.routing.next(&routing_info.meta.as_ref().unwrap()[0].borrow(),topology,current_router,target_server,num_virtual_channels,rng);
		let hops = &routing_info.selections.as_ref().unwrap();
		candidates.into_iter().filter(|c|{
			let (_next_location,link_class)=topology.neighbour(current_router,c.port);
			let h = hops[link_class] as usize;
			//println!("h={} link_class={} channels={:?}",h,link_class,self.channels[link_class]);
			if self.channels[link_class].len()<=h
			{
				panic!("Already given {} hops by link class {}",h,link_class);
			}
			//self.channels[link_class].len()>h && self.channels[link_class][h].contains(&c.virtual_channel)
			self.channels[link_class][h].contains(&c.virtual_channel)
		}).collect()
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let mut info = routing_info.borrow_mut();
		info.meta=Some(vec![ RefCell::new(RoutingInfo::new())]);
		info.selections = Some(vec![0;self.channels.len()]);
		self.routing.initialize_routing_info(&info.meta.as_ref().unwrap()[0],topology,current_router,target_server,rng);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let (_previous_location,link_class)=topology.neighbour(current_router,current_port);
		let mut info = routing_info.borrow_mut();
		if let Some(ref mut hops)=info.selections
		{
			if hops.len() <= link_class
			{
				println!("WARNING: In ChannelsPerHopPerLinkClass, {} classes where not enough, hop through class {}",hops.len(),link_class);
				hops.resize(link_class+1,0);
			}
			hops[link_class] += 1;
		}
		let subinfo = &info.meta.as_ref().unwrap()[0];
		subinfo.borrow_mut().hops+=1;
		self.routing.update_routing_info(subinfo,topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.routing.initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng:&RefCell<StdRng>)
	{
		self.routing.performed_request(requested,&routing_info.borrow().meta.as_ref().unwrap()[0],topology,current_router,target_server,num_virtual_channels,rng);
	}
	fn statistics(&self, cycle:usize) -> Option<ConfigurationValue>
	{
		self.routing.statistics(cycle)
	}
	fn reset_statistics(&mut self, next_cycle:usize)
	{
		self.routing.reset_statistics(next_cycle)
	}
}

impl ChannelsPerHopPerLinkClass
{
	pub fn new(arg: RoutingBuilderArgument) -> ChannelsPerHopPerLinkClass
	{
		let mut routing =None;
		let mut channels =None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="ChannelsPerHopPerLinkClass"
			{
				panic!("A ChannelsPerHopPerLinkClass must be created from a `ChannelsPerHopPerLinkClass` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"routing" => routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg})),
					"channels" => match value
					{
						&ConfigurationValue::Array(ref classlist) => channels=Some(classlist.iter().map(|v|match v{
							&ConfigurationValue::Array(ref hoplist) => hoplist.iter().map(|v|match v{
								&ConfigurationValue::Array(ref vcs) => vcs.iter().map(|v|match v{
									&ConfigurationValue::Number(f) => f as usize,
									_ => panic!("bad value in channels"),
								}).collect(),
								_ => panic!("bad value in channels"),
							}).collect(),
							_ => panic!("bad value in channels"),
						}).collect()),
						_ => panic!("bad value for channels"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in ChannelsPerHopPerLinkClass",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ChannelsPerHopPerLinkClass from a non-Object");
		}
		let routing=routing.expect("There were no routing");
		let channels=channels.expect("There were no channels");
		ChannelsPerHopPerLinkClass{
			routing,
			channels,
		}
	}
}

#[derive(Debug)]
pub struct AscendantChannelsWithLinkClass
{
	///The base routing to use.
	routing: Box<dyn Routing>,
	bases: Vec<usize>,
}

impl Routing for AscendantChannelsWithLinkClass
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//println!("{}",topology.diameter());
		let candidates = self.routing.next(&routing_info.meta.as_ref().unwrap()[0].borrow(),topology,current_router,target_server,num_virtual_channels,rng);
		let hops_since = &routing_info.selections.as_ref().unwrap();
		candidates.into_iter().filter(|c|{
			let (_next_location,link_class)=topology.neighbour(current_router,c.port);
			if link_class>= self.bases.len() { return true; }
			//let h = hops_since[link_class] as usize;
			let vc = (link_class..self.bases.len()).rev().fold(0, |x,class| x*self.bases[class]+(hops_since[class] as usize) );
			//if link_class==0 && vc!=hops_since[1] as usize{ println!("hops_since={:?} link_class={} vc={}",hops_since,link_class,vc); }
			c.virtual_channel == vc
		}).collect()
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let mut info = routing_info.borrow_mut();
		info.meta=Some(vec![ RefCell::new(RoutingInfo::new())]);
		info.selections = Some(vec![0;self.bases.len()]);
		self.routing.initialize_routing_info(&info.meta.as_ref().unwrap()[0],topology,current_router,target_server,rng);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let (_previous_location,link_class)=topology.neighbour(current_router,current_port);
		let mut info = routing_info.borrow_mut();
		if let Some(ref mut hops_since)=info.selections
		{
			if hops_since.len() <= link_class
			{
				println!("WARNING: In AscendantChannelsWithLinkClass, {} classes where not enough, hop through class {}",hops_since.len(),link_class);
				hops_since.resize(link_class+1,0);
			}
			hops_since[link_class] += 1;
			for x in hops_since[0..link_class].iter_mut()
			{
				*x=0;
			}
		}
		let subinfo = &info.meta.as_ref().unwrap()[0];
		subinfo.borrow_mut().hops+=1;
		self.routing.update_routing_info(subinfo,topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.routing.initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, rng:&RefCell<StdRng>)
	{
		self.routing.performed_request(requested,&routing_info.borrow().meta.as_ref().unwrap()[0],topology,current_router,target_server,num_virtual_channels,rng);
	}
	fn statistics(&self, cycle:usize) -> Option<ConfigurationValue>
	{
		self.routing.statistics(cycle)
	}
	fn reset_statistics(&mut self, next_cycle:usize)
	{
		self.routing.reset_statistics(next_cycle)
	}
}

impl AscendantChannelsWithLinkClass
{
	pub fn new(arg: RoutingBuilderArgument) -> AscendantChannelsWithLinkClass
	{
		let mut routing =None;
		let mut bases =None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="AscendantChannelsWithLinkClass"
			{
				panic!("A AscendantChannelsWithLinkClass must be created from a `AscendantChannelsWithLinkClass` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"routing" => routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg})),
					"bases" => match value
					{
						&ConfigurationValue::Array(ref classlist) => bases=Some(classlist.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in bases"),
						}).collect()),
						_ => panic!("bad value in bases"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in AscendantChannelsWithLinkClass",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a AscendantChannelsWithLinkClass from a non-Object");
		}
		let routing=routing.expect("There were no routing");
		let bases=bases.expect("There were no bases");
		AscendantChannelsWithLinkClass{
			routing,
			bases,
		}
	}
}

///Just remap the virtual channels.
#[derive(Debug)]
pub struct ChannelMap
{
	///The base routing to use.
	routing: Box<dyn Routing>,
	map: Vec<Vec<usize>>,
}

impl Routing for ChannelMap
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, _num_virtual_channels:usize, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//println!("{}",topology.diameter());
		//let vcs = &self.channels[routing_info.hops];
		let candidates = self.routing.next(routing_info,topology,current_router,target_server,self.map.len(),rng);
		//candidates.into_iter().filter(|c|vcs.contains(&c.virtual_channel)).collect()
		let mut r=Vec::with_capacity(candidates.len());
		for can in candidates.into_iter()
		{
			for vc in self.map[can.virtual_channel].iter()
			{
				let mut new = can.clone();
				new.virtual_channel = *vc;
				r.push(new);
			}
		}
		r
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		self.routing.initialize_routing_info(routing_info,topology,current_router,target_server,rng);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		self.routing.update_routing_info(routing_info,topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.routing.initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, _num_virtual_channels:usize, rng:&RefCell<StdRng>)
	{
		self.routing.performed_request(requested,routing_info,topology,current_router,target_server,self.map.len(),rng);
	}
	fn statistics(&self, cycle:usize) -> Option<ConfigurationValue>
	{
		self.routing.statistics(cycle)
	}
	fn reset_statistics(&mut self, next_cycle:usize)
	{
		self.routing.reset_statistics(next_cycle)
	}
}

impl ChannelMap
{
	pub fn new(arg: RoutingBuilderArgument) -> ChannelMap
	{
		let mut routing =None;
		let mut map =None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="ChannelMap"
			{
				panic!("A ChannelMap must be created from a `ChannelMap` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"routing" => routing=Some(new_routing(RoutingBuilderArgument{cv:value,..arg})),
					"map" => match value
					{
						&ConfigurationValue::Array(ref hoplist) => map=Some(hoplist.iter().map(|v|match v{
							&ConfigurationValue::Array(ref vcs) => vcs.iter().map(|v|match v{
								&ConfigurationValue::Number(f) => f as usize,
								_ => panic!("bad value in map"),
							}).collect(),
							_ => panic!("bad value in map"),
						}).collect()),
						_ => panic!("bad value for map"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in ChannelMap",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ChannelMap from a non-Object");
		}
		let routing=routing.expect("There were no routing");
		let map=map.expect("There were no map");
		ChannelMap{
			routing,
			map,
		}
	}
}


///Encapsulation of SourceRouting, to allow storing several paths in the packet. And then, have adaptiveness for the first hop.
#[derive(Debug)]
pub struct SourceAdaptiveRouting
{
	///The base routing
	pub routing: Box<dyn InstantiableSourceRouting>,
	///Maximum amount of paths to store
	pub amount: usize,
}

impl Routing for SourceAdaptiveRouting
{
	fn next(&self, routing_info:&RoutingInfo, topology:&dyn Topology, current_router:usize, target_server:usize, num_virtual_channels:usize, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let distance=topology.distance(current_router,target_router);
		if distance==0
		{
			for i in 0..topology.ports(current_router)
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::ServerPort(server),_link_class)=topology.neighbour(current_router,i)
				{
					if server==target_server
					{
						//return (0..num_virtual_channels).map(|vc|(i,vc)).collect();
						return (0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)).collect();
					}
				}
			}
			unreachable!();
		}
		let source_router = routing_info.visited_routers.as_ref().unwrap()[0];
		let num_ports=topology.ports(current_router);
		let mut r=Vec::with_capacity(num_ports*num_virtual_channels);
		let selections = routing_info.selections.as_ref().unwrap().clone();
		for path_index in selections
		{
			let path = &self.routing.get_paths(source_router,target_router)[<usize>::try_from(path_index).unwrap()];
			let next_router = path[routing_info.hops+1];
			let length = path.len() - 1;//substract source router
			let remain = length - routing_info.hops;
			for i in 0..num_ports
			{
				//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
				if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
				{
					//if distance-1==topology.distance(router_index,target_router)
					if router_index==next_router
					{
						//r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
						r.extend((0..num_virtual_channels).map(|vc|{
							let mut egress = CandidateEgress::new(i,vc);
							egress.estimated_remaining_hops = Some(remain);
							egress
						}));
					}
				}
			}
		}
		//println!("From router {} to router {} distance={} cand={}",current_router,target_router,distance,r.len());
		r
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		routing_info.borrow_mut().visited_routers=Some(vec![current_router]);
		if current_router!=target_router
		{
			let path_collection = self.routing.get_paths(current_router,target_router);
			//println!("path_collection.len={} for source={} target={}\n",path_collection.len(),current_router,target_router);
			if path_collection.is_empty()
			{
				panic!("No path found from router {} to router {}",current_router,target_router);
			}
			let mut selected_indices : Vec<i32> = (0i32..<i32>::try_from(path_collection.len()).unwrap()).collect();
			if selected_indices.len()>self.amount
			{
				rng.borrow_mut().shuffle(&mut selected_indices);
				selected_indices.resize_with(self.amount,||unreachable!());
			}
			routing_info.borrow_mut().selections=Some(selected_indices);
		}
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, _current_port:usize, target_server:usize, _rng: &RefCell<StdRng>)
	{
		let (target_location,_link_class)=topology.server_neighbour(target_server);
		let target_router=match target_location
		{
			Location::RouterPort{router_index,router_port:_} =>router_index,
			_ => panic!("The server is not attached to a router"),
		};
		let mut ri=routing_info.borrow_mut();
		let hops = ri.hops;
		if let Some(ref mut visited)=ri.visited_routers
		{
			let source_router = visited[0];
			visited.push(current_router);
			//Now discard all selections toward other routers.
			let paths = &self.routing.get_paths(source_router,target_router);
			if let Some(ref mut selections)=ri.selections
			{
				selections.retain(|path_index|{
					let path = &paths[<usize>::try_from(*path_index).unwrap()];
					path[hops]==current_router
				});
				if selections.is_empty()
				{
					panic!("No selections remaining.");
				}
			}
		}
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.routing.initialize(topology,rng);
	}
	fn performed_request(&self, _requested:&CandidateEgress, _routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
	}
	fn statistics(&self, _cycle:usize) -> Option<ConfigurationValue>
	{
		return None;
	}
	fn reset_statistics(&mut self, _next_cycle:usize)
	{
	}
}







