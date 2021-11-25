
use crate::config_parser::ConfigurationValue;
use crate::routing::CandidateEgress;
use crate::router::Router;
use crate::topology::{Topology,Location};
use crate::Plugs;

use std::cell::{RefCell};
use std::fmt::Debug;
use std::convert::TryInto;

use ::rand::{Rng,StdRng};

///Extra information to be used by the policies of virtual channels.
#[derive(Debug)]
pub struct RequestInfo<'a>
{
	///target_router_index: The index of the router to which the destination server is attached.
	pub target_router_index: usize,
	///entry_port: The port for which the packet has entered into the current router.
	pub entry_port: usize,
	///entry_virtual_channel: The virtual_channel the packet used when it entered into the current router.
	pub entry_virtual_channel: usize,
	///performed_hops: the amount of hops already made by the packet.
	pub performed_hops: usize,
	///server_ports: a list of which ports from the current router go to server.
	pub server_ports: Option<&'a Vec<usize>>,
	///port_average_neighbour_queue_length: for each port the average queue length in the queues of the port in the neighbour router.
	pub port_average_neighbour_queue_length: Option<&'a Vec<f32>>,
	///port_last_transmission: a timestamp for each port of the last time that it was used.
	pub port_last_transmission: Option<&'a Vec<usize>>,
	///Number of phits currently in the output space of the current router at the indexed port.
	pub port_occupied_output_space: Option<&'a Vec<usize>>,
	///Number of available phits in the output space of the current router at the indexed port.
	pub port_available_output_space: Option<&'a Vec<usize>>,
	///Number of phits currently in the output space allocated to a virtual channel. Index by `[port_index][virtual_channel]`.
	pub virtual_channel_occupied_output_space: Option<&'a Vec<Vec<usize>>>,
	///Number of available phits in the output space allocated to a virtual channel. Index by `[port_index][virtual_channel]`.
	pub virtual_channel_available_output_space: Option<&'a Vec<Vec<usize>>>,
	///Number of cycles at the front of input space,
	pub time_at_front: Option<usize>,
	///current_cycle: The current cycle of the simulation.
	pub current_cycle: usize,
}

///How virtual channels are selected for a packet
///They provide the function filter(Vec<CandidateEgress>) -> Vec<CandidateEgress>
///It needs:
///	rng, self.virtual_ports(credits and length), phit.packet.routing_info.borrow().hops, server_ports,
/// topology.{distance,neighbour}, port_average_neighbour_queue_length, port_last_transmission
///We could also provide functions to declare which aspects must be computed. Thus allowing to both share when necessary and to not computing ti when unnecessary.
pub trait VirtualChannelPolicy : Debug
{
	///Apply the policy over a list of candidates and return the candidates that fulfil the policy requirements.
	///candidates: the list to be filtered.
	///router: the router in which the decision is being made.
	///topology: The network topology.
	///rng: the global random number generator.
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, info: &RequestInfo, topology:&dyn Topology, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>;
	fn need_server_ports(&self)->bool;
	fn need_port_average_queue_length(&self)->bool;
	fn need_port_last_transmission(&self)->bool;
}

#[derive(Debug)]
pub struct VCPolicyBuilderArgument<'a>
{
	///A ConfigurationValue::Object defining the policy.
	pub cv: &'a ConfigurationValue,
	///The user defined plugs. In case the policy needs to create elements.
	pub plugs: &'a Plugs,
}

//pub fn new_virtual_channel_policy(cv: &ConfigurationValue, plugs:&Plugs) -> Box<dyn VirtualChannelPolicy>
pub fn new_virtual_channel_policy(arg:VCPolicyBuilderArgument) -> Box<dyn VirtualChannelPolicy>
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=arg.cv
	{
		match arg.plugs.policies.get(cv_name)
		{
			Some(builder) => return builder(arg),
			_ => (),
		};
		match cv_name.as_ref()
		{
			"Identity" => Box::new(Identity::new(arg)),
			"Random" => Box::new(Random::new(arg)),
			"Shortest" => Box::new(Shortest::new(arg)),
			"Hops" => Box::new(Hops::new(arg)),
			"EnforceFlowControl" => Box::new(EnforceFlowControl::new(arg)),
			"WideHops" => Box::new(WideHops::new(arg)),
			"LowestSinghWeight" => Box::new(LowestSinghWeight::new(arg)),
			"LowestLabel" => Box::new(LowestLabel::new(arg)),
			"LabelSaturate" => Box::new(LabelSaturate::new(arg)),
			"LabelTransform" => Box::new(LabelTransform::new(arg)),
			"OccupancyFunction" => Box::new(OccupancyFunction::new(arg)),
			"NegateLabel" => Box::new(NegateLabel::new(arg)),
			"VecLabel" => Box::new(VecLabel::new(arg)),
			"MapLabel" => Box::new(MapLabel::new(arg)),
			"ShiftEntryVC" => Box::new(MapLabel::new(arg)),
			_ => panic!("Unknown traffic {}",cv_name),
		}
	}
	else
	{
		panic!("Trying to create a traffic from a non-Object");
	}
}

