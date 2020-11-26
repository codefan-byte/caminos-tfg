
use crate::config_parser::ConfigurationValue;
use crate::topology::cartesian::{DOR,O1TURN,ValiantDOR,OmniDimensionalDeroute};
use crate::topology::{Topology,Location};
use crate::matrix::Matrix;
use std::cell::RefCell;
use ::rand::{StdRng,Rng};
use quantifiable_derive::Quantifiable;//the derive macro
use crate::Plugs;
use std::fmt::Debug;

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
	pub port: usize,
	pub virtual_channel: usize,
	pub label: i32,
	pub estimated_remaining_hops: Option<usize>,

	///The routing must set this as false.
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
#[non_exhaustive]
#[derive(Debug)]
pub struct RoutingBuilderArgument<'a>
{
	///A ConfigurationValue::Object defining the routing.
	pub cv: &'a ConfigurationValue,
	///The user defined plugs. In case the routing needs to create elements.
	pub plugs: &'a Plugs,
}

///Build a new routing.
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
					//r.extend((0..num_virtual_channels).map(|vc|(i,vc)));
					r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
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
				//self.second.next(&routing_info.meta.unwrap()[1].borrow(),topology,current_router,target_server,num_virtual_channels,rng)
				self.second.next(&meta[1].borrow(),topology,current_router,target_server,num_virtual_channels,rng)
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
				//self.first.next(&routing_info.meta.unwrap()[0].borrow(),topology,current_router,middle_server,num_virtual_channels,rng)
				self.first.next(&meta[0].borrow(),topology,current_router,middle_server,num_virtual_channels,rng)
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
		let middle=rng.borrow_mut().gen_range(0,n);
		let mut bri=routing_info.borrow_mut();
		bri.meta=Some(vec![RefCell::new(RoutingInfo::new()),RefCell::new(RoutingInfo::new())]);
		if middle==current_router || middle==target_router
		{
			self.second.initialize_routing_info(&bri.meta.as_ref().unwrap()[1],topology,current_router,target_server,rng);
		}
		else
		{
			bri.selections=Some(vec![middle as i32]);
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
					let meta=bri.meta.as_mut().unwrap();
					meta[0].borrow_mut().hops+=1;
					self.first.update_routing_info(&meta[0],topology,current_router,current_port,target_server,rng);
				}
			}
		};
	}
	fn initialize(&mut self, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
		//TODO: recurse over routings
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
		Valiant{
			first,
			second,
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
		for i in 0..num_ports
		{
			//println!("{} -> {:?}",i,topology.neighbour(current_router,i));
			if let (Location::RouterPort{router_index,router_port:_},_link_class)=topology.neighbour(current_router,i)
			{
				//if distance-1==topology.distance(router_index,target_router)
				if router_index==next_router
				{
					//r.extend((0..num_virtual_channels).map(|vc|(i,vc)));
					r.extend((0..num_virtual_channels).map(|vc|CandidateEgress::new(i,vc)));
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
}

pub fn new_sum_routing_policy(cv: &ConfigurationValue) -> SumRoutingPolicy
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=cv
	{
		match cv_name.as_ref()
		{
			"Random" => SumRoutingPolicy::Random,
			"TryBoth" => SumRoutingPolicy::TryBoth,
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
	first_routing:Box<dyn Routing>,
	second_routing:Box<dyn Routing>,
	first_allowed_virtual_channels: Vec<usize>,
	second_allowed_virtual_channels: Vec<usize>,
}

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
				if s.len()==2
				{
					let avc0=&self.first_allowed_virtual_channels;
					let r0=self.first_routing.next(&meta[0].borrow(),topology,current_router,target_server,avc0.len(),rng).into_iter().map( |candidate| CandidateEgress{virtual_channel:avc0[candidate.virtual_channel],annotation:Some(RoutingAnnotation{values:vec![0],meta:vec![candidate.annotation]}),..candidate} );
					let avc1=&self.first_allowed_virtual_channels;
					let r1=self.second_routing.next(&meta[0].borrow(),topology,current_router,target_server,avc1.len(),rng).into_iter().map( |candidate| CandidateEgress{virtual_channel:avc1[candidate.virtual_channel],annotation:Some(RoutingAnnotation{values:vec![0],meta:vec![candidate.annotation]}),..candidate} );
					r0.chain(r1).collect()
				}
				else
				{
					let routing=if s[0]==0 { &self.first_routing } else { &self.second_routing };
					let allowed_virtual_channels=if s[0]==0 { &self.first_allowed_virtual_channels } else { &self.second_allowed_virtual_channels };
					let r=routing.next(&meta[0].borrow(),topology,current_router,target_server,allowed_virtual_channels.len(),rng);
					//r.into_iter().map( |(x,c)| (x,allowed_virtual_channels[c]) ).collect()
					r.into_iter()
					//.map( |CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}| CandidateEgress{port,virtual_channel:allowed_virtual_channels[virtual_channel],label,estimated_remaining_hops} ).collect()
					.map( |candidate| CandidateEgress{virtual_channel:allowed_virtual_channels[candidate.virtual_channel],..candidate} ).collect()
				}
			}
		}
	}
	fn initialize_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let all = match self.policy
		{
			SumRoutingPolicy::Random => vec![rng.borrow_mut().gen_range(0,2)],
			SumRoutingPolicy::TryBoth => vec![0,1],
		};
		let mut bri=routing_info.borrow_mut();
		//bri.meta=Some(vec![RefCell::new(RoutingInfo::new()),RefCell::new(RoutingInfo::new())]);
		bri.meta=Some(vec![RefCell::new(RoutingInfo::new())]);
		for &s in all.iter()
		{
			let routing=if s==0 { &self.first_routing } else { &self.second_routing };
			routing.initialize_routing_info(&bri.meta.as_ref().unwrap()[0],topology,current_router,target_server,rng)
		}
		bri.selections=Some(all);
	}
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, topology:&dyn Topology, current_router:usize, current_port:usize, target_server:usize, rng: &RefCell<StdRng>)
	{
		let mut bri=routing_info.borrow_mut();
		let s=match bri.selections
		{
			None => unreachable!(),
			Some(ref t) => t[0],
		};
		let routing=if s==0 { &self.first_routing } else { &self.second_routing };
		let meta=bri.meta.as_mut().unwrap();
		meta[0].borrow_mut().hops+=1;
		routing.update_routing_info(&meta[0],topology,current_router,current_port,target_server,rng);
	}
	fn initialize(&mut self, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.first_routing.initialize(topology,rng);
		self.second_routing.initialize(topology,rng);
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
		let mut bri=routing_info.borrow_mut();
		if let SumRoutingPolicy::TryBoth=self.policy
		{
			let &CandidateEgress{ref annotation,..} = requested;
			if let Some(annotation) = annotation.as_ref()
			{
				let s = annotation.values[0];
				bri.selections=Some(vec![s]);
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
			first_routing,
			second_routing,
			first_allowed_virtual_channels,
			second_allowed_virtual_channels,
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
				//if distance-1==topology.distance(router_index,target_router)
				if distance>*self.distance_matrix.get(router_index,target_router)
				{
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
	fn update_routing_info(&self, routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _current_port:usize, _target_server:usize, _rng: &RefCell<StdRng>)
	{
		routing_info.borrow_mut().selections=None;
	}
	fn initialize(&mut self, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
	}
	fn performed_request(&self, requested:&CandidateEgress, routing_info:&RefCell<RoutingInfo>, _topology:&dyn Topology, _current_router:usize, _target_server:usize, _num_virtual_channels:usize, _rng:&RefCell<StdRng>)
	{
		let &CandidateEgress{port,virtual_channel,ref annotation,..} = requested;
		if let Some(annotation) = annotation.as_ref()
		{
			let label = annotation.values[0];
			routing_info.borrow_mut().selections=Some(vec![port as i32, virtual_channel as i32, label]);
			//TODO: recurse over routing
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