///Does not do anything. Just a placeholder for some operations.
#[derive(Debug)]
pub struct Identity{}

impl VirtualChannelPolicy for Identity
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		candidates
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}
}

impl Identity
{
	pub fn new(_arg:VCPolicyBuilderArgument) -> Identity
	{
		Identity{}
	}
}


///Request a port+virtual channel at random from all available.
#[derive(Debug)]
pub struct Random{}

impl VirtualChannelPolicy for Random
{
	//fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _target_router_index:usize, _entry_port:usize, _entry_virtual_channel:usize, _performed_hops:usize, _server_ports:&Option<Vec<usize>>, _port_average_neighbour_queue_length:&Option<Vec<f32>>, _port_last_transmission:&Option<Vec<usize>>, _port_occuped_output_space:&Option<Vec<usize>>, _port_available_output_space:&Option<Vec<usize>>, _current_cycle:usize, _topology:&dyn Topology, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		vec![candidates[rng.borrow_mut().gen_range(0,candidates.len())].clone()]
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}
}

impl Random
{
	pub fn new(_arg:VCPolicyBuilderArgument) -> Random
	{
		//let mut servers=None;
		//let mut load=None;
		//let mut pattern=None;
		//let mut message_size=None;
		//if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=cv
		//{
		//	if cv_name!="Random"
		//	{
		//		panic!("A Random must be created from a `Random` object not `{}`",cv_name);
		//	}
		//	for &(ref name,ref value) in cv_pairs
		//	{
		//		//match name.as_ref()
		//		match name.as_ref()
		//		{
		//			//"pattern" => pattern=Some(new_pattern(value)),
		//			//"servers" => match value
		//			//{
		//			//	&ConfigurationValue::Number(f) => servers=Some(f as usize),
		//			//	_ => panic!("bad value for servers"),
		//			//}
		//			//"load" => match value
		//			//{
		//			//	&ConfigurationValue::Number(f) => load=Some(f as f32),
		//			//	_ => panic!("bad value for load ({:?})",value),
		//			//}
		//			//"message_size" => match value
		//			//{
		//			//	&ConfigurationValue::Number(f) => message_size=Some(f as usize),
		//			//	_ => panic!("bad value for message_size"),
		//			//}
		//			_ => panic!("Nothing to do with field {} in Random",name),
		//		}
		//	}
		//}
		//else
		//{
		//	panic!("Trying to create a Random from a non-Object");
		//}
		//let servers=servers.expect("There were no servers");
		//let message_size=message_size.expect("There were no message_size");
		//let load=load.expect("There were no load");
		//let mut pattern=pattern.expect("There were no pattern");
		Random{}
	}
}

///Request the port+virtual channel with more credits. Does not solve ties, so it needs to be followed by Random or something.
#[derive(Debug)]
pub struct Shortest{}

impl VirtualChannelPolicy for Shortest
{
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let mut best=vec![];
		let mut best_credits=0;
		//for i in 1..vps.len()
		for i in 0..candidates.len()
		{
			let CandidateEgress{port:p,virtual_channel:vc,..}=candidates[i];
			//let next_credits=router.virtual_ports[p][vc].neighbour_credits;
			//let next_credits=router.get_virtual_port(p,vc).expect("This router does not have virtual ports (and not credits therefore)").neighbour_credits;
			let next_credits=router.get_status_at_emisor(p).expect("This router does not have transmission status").known_available_space_for_virtual_channel(vc).expect("remote available space is not known");
			if next_credits>best_credits
			{
				best_credits=next_credits;
				//best=vec![CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops}];
				best=vec![candidates[i].clone()];
			}
			else if next_credits==best_credits
			{
				//best.push(CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops});
				best.push(candidates[i].clone());
			}
		}
		best
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl Shortest
{
	pub fn new(_arg:VCPolicyBuilderArgument) -> Shortest
	{
		Shortest{}
	}
}


///Select virtual channel=packet.hops.
#[derive(Debug)]
pub struct Hops{}

impl VirtualChannelPolicy for Hops
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let server_ports=info.server_ports.expect("server_ports have not been computed for policy Hops");
		let filtered=candidates.into_iter().filter(|&CandidateEgress{port,virtual_channel,label:_label,estimated_remaining_hops:_,..}|virtual_channel==info.performed_hops|| server_ports.contains(&port)).collect::<Vec<_>>();
		//let filtered=candidates.iter().filter_map(|e|if e.1==performed_hops{Some(*e)}else {None}).collect::<Vec<_>>();
		//if filtered.len()==0
		//{
		//	//panic!("There is no route from router {} to server {} increasing on virtual channels",self.router_index,phit.packet.message.destination);
		//	continue;
		//}
		//filtered[simulation.rng.borrow_mut().gen_range(0,filtered.len())]
		filtered
	}

	fn need_server_ports(&self)->bool
	{
		true
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl Hops
{
	pub fn new(_arg:VCPolicyBuilderArgument) -> Hops
	{
		Hops{}
	}
}

///
#[derive(Debug)]
pub struct EnforceFlowControl{}

impl VirtualChannelPolicy for EnforceFlowControl
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let filtered=candidates.into_iter().filter(|candidate|candidate.router_allows.unwrap_or(true)).collect::<Vec<_>>();
		filtered
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl EnforceFlowControl
{
	pub fn new(_arg:VCPolicyBuilderArgument) -> EnforceFlowControl
	{
		EnforceFlowControl{}
	}
}


///Select virtual channel in (width*packet.hops..width*(packet.hops+1)).
#[derive(Debug)]
pub struct WideHops{
	width:usize,
}

impl VirtualChannelPolicy for WideHops
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let server_ports=info.server_ports.expect("server_ports have not been computed for policy WideHops");
		let lower_limit = self.width*info.performed_hops;
		let upper_limit = self.width*(info.performed_hops+1);
		let filtered=candidates.into_iter().filter(
			|&CandidateEgress{port,virtual_channel,label:_,estimated_remaining_hops:_,..}| (lower_limit<=virtual_channel && virtual_channel<upper_limit) || server_ports.contains(&port)
		).collect::<Vec<_>>();
		//let filtered=candidates.iter().filter_map(|e|if e.1==info.performed_hops{Some(*e)}else {None}).collect::<Vec<_>>();
		//if filtered.len()==0
		//{
		//	//panic!("There is no route from router {} to server {} increasing on virtual channels",self.router_index,phit.packet.message.destination);
		//	continue;
		//}
		//filtered[simulation.rng.borrow_mut().gen_range(0,filtered.len())]
		filtered
	}

	fn need_server_ports(&self)->bool
	{
		true
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl WideHops
{
	pub fn new(arg:VCPolicyBuilderArgument) -> WideHops
	{
		let mut width=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="WideHops"
			{
				panic!("A WideHops must be created from a `WideHops` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
					"width" => match value
					{
						&ConfigurationValue::Number(f) => width=Some(f as usize),
						_ => panic!("bad value for width"),
					}
					_ => panic!("Nothing to do with field {} in WideHops",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a WideHops from a non-Object");
		}
		let width=width.expect("There were no width");
		WideHops{
			width
		}
	}
}

///Select the lowest value of the product of the queue length (that is, consumed credits) times the estimated hop count (usually 1 plus the distance from next router to target router)
///This was initially proposed for the UGAL routing.
///parameters=(extra_congestion,extra_distance,aggregate_buffers), which are added in the formula to allow tuning. Firth two default to 0.
///aggregate_buffers indicates to use all buffers instead of just the selected one.
#[derive(Debug)]
pub struct LowestSinghWeight
{
	///constant added to the occupied space
	extra_congestion: usize,
	///constant added to the distance to target
	extra_distance: usize,
	///Whether we consider all the space in each port (when true) or we segregate by virtual channels (when false).
	///defaults to false
	///Previously called aggregate_buffers
	aggregate: bool,
	///Whether to use internal output space in the calculations instead of the counters relative to the next router.
	///defaults to false
	use_internal_space: bool,
	///Whether to add the neighbour space.
	///Defaults to true.
	use_neighbour_space: bool,
	///Try `estimated_remaining_hops` before calling distance
	use_estimation: bool,
}

impl VirtualChannelPolicy for LowestSinghWeight
{
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, info: &RequestInfo, topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//let port_average_neighbour_queue_length=info.port_average_neighbour_queue_length.expect("port_average_neighbour_queue_length have not been computed for policy LowestSinghWeight");
		let dist=topology.distance(router.get_index().expect("we need routers with index"),info.target_router_index);
		if dist==0
		{
			//do nothing
			candidates
		}
		else
		{
			let mut best=vec![];
			//let mut best_weight=<usize>::max_value();
			let mut best_weight=<i32>::max_value();
			//let mut best_weight=::std::f32::MAX;
			//for i in 0..candidates.len()
			//for CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops} in candidates
			for candidate in candidates
			{
				let CandidateEgress{port:p,virtual_channel:vc, estimated_remaining_hops, ..} = candidate;
				//let next_consumed_credits:f32=(self.extra_congestion as f32)+if self.aggregate_buffers
				//{
				//	if self.use_internal_space
				//	{
				//		let port_occupied_output_space=info.port_occupied_output_space.expect("port_occupied_output_space have not been computed for policy LowestSinghWeight");
				//		port_occupied_output_space[p] as f32
				//	}
				//	else
				//	{
				//		port_average_neighbour_queue_length[p]
				//	}
				//}
				//else
				//{
				//	if self.use_internal_space
				//	{
				//		unimplemented!()
				//	}
				//	else
				//	{
				//		//(router.buffer_size - router.virtual_ports[p][vc].neighbour_credits) as f32
				//		let next_credits=router.get_status_at_emisor(p).expect("This router does not have transmission status").known_available_space_for_virtual_channel(vc).expect("remote available space is not known");
				//		(router.get_maximum_credits_towards(p,vc).expect("we need routers with maximum credits") - next_credits) as f32
				//	}
				//};
				let q:i32 = (self.extra_congestion as i32) + if self.use_internal_space
				{
					if self.aggregate
					{
						let port_occupied_output_space=info.port_occupied_output_space.expect("port_occupied_output_space have not been computed for policy LowestSinghWeight");
						port_occupied_output_space[p] as i32
					}
					else
					{
						let virtual_channel_occupied_output_space=info.virtual_channel_occupied_output_space.expect("virtual_channel_occupied_output_space have not been computed for LowestSinghWeight");
						virtual_channel_occupied_output_space[p][vc] as i32
					}
				}
				else {0} + if self.use_neighbour_space
				{
					if self.aggregate
					{
						//port_average_neighbour_queue_length[p]
						let status=router.get_status_at_emisor(p).expect("This router does not have transmission status");
						//FIXME: this could be different than the whole occuped space if using a DAMQ or something, although they are yet to be implemented.
						(0..status.num_virtual_channels()).map(|c|router.get_maximum_credits_towards(p,c).expect("we need routers with maximum credits") as i32 - status.known_available_space_for_virtual_channel(c).expect("remote available space is not known.") as i32).sum()
					}
					else
					{
						//port_average_neighbour_queue_length[p]
						let status=router.get_status_at_emisor(p).expect("This router does not have transmission status");
						router.get_maximum_credits_towards(p,vc).expect("we need routers with maximum credits") as i32 - status.known_available_space_for_virtual_channel(vc).expect("remote available space is not known.") as i32
					}
				}
				else {0};
				let next_router=if let (Location::RouterPort{router_index, router_port:_},_link_class)=topology.neighbour(router.get_index().expect("we need routers with index"),p)
				{
					router_index
				}
				else
				{
					panic!("We trying to go to the server when we are at distance {} greater than 0.",dist);
				};
				//let distance=self.extra_distance + 1+topology.distance(next_router,info.target_router_index);
				let distance = self.extra_distance + if let (true,Some(d)) = (self.use_estimation,estimated_remaining_hops) {
					d
				} else {
					1 + topology.distance(next_router,info.target_router_index)
				};
				let next_weight= q * (distance as i32);
				if next_weight<best_weight
				{
					best_weight=next_weight;
					//best=vec![CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops}];
					best=vec![candidate];
				}
				else if next_weight==best_weight
				{
					//best.push(CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops});
					best.push(candidate);
				}
			}
			best
		}
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		//We have removed it. Now it uses router.get_status_at_emisor
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl LowestSinghWeight
{
	pub fn new(arg:VCPolicyBuilderArgument) -> LowestSinghWeight
	{
		let mut extra_congestion=None;
		let mut extra_distance=None;
		let mut aggregate=false;
		let mut use_internal_space=false;
		let mut use_neighbour_space=true;
		let mut use_estimation=true;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="LowestSinghWeight"
			{
				panic!("A LowestSinghWeight must be created from a `LowestSinghWeight` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
 					"extra_congestion" => match value
 					{
 						&ConfigurationValue::Number(f) => extra_congestion=Some(f as usize),
 						_ => panic!("bad value for extra_congestion"),
 					}
 					"extra_distance" => match value
 					{
 						&ConfigurationValue::Number(f) => extra_distance=Some(f as usize),
 						_ => panic!("bad value for extra_distance"),
 					}
 					"aggregate" => match value
 					{
 						&ConfigurationValue::True => aggregate=true,
 						&ConfigurationValue::False => aggregate=false,
 						_ => panic!("bad value for aggregate"),
 					}
 					"aggregate_buffers" => {
						println!("WARNING: the name `aggregate_buffers` has been deprecated in favour of just `aggregate`");
						match value
						{
							&ConfigurationValue::True => aggregate=true,
							&ConfigurationValue::False => aggregate=false,
							_ => panic!("bad value for aggregate_buffers"),
						}
					},
 					"use_internal_space" => match value
 					{
 						&ConfigurationValue::True => use_internal_space=true,
 						&ConfigurationValue::False => use_internal_space=false,
 						_ => panic!("bad value for use_internal_space"),
 					}
 					"use_neighbour_space" => match value
 					{
 						&ConfigurationValue::True => use_neighbour_space=true,
 						&ConfigurationValue::False => use_neighbour_space=false,
 						_ => panic!("bad value for use_neighbour_space"),
 					}
 					"use_estimation" => match value
 					{
 						&ConfigurationValue::True => use_estimation=true,
 						&ConfigurationValue::False => use_estimation=false,
 						_ => panic!("bad value for use_estimation"),
 					}
					_ => panic!("Nothing to do with field {} in LowestSinghWeight",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a LowestSinghWeight from a non-Object");
		}
		let extra_congestion=extra_congestion.unwrap_or(0);
		let extra_distance=extra_distance.unwrap_or(0);
		LowestSinghWeight{
			extra_congestion,
			extra_distance,
			aggregate,
			use_internal_space,
			use_neighbour_space,
			use_estimation,
		}
	}
}


///Select the egresses with lowest label.
#[derive(Debug)]
pub struct LowestLabel{}

impl VirtualChannelPolicy for LowestLabel
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		let mut best=vec![];
		let mut best_label=<i32>::max_value();
		//for CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops} in candidates
		for candidate in candidates
		{
			let label = candidate.label;
			if label<best_label
			{
				best_label=label;
				//best=vec![CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops}];
				best=vec![candidate];
			}
			else if label==best_label
			{
				//best.push(CandidateEgress{port:p,virtual_channel:vc,label,estimated_remaining_hops});
				best.push(candidate);
			}
		}
		best
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl LowestLabel
{
	pub fn new(_arg:VCPolicyBuilderArgument) -> LowestLabel
	{
		LowestLabel{}
	}
}












///New label = min{old_label,value} or max{old_label,value}
///(value,bottom)
#[derive(Debug)]
pub struct LabelSaturate
{
	value:i32,
	bottom:bool,
}

impl VirtualChannelPolicy for LabelSaturate
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		if self.bottom
		{
			candidates.into_iter().map(
				//|CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}|
				|candidate|{
				let label= candidate.label;
				//label as usize <= simulation.cycle -1 - self.virtual_ports[port][virtual_channel].last_transmission
				let new_label = ::std::cmp::max(label,self.value);
				CandidateEgress{label:new_label,..candidate}
			}).collect::<Vec<_>>()
		}
		else
		{
			candidates.into_iter().map(
				|candidate|{
				let label= candidate.label;
				//label as usize <= simulation.cycle -1 - self.virtual_ports[port][virtual_channel].last_transmission
				let new_label = ::std::cmp::min(label,self.value);
				CandidateEgress{label:new_label,..candidate}
			}).collect::<Vec<_>>()
		}
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl LabelSaturate
{
	pub fn new(arg:VCPolicyBuilderArgument) -> LabelSaturate
	{
		let mut xvalue=None;
		let mut bottom=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="LabelSaturate"
			{
				panic!("A LabelSaturate must be created from a `LabelSaturate` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
					"value" => match value
					{
						&ConfigurationValue::Number(f) => xvalue=Some(f as i32),
						_ => panic!("bad value for value"),
					}
					"bottom" => match value
					{
						&ConfigurationValue::True => bottom=Some(true),
						&ConfigurationValue::False => bottom=Some(false),
						_ => panic!("bad value for bottom"),
					}
					_ => panic!("Nothing to do with field {} in LabelSaturate",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a LabelSaturate from a non-Object");
		}
		let value=xvalue.expect("There were no value");
		let bottom=bottom.expect("There were no bottom");
		LabelSaturate{
			value,
			bottom,
		}
	}
}


///New label = old_label*multplier+summand.
///(multiplier,summand,saturate_bottom,saturate_top,minimum,maximum)
#[derive(Debug)]
pub struct LabelTransform
{
	multiplier:i32,
	summand:i32,
	saturate_bottom: Option<i32>,
	saturate_top: Option<i32>,
	minimum: Option<i32>,
	maximum: Option<i32>,
}

impl VirtualChannelPolicy for LabelTransform
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		candidates.into_iter().filter_map(
			//|CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}|
			|candidate|{
			let mut new_label = candidate.label*self.multiplier + self.summand;
			//let new_label = ::std::cmp::min(::std::cmp::max(label*self.multiplier + self.summand, saturate_bottom),saturate_top);
			if let Some(value)=self.saturate_bottom
			{
				if value>new_label
				{
					new_label=value;
				}
			}
			if let Some(value)=self.saturate_top
			{
				if value<new_label
				{
					new_label=value;
				}
			}
			//if new_label>=minimum && new_label<=maximum;
			let mut good=true;
			if let Some(value)=self.minimum
			{
				if value>new_label
				{
					good=false;
				}
			}
			if let Some(value)=self.maximum
			{
				if value<new_label
				{
					good=false;
				}
			}
			if good
			{
				Some(CandidateEgress{label:new_label,..candidate})
			}
			else
			{
				None
			}
		}).collect::<Vec<_>>()
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		true
	}

}

impl LabelTransform
{
	pub fn new(arg:VCPolicyBuilderArgument) -> LabelTransform
	{
		let mut multiplier=None;
		let mut summand=None;
		let mut saturate_bottom=None;
		let mut saturate_top=None;
		let mut minimum=None;
		let mut maximum=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="LabelTransform"
			{
				panic!("A LabelTransform must be created from a `LabelTransform` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
					"multiplier" => match value
					{
						&ConfigurationValue::Number(f) => multiplier=Some(f as i32),
						_ => panic!("bad value for multiplier"),
					}
					"summand" => match value
					{
						&ConfigurationValue::Number(f) => summand=Some(f as i32),
						_ => panic!("bad value for summand"),
					}
					"saturate_bottom" => match value
					{
						&ConfigurationValue::Number(f) => saturate_bottom=Some(f as i32),
						_ => panic!("bad value for saturate_bottom"),
					}
					"saturate_top" => match value
					{
						&ConfigurationValue::Number(f) => saturate_top=Some(f as i32),
						_ => panic!("bad value for saturate_top"),
					}
					"minimum" => match value
					{
						&ConfigurationValue::Number(f) => minimum=Some(f as i32),
						_ => panic!("bad value for minimum"),
					}
					"maximum" => match value
					{
						&ConfigurationValue::Number(f) => maximum=Some(f as i32),
						_ => panic!("bad value for maximum"),
					}
					_ => panic!("Nothing to do with field {} in LabelTransform",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a LabelTransform from a non-Object");
		}
		let multiplier=multiplier.expect("There were no multiplier");
		let summand=summand.expect("There were no summand");
		LabelTransform{
			multiplier,
			summand,
			saturate_bottom,
			saturate_top,
			minimum,
			maximum,
		}
	}
}




///Transform (l,q) into new label a*l+b*q+c*l*q+d
///where l is the label and q is the occupancy.
#[derive(Debug)]
pub struct OccupancyFunction
{
	///Which multiplies the label.
	label_coefficient: i32,
	///Which multiplies the occupancy.
	occupancy_coefficient: i32,
	///Which multiplies the product of label and occupancy.
	product_coefficient: i32,
	///Just added.
	constant_coefficient: i32,
	///Whether to include the own router buffers in the calculation.
	use_internal_space: bool,
	///Whether to include the known state of the next router buffers in the calculation.
	use_neighbour_space: bool,
	///Whether to aggregate all virtual channels associated to the port.
	///Defaults to true.
	aggregate: bool,
}

impl VirtualChannelPolicy for OccupancyFunction
{
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, info: &RequestInfo, topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//let port_average_neighbour_queue_length=port_average_neighbour_queue_length.as_ref().expect("port_average_neighbour_queue_length have not been computed for policy OccupancyFunction");
		let dist=topology.distance(router.get_index().expect("we need routers with index"),info.target_router_index);
		if dist==0
		{
			//do nothing
			candidates
		}
		else
		{
			candidates.into_iter().filter_map(
				//|CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}|
				|candidate|{
				let CandidateEgress{port,virtual_channel,label,..} = candidate;
				let q=if self.use_internal_space
				{
					if self.aggregate
					{
						let port_occupied_output_space=info.port_occupied_output_space.expect("port_occupied_output_space have not been computed for policy OccupancyFunction");
						port_occupied_output_space[port] as i32
					}
					else
					{
						let virtual_channel_occupied_output_space=info.virtual_channel_occupied_output_space.expect("virtual_channel_occupied_output_space have not been computed for OccupancyFunction");
						virtual_channel_occupied_output_space[port][virtual_channel] as i32
					}
				}
				else {0} + if self.use_neighbour_space
				{
					if self.aggregate
					{
						//port_average_neighbour_queue_length[port]
						let status=router.get_status_at_emisor(port).expect("This router does not have transmission status");
						//FIXME: this could be different than the whole occuped space if using a DAMQ or something, although they are yet to be implemented.
						(0..status.num_virtual_channels()).map(|c|router.get_maximum_credits_towards(port,c).expect("we need routers with maximum credits") as i32 - status.known_available_space_for_virtual_channel(c).expect("remote available space is not known.") as i32).sum()
					}
					else
					{
						//port_average_neighbour_queue_length[port]
						let status=router.get_status_at_emisor(port).expect("This router does not have transmission status");
						router.get_maximum_credits_towards(port,virtual_channel).expect("we need routers with maximum credits") as i32 - status.known_available_space_for_virtual_channel(virtual_channel).expect("remote available space is not known.") as i32
					}
				}
				else {0};
				let new_label = self.label_coefficient*label + self.occupancy_coefficient*q + self.product_coefficient*label*q + self.constant_coefficient;
				Some(CandidateEgress{label:new_label,..candidate})
			}).collect::<Vec<_>>()
		}
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl OccupancyFunction
{
	pub fn new(arg:VCPolicyBuilderArgument) -> OccupancyFunction
	{
		let mut label_coefficient=None;
		let mut occupancy_coefficient=None;
		let mut product_coefficient=None;
		let mut constant_coefficient=None;
		let mut use_internal_space=false;
		let mut use_neighbour_space=false;
		let mut aggregate=true;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="OccupancyFunction"
			{
				panic!("A OccupancyFunction must be created from a `OccupancyFunction` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
					"label_coefficient" => match value
					{
						&ConfigurationValue::Number(f) => label_coefficient=Some(f as i32),
						_ => panic!("bad value for label_coefficient"),
					}
					"occupancy_coefficient" => match value
					{
						&ConfigurationValue::Number(f) => occupancy_coefficient=Some(f as i32),
						_ => panic!("bad value for occupancy_coefficient"),
					}
					"product_coefficient" => match value
					{
						&ConfigurationValue::Number(f) => product_coefficient=Some(f as i32),
						_ => panic!("bad value for product_coefficient"),
					}
					"constant_coefficient" => match value
					{
						&ConfigurationValue::Number(f) => constant_coefficient=Some(f as i32),
						_ => panic!("bad value for constant_coefficient"),
					}
 					"use_neighbour_space" => match value
 					{
 						&ConfigurationValue::True => use_neighbour_space=true,
 						&ConfigurationValue::False => use_neighbour_space=false,
 						_ => panic!("bad value for use_neighbour_space"),
 					}
 					"use_internal_space" => match value
 					{
 						&ConfigurationValue::True => use_internal_space=true,
 						&ConfigurationValue::False => use_internal_space=false,
 						_ => panic!("bad value for use_internal_space"),
 					}
 					"aggregate" => match value
 					{
 						&ConfigurationValue::True => aggregate=true,
 						&ConfigurationValue::False => aggregate=false,
 						_ => panic!("bad value for aggregate"),
 					}
					_ => panic!("Nothing to do with field {} in OccupancyFunction",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a OccupancyFunction from a non-Object");
		}
		let label_coefficient=label_coefficient.expect("There were no multiplier");
		let occupancy_coefficient=occupancy_coefficient.expect("There were no multiplier");
		let product_coefficient=product_coefficient.expect("There were no multiplier");
		let constant_coefficient=constant_coefficient.expect("There were no multiplier");
		OccupancyFunction{
			label_coefficient,
			occupancy_coefficient,
			product_coefficient,
			constant_coefficient,
			use_internal_space,
			use_neighbour_space,
			aggregate,
		}
	}
}


///New label = -old_label
///Just until I fix the grammar to accept preceding minuses.
#[derive(Debug)]
pub struct NegateLabel
{
}

impl VirtualChannelPolicy for NegateLabel
{
	fn filter(&self, candidates:Vec<CandidateEgress>, _router:&dyn Router, _info: &RequestInfo, _topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		candidates.into_iter().filter_map(
			//|CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}|
			|candidate|Some(CandidateEgress{label:-candidate.label,..candidate})
		).collect::<Vec<_>>()
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl NegateLabel
{
	pub fn new(arg:VCPolicyBuilderArgument) -> NegateLabel
	{
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="NegateLabel"
			{
				panic!("A NegateLabel must be created from a `NegateLabel` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
					//"multiplier" => match value
					//{
					//	&ConfigurationValue::Number(f) => multiplier=Some(f as i32),
					//	_ => panic!("bad value for multiplier"),
					//}
					_ => panic!("Nothing to do with field {} in NegateLabel",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a NegateLabel from a non-Object");
		}
		NegateLabel{}
	}
}




///Vector of labels
///`new_label = vector[old_label]`
#[derive(Debug)]
pub struct VecLabel
{
	label_vector: Vec<i32>,
}

impl VirtualChannelPolicy for VecLabel
{
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, info: &RequestInfo, topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//let port_average_neighbour_queue_length=port_average_neighbour_queue_length.as_ref().expect("port_average_neighbour_queue_length have not been computed for policy VecLabel");
		let dist=topology.distance(router.get_index().expect("we need routers with index"),info.target_router_index);
		if dist==0
		{
			//do nothing
			candidates
		}
		else
		{
			candidates.into_iter().filter_map(
				//|CandidateEgress{port,virtual_channel,label,estimated_remaining_hops}|
				|candidate|{
				let label = candidate.label;
				if label<0 || label>=self.label_vector.len() as i32
				{
					panic!("label={} is out of range 0..{}",label,self.label_vector.len());
				}
				let new_label = self.label_vector[label as usize];
				Some(CandidateEgress{label:new_label,..candidate})
			}).collect::<Vec<_>>()
		}
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl VecLabel
{
	pub fn new(arg:VCPolicyBuilderArgument) -> VecLabel
	{
		let mut label_vector=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="VecLabel"
			{
				panic!("A VecLabel must be created from a `VecLabel` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
 					"label_vector" => match value
 					{
						&ConfigurationValue::Array(ref l) => label_vector=Some(l.iter().map(|v| match v{
							ConfigurationValue::Number(f) => *f as i32,
							_ => panic!("bad value for label_vector"),
						}).collect()),
 						_ => panic!("bad value for label_vector"),
 					}
					_ => panic!("Nothing to do with field {} in VecLabel",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a VecLabel from a non-Object");
		}
		let label_vector=label_vector.expect("There were no label_vector");
		VecLabel{
			label_vector,
		}
	}
}

///Apply a different policy to candidates with each label.
#[derive(Debug)]
pub struct MapLabel
{
	label_to_policy: Vec<Box<dyn VirtualChannelPolicy>>,
	below_policy: Box<dyn VirtualChannelPolicy>,
	above_policy: Box<dyn VirtualChannelPolicy>,
}

impl VirtualChannelPolicy for MapLabel
{
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, info: &RequestInfo, topology:&dyn Topology, rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//let port_average_neighbour_queue_length=port_average_neighbour_queue_length.as_ref().expect("port_average_neighbour_queue_length have not been computed for policy MapLabel");
		let dist=topology.distance(router.get_index().expect("we need routers with index"),info.target_router_index);
		if dist==0
		{
			//do nothing
			candidates
		}
		else
		{
			let n = self.label_to_policy.len();
			//above goes into candidate_map[ labels  ]
			//below goes into candidate_map[ labels+1  ]
			let mut candidate_map = vec![vec![];n+2];
			for cand in candidates.into_iter()
			{
				let label : usize = if cand.label < 0
				{
					self.label_to_policy.len()+1
				} else if cand.label > n.try_into().unwrap() {
					n
				} else {
					cand.label.try_into().unwrap()
				};
				candidate_map[label].push(cand);
			}
			let mut policies = self.label_to_policy.iter().chain( vec![&self.below_policy].into_iter() ).chain( vec![&self.above_policy].into_iter() );
			let mut r = vec![];
			for candidate_list in candidate_map
			{
				let policy : &dyn VirtualChannelPolicy = policies.next().unwrap().as_ref();
				r.extend( policy.filter(candidate_list,router,info,topology,rng)  );
			}
			r
		}
	}

	fn need_server_ports(&self)->bool
	{
		true
	}

	fn need_port_average_queue_length(&self)->bool
	{
		true
	}

	fn need_port_last_transmission(&self)->bool
	{
		true
	}

}

impl MapLabel
{
	pub fn new(arg:VCPolicyBuilderArgument) -> MapLabel
	{
		let mut label_to_policy=None;
		let mut below_policy : Box<dyn VirtualChannelPolicy> =Box::new(Identity{});
		let mut above_policy : Box<dyn VirtualChannelPolicy> =Box::new(Identity{});
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="MapLabel"
			{
				panic!("A MapLabel must be created from a `MapLabel` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
 					"label_to_policy" => match value
 					{
						&ConfigurationValue::Array(ref l) => label_to_policy=Some(l.iter().map(|v| match v{
							&ConfigurationValue::Object(_,_) => new_virtual_channel_policy(VCPolicyBuilderArgument{cv:v,..arg}),
							_ => panic!("bad value for label_to_policy"),
						}).collect()),
 						_ => panic!("bad value for label_to_policy"),
 					}
					"below_policy" => match value
					{
						&ConfigurationValue::Object(_,_) => below_policy = new_virtual_channel_policy(VCPolicyBuilderArgument{cv:value,..arg}),
 						_ => panic!("bad value for below_policy"),
					}
					"above_policy" => match value
					{
						&ConfigurationValue::Object(_,_) => above_policy = new_virtual_channel_policy(VCPolicyBuilderArgument{cv:value,..arg}),
 						_ => panic!("bad value for above_policy"),
					}
					_ => panic!("Nothing to do with field {} in MapLabel",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a MapLabel from a non-Object");
		}
		let label_to_policy=label_to_policy.expect("There were no label_to_policy");
		MapLabel{
			label_to_policy,
			below_policy,
			above_policy,
		}
	}
}


///Only allows those candidates whose vc equals their entry vc plus some `s` in `shifts`.
#[derive(Debug)]
pub struct ShiftEntryVC
{
	shifts: Vec<i32>,
}

impl VirtualChannelPolicy for ShiftEntryVC
{
	fn filter(&self, candidates:Vec<CandidateEgress>, router:&dyn Router, info: &RequestInfo, topology:&dyn Topology, _rng: &RefCell<StdRng>) -> Vec<CandidateEgress>
	{
		//let port_average_neighbour_queue_length=port_average_neighbour_queue_length.as_ref().expect("port_average_neighbour_queue_length have not been computed for policy ShiftEntryVC");
		let dist=topology.distance(router.get_index().expect("we need routers with index"),info.target_router_index);
		if dist==0
		{
			//do nothing
			candidates
		}
		else
		{
			let evc = info.entry_virtual_channel as i32;
			candidates.into_iter().filter(|&CandidateEgress{virtual_channel,..}| self.shifts.contains(&(virtual_channel as i32-evc)) ).collect::<Vec<_>>()
		}
	}

	fn need_server_ports(&self)->bool
	{
		false
	}

	fn need_port_average_queue_length(&self)->bool
	{
		false
	}

	fn need_port_last_transmission(&self)->bool
	{
		false
	}

}

impl ShiftEntryVC
{
	pub fn new(arg:VCPolicyBuilderArgument) -> ShiftEntryVC
	{
		let mut shifts=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="ShiftEntryVC"
			{
				panic!("A ShiftEntryVC must be created from a `ShiftEntryVC` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
 					"shifts" => match value
 					{
						&ConfigurationValue::Array(ref l) => shifts=Some(l.iter().map(|v| match v{
							&ConfigurationValue::Number(x) => x as i32,
							_ => panic!("bad value for shifts"),
						}).collect()),
 						_ => panic!("bad value for shifts"),
 					}
					_ => panic!("Nothing to do with field {} in ShiftEntryVC",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ShiftEntryVC from a non-Object");
		}
		let shifts=shifts.expect("There were no shifts");
		ShiftEntryVC{
			shifts,
		}
	}
}
