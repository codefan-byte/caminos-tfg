
mod basic;

use std::rc::{Rc};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem::{size_of};
use std::collections::{BTreeMap};
use ::rand::{Rng,StdRng};
use quantifiable_derive::Quantifiable;//the derive macro
use self::basic::Basic;
use crate::config_parser::ConfigurationValue;
use crate::topology::{Topology};
use crate::{Phit,Packet};
use crate::event::{Eventful};
use crate::quantify::Quantifiable;
use crate::Plugs;

///The interface that a router type must follow.
pub trait Router: Eventful + Quantifiable
{
	///Introduces a phit into the router in the specified port
	fn insert(&mut self, phit:Rc<Phit>, port:usize, rng: &RefCell<StdRng>);
	///Receive the acknowledge of a phit clear. Generally to increase the credit count
	fn acknowledge(&mut self, port:usize, ack_message:AcknowledgeMessage);
	///To get the number of virtual channels the router uses.
	fn num_virtual_channels(&self) -> usize;
	///Get the number of phits that fit inside the buffer of a port.
	fn virtual_port_size(&self, port:usize, virtual_channel:usize) -> usize;
	///To iterate over the phits managed by the router. Required to account memory.
	fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>;
	///Get a virtual port if any.
	///To be used in some policies, e.g., VirtualChannelPolicy::Shortest.
	fn get_status_at_emisor(&self, port:usize) -> Option<&dyn StatusAtEmissor>;
	///Get the maximum number of credits towards the neighbour.
	///To be used in policies such as VirtualChannelPolicy::LowestSinghWeight.
	fn get_maximum_credits_towards(&self, port:usize, virtual_channel:usize) -> Option<usize>;
	///Get the index of the router in the topology.
	///To be used in policies such as VirtualChannelPolicy::LowestSinghWeight.
	fn get_index(&self)->Option<usize>;
	///To optionally write router statistics into the simulation output.
	///Each router receives the aggregate of the statistics of the previous routers.
	///In the frist router we have `statistics=None` and `router_index=0`.
	///In the last router we have `router_index+1==total_routers==topology.routers.len()`, that may be used for final normalizations.
	fn aggregate_statistics(&self, statistics:Option<ConfigurationValue>, router_index:usize, total_routers:usize, cycle:usize) -> Option<ConfigurationValue>;
	///Clears all collected statistics
	fn reset_statistics(&mut self,next_cycle:usize);
}

#[non_exhaustive]
pub struct RouterBuilderArgument<'a>
{
	///The index of the router being created
	pub router_index: usize,
	///A ConfigurationValue::Object defining the router.
	pub cv: &'a ConfigurationValue,
	///The user defined plugs. In case the router needs to create elements.
	pub plugs: &'a Plugs,
	///The topology of which the router is gonna be part.
	pub topology: &'a dyn Topology,
	///The maximum number of phits that packet gonna have.
	pub maximum_packet_size: usize,
}

///Creates a router from a configuration value.
pub fn new_router(arg:RouterBuilderArgument) -> Rc<RefCell<dyn Router>>
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=arg.cv
	{
		match arg.plugs.routers.get(cv_name)
		{
			//Some(builder) => return builder(router_index,cv,plugs,topology,maximum_packet_size),
			Some(builder) => return builder(arg),
			_ => (),
		};
		match cv_name.as_ref()
		{
			"Basic" => Basic::<SimpleVirtualChannels>::new(arg.router_index, arg.cv, arg.plugs, arg.topology, arg.maximum_packet_size),
			_ => panic!("Unknown router {}",cv_name),
		}
	}
	else
	{
		panic!("Trying to create a Router from a non-Object");
	}
}


///An unbounded queue of phits.
pub struct Buffer
{
	pub phits: VecDeque<Rc<Phit>>,
}

impl Buffer
{
	#[allow(dead_code)]
	pub fn new() -> Buffer
	{
		Buffer{ phits: VecDeque::new() }
	}
	pub fn push(&mut self, phit:Rc<Phit>)
	{
		self.phits.push_back(phit);
	}
	pub fn pop(&mut self) -> Option<Rc<Phit>>
	{
		self.phits.pop_front()
	}
	pub fn front(&self) -> Option<Rc<Phit>>
	{
		match self.phits.front()
		{
			None => None,
			Some(rphit) => Some(rphit.clone()),
		}
	}
	///How many phits are currently in the buffer.
	pub fn len(&self) -> usize
	{
		self.phits.len()
	}
	pub fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		Box::new(self.phits.iter().map(|p|p.clone()).collect::<Vec<_>>().into_iter())
	}
}

impl Quantifiable for Buffer
{
	fn total_memory(&self) -> usize
	{
		//We add +1 beacause one hole in the implementation of VecDeque
		return size_of::<Buffer>() + (self.phits.capacity()+1)*size_of::<Rc<Phit>>();
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


///An unbounded queue of phits with extra info.
///To use in `Router`s to keep track of selections.
struct AugmentedBuffer<ExtraInfo>
{
	phits: VecDeque<(Rc<Phit>,ExtraInfo)>,
}

impl<ExtraInfo> AugmentedBuffer<ExtraInfo>
{
	fn new() -> AugmentedBuffer<ExtraInfo>
	{
		AugmentedBuffer{ phits: VecDeque::new() }
	}
	fn push(&mut self, phit:Rc<Phit>, extra: ExtraInfo)
	{
		self.phits.push_back((phit,extra));
	}
	fn pop(&mut self) -> Option<(Rc<Phit>,ExtraInfo)>
	{
		self.phits.pop_front()
	}
	fn front(&self) -> Option<(Rc<Phit>,ExtraInfo)> where ExtraInfo:Clone
	{
		match self.phits.front()
		{
			None => None,
			Some(rphit) => Some(rphit.clone()),
		}
	}
	///How many phits are currently in the buffer.
	fn len(&self) -> usize
	{
		self.phits.len()
	}
	#[allow(dead_code)]
	fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		Box::new(self.phits.iter().map(|p|p.0.clone()).collect::<Vec<_>>().into_iter())
	}
}

impl<ExtraInfo> Quantifiable for AugmentedBuffer<ExtraInfo>
{
	fn total_memory(&self) -> usize
	{
		//We add +1 beacause one hole in the implementation of VecDeque
		return size_of::<AugmentedBuffer<ExtraInfo>>() + (self.phits.capacity()+1)*size_of::<(Rc<Phit>,ExtraInfo)>();
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



///Structure for a port to contain the information necessary about the other endpoint, so that we can know if we can send data.
pub trait StatusAtEmissor
{
	///Get the number of virtual channels used in the link.
	fn num_virtual_channels(&self)->usize;
	///Receive a phit acknowledge from the receiving endpoint.
	fn acknowledge(&mut self, message:AcknowledgeMessage);
	///Keep track of a outcoming phit.
	fn notify_outcoming_phit(&mut self, virtual_channel: usize, cycle:usize);
	///Check if we can transmit a given phit.
	fn can_transmit(&self, phit:&Rc<Phit>, virtual_channel:usize)->bool;
	///Check if we can surely transmit and store the whole remaining of the packet.
	fn can_transmit_whole_packet(&self, phit:&Rc<Phit>, virtual_channel:usize)->bool;
	///Consult available space.
	fn known_available_space_for_virtual_channel(&self,virtual_channel:usize)->Option<usize>;
	///Get timestamp of last transmission.
	fn get_last_transmission(&self)->usize;
}

///A structure to store incoming phits.
pub trait SpaceAtReceptor
{
	///inserts a phit in the buffer space. I may return an error if the phit cannot be inserted.
	fn insert(&mut self, phit:Rc<Phit>, rng: &RefCell<StdRng>) -> Result<(),()>;
	///Iterate over the phits that can be processed by other structures, such as a crossbar.
	fn front_iter(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>;
	///Consult if there is a processable phit in a given virtual channel.
	fn front_virtual_channel(&self,virtual_channel:usize) -> Option<Rc<Phit>>;
	///Extract a phit in a given virtual channel and returns it.
	fn extract(&mut self, virtual_channel:usize) -> Result<(Rc<Phit>,Option<AcknowledgeMessage>),()>;
	///Iterates over all the stored phits. Do not assume any ordering.
	fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>;
	///Consult currently available space in phits dedicated to a virtual channel.
	fn available_dedicated_space(&self, virtual_channel:usize) -> Option<usize>;
	///Consult current number of phits in space dedicated to a virtual channel.
	fn occupied_dedicated_space(&self, virtual_channel:usize) -> Option<usize>;
}

///A message send from the receptor to the emissor when the receptor state changes.
///Usually a phit is extracted from the buffer and we want the emissor's credit counter to increase.
#[derive(Clone,Debug)]
pub struct AcknowledgeMessage
{
	virtual_channel: Option<usize>,
	set_available_size: Option<usize>,
}

impl AcknowledgeMessage
{
	pub fn ack_empty()->AcknowledgeMessage
	{
		AcknowledgeMessage{
			virtual_channel: None,
			set_available_size: None,
		}
	}
	pub fn ack_phit_clear_from_virtual_channel(virtual_channel:usize)->AcknowledgeMessage
	{
		AcknowledgeMessage{
			virtual_channel: Some(virtual_channel),
			set_available_size: None,
		}
	}
	pub fn ack_fix_available_size(amount:usize)->AcknowledgeMessage
	{
		
		AcknowledgeMessage{
			virtual_channel: None,
			set_available_size: Some(amount),
		}
	}
}


///How packets left a router and reach the next.
///Declares a `StatusAtEmissor` to keep a register on the emissor of the status of the receptor, such as credit counters.
///Declares a `SpaceAtReceptor` necessary to store the incoming packets.
///It implies a contract between the pair of types (`StatusAtEmissor`, `SpaceAtReceptor`), which should be logically compatible.
pub trait TransmissionMechanism
{
	type StatusAtEmissor: StatusAtEmissor;
	type SpaceAtReceptor: SpaceAtReceptor;
	//type AcknowledgeMessage: AcknowledgeMessage;
	fn new_status_at_emissor(&self)-> Self::StatusAtEmissor;
	fn new_space_at_receptor(&self)-> Self::SpaceAtReceptor;
	//Receive a phit acknowledge from the receiving endpoint.
	//fn acknowledge(status:&mut Self::StatusAtEmissor, message:Self::AcknowledgeMessage);
}

//struct AckPhitFromVirtualChannel
//{
//	virtual_channel: usize,
//}
//
//impl AcknowledgeMessage for AckPhitFromVirtualChannel {}

///A simple status consisting of a credit counter per virtual channel.
struct CreditCounterVector
{
	///The known available space in the next router by the given index (usually for virtual channel).
	pub neighbour_credits: Vec<usize>,
	///Cycle in which the last phit was trasmitted out of this port.
	last_transmission:usize,
	///Credits required in the next router's virtual port to begin the transmission
	flit_size: usize,
}

impl StatusAtEmissor for CreditCounterVector
{
	fn num_virtual_channels(&self)->usize
	{
		self.neighbour_credits.len()
	}

	//fn acknowledge(&mut self, virtual_channel:usize)
	fn acknowledge(&mut self, message:AcknowledgeMessage)
	{
		//self.neighbour_credits[virtual_channel]+=1;
		self.neighbour_credits[message.virtual_channel.expect("there is no virtual channel in the message")]+=1;
	}
	
	fn notify_outcoming_phit(&mut self, virtual_channel: usize, cycle:usize)
	{
		self.neighbour_credits[virtual_channel]-=1;
		self.last_transmission=cycle;
	}
	
	fn can_transmit(&self, phit:&Rc<Phit>, virtual_channel:usize)->bool
	{
		let mut necessary_credits=1;
		if phit.is_begin()
		{
			necessary_credits=self.flit_size;
		}
		self.neighbour_credits[virtual_channel]>=necessary_credits
	}
	
	fn can_transmit_whole_packet(&self, phit:&Rc<Phit>, virtual_channel:usize)->bool
	{
		let necessary_credits=phit.packet.size - phit.index;
		self.neighbour_credits[virtual_channel]>=necessary_credits
	}
	
	fn known_available_space_for_virtual_channel(&self,virtual_channel:usize)->Option<usize>
	{
		Some(self.neighbour_credits[virtual_channel])
	}
	
	fn get_last_transmission(&self)->usize
	{
		self.last_transmission
	}
}

///A simple collection of buffers. Normally each being dedicated to a virtual channel.
pub struct ParallelBuffers
{
	///The phits in the transit queue that came from the previous router/server
	buffers: Vec<Buffer>,
	///Stores the virtual channels chosen for incoming packets without a virtual channel already selected.
	//FIXME: try to delete this.
	input_virtual_channel_choices: BTreeMap<*const Packet,usize>,
}

impl SpaceAtReceptor for ParallelBuffers
{
	fn insert(&mut self, phit:Rc<Phit>, rng: &RefCell<StdRng>) -> Result<(),()>
	{
		let current_vc=*phit.virtual_channel.borrow();
		let vc=match current_vc
		{
			// XXX We need to ensure that all the phits get into the same buffer.
			//FIXME: revise, see basic.rs
			//None => 0,//FIXME we should use the policy. But for the whole packet.
		 	None =>
		 	{
		 		let packet=phit.packet.clone();
		 		let packet_ptr=packet.as_ref() as *const Packet;
		 		let vc={
		 			if phit.is_begin()
		 			{
		 				let r=rng.borrow_mut().gen_range(0,self.buffers.len());
		 				self.input_virtual_channel_choices.insert(packet_ptr,r);
		 				r
		 			}
		 			else
		 			{
		 				//*self.input_virtual_channel_choices.get(&packet_ptr).expect("cannot assign a virtual channel if it is not the first phit.")
		 				match self.input_virtual_channel_choices.get(&packet_ptr)
						{
							Some ( x ) => *x,
							None =>
							{
								panic!("Cannot assign a virtual channel if it is not the first phit.\n\tphit index={}\n\tpacket size={}\n\tpacket index={}\n\trouting info hops={}\n",phit.index,packet.size,packet.index,packet.routing_info.borrow().hops);
							}
						}
		 			}
		 		};
		 		if phit.is_end()
		 		{
		 			self.input_virtual_channel_choices.remove(&packet_ptr);
		 		}
		 		*phit.virtual_channel.borrow_mut()=Some(vc);
		 		//*phit_vc_borrow=Some(vc);
		 		vc
		 	}
			Some(vc) => vc,
		};
		self.buffers[vc].push(phit);
		Ok(())
	}
	
	fn front_iter(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		Box::new(self.buffers.iter().filter_map(|b|b.front()).collect::<Vec<_>>().into_iter())
	}
	
	fn front_virtual_channel(&self,virtual_channel:usize) -> Option<Rc<Phit>>
	{
		self.buffers[virtual_channel].front()
	}
	
	fn extract(&mut self, virtual_channel:usize) -> Result<(Rc<Phit>,Option<AcknowledgeMessage>),()>
	{
		//self.buffers[virtual_channel].pop().ok_or(())
		match self.buffers[virtual_channel].pop()
		{
			Some(phit) =>
			{
				let message=AcknowledgeMessage::ack_phit_clear_from_virtual_channel(virtual_channel);
				Ok((phit,Some(message)))
			},
			_ => Err(()),
		}
	}
	
	fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		Box::new(self.buffers.iter().flat_map(|buffer|buffer.iter_phits()).collect::<Vec<_>>().into_iter())
	}
	fn available_dedicated_space(&self, _virtual_channel:usize) -> Option<usize>
	{
		//We are not storing this size...
		None
	}
	fn occupied_dedicated_space(&self, virtual_channel:usize) -> Option<usize>
	{
		Some(self.buffers[virtual_channel].len())
	}
}

//pub struct AcknowledgeSinglePhit();
//impl AcknowledgeMessage for AcknowledgeSinglePhit{}

///A simple virtual channel mechanism
///There is an independent buffer for each of the `virtual_channels` with space for `buffer_size` phits.
///It keeps track of the space of the neighbour using credit counters.
struct SimpleVirtualChannels
{
	///The number of virtual channels = number of buffers.
	virtual_channels: usize,
	///The size of each buffer.
	buffer_size: usize,
	///Credits required in the next router's virtual port to begin the transmission
	flit_size: usize,
}

impl SimpleVirtualChannels
{
	fn new(virtual_channels: usize, buffer_size: usize, flit_size:usize) -> SimpleVirtualChannels
	{
		SimpleVirtualChannels{virtual_channels, buffer_size, flit_size}
	}
}

impl TransmissionMechanism for SimpleVirtualChannels
{
	type StatusAtEmissor = CreditCounterVector;
	type SpaceAtReceptor = ParallelBuffers;
	//type AcknowledgeMessage = AckPhitFromVirtualChannel;
	
	fn new_status_at_emissor(&self)-> CreditCounterVector
	{
		CreditCounterVector{
			neighbour_credits: vec![self.buffer_size;self.virtual_channels],
			last_transmission: 0,
			flit_size: self.flit_size,
		}
	}

	fn new_space_at_receptor(&self)-> ParallelBuffers
	{
		ParallelBuffers{
			buffers: (0..self.virtual_channels).map(|_|Buffer{phits: VecDeque::new()}).collect(),
			input_virtual_channel_choices: BTreeMap::new(),
		}
	}
}


///For senders that not care about the receptor or phantom senders that do not actually send anything.
struct EmptyStatus();

///For receptors that do not require space, let it be because they consume it immediately or because they do not actually receive anything.
struct NoSpace();

impl StatusAtEmissor for EmptyStatus
{
	fn num_virtual_channels(&self)->usize
	{
		1
	}

	fn acknowledge(&mut self, _message:AcknowledgeMessage)
	//fn acknowledge(&mut self, _virtual_channel:usize)
	{
	}

	fn notify_outcoming_phit(&mut self, _virtual_channel: usize, _cycle:usize)
	{
	}

	fn can_transmit(&self, _phit:&Rc<Phit>, _virtual_channel:usize)->bool
	{
		true
	}
	
	fn can_transmit_whole_packet(&self, _phit:&Rc<Phit>, _virtual_channel:usize)->bool
	{
		true
	}

	fn known_available_space_for_virtual_channel(&self,_virtual_channel:usize)->Option<usize>
	{
		//FIXME: unlimited?
		Some(1000)
	}

	fn get_last_transmission(&self)->usize
	{
		//FIXME: this is not true, but is only used for servers...
		0
	}
}

impl SpaceAtReceptor for NoSpace
{
	fn insert(&mut self, _phit:Rc<Phit>, _rng: &RefCell<StdRng>) -> Result<(),()>
	{
		unimplemented!()
	}

	fn front_iter(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		unimplemented!()
	}

	fn front_virtual_channel(&self,_virtual_channel:usize) -> Option<Rc<Phit>>
	{
		unimplemented!()
	}

	fn extract(&mut self, _virtual_channel:usize) -> Result<(Rc<Phit>,Option<AcknowledgeMessage>),()>
	{
		unimplemented!()
	}

	fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		//Is there a better empty iterator?
		//Box::new(Vec::new().into_iter())
		unimplemented!()
	}
	fn available_dedicated_space(&self, _virtual_channel:usize) -> Option<usize>
	{
		Some(0)
	}
	fn occupied_dedicated_space(&self, _virtual_channel:usize) -> Option<usize>
	{
		Some(0)
	}
}

///A mechanism for sending phits to a server. We assume the server can consume all that comes via the link. Thus we do not require any check.
struct TransmissionToServer();
//struct AckFixAvailableSize
//{
//	available_size: usize,
//}
//
//impl AcknowledgeMessage for AckFixAvailableSize {}

impl TransmissionMechanism for TransmissionToServer
{
	type StatusAtEmissor = EmptyStatus;
	type SpaceAtReceptor = NoSpace;
	//type AcknowledgeMessage = AcknowledgeSinglePhit;//FIXME
	
	fn new_status_at_emissor(&self)-> EmptyStatus
	{
		EmptyStatus()
	}

	fn new_space_at_receptor(&self)-> NoSpace
	{
		NoSpace()
	}
}

///What a server needs to know of a router to send it packets.
#[derive(Clone,Quantifiable)]
pub struct StatusAtServer
{
	//buffer_amount: usize,
	//buffer_size: usize,
	available_size: usize,
	size_to_send: usize,
}

impl StatusAtEmissor for StatusAtServer
{
	fn num_virtual_channels(&self)->usize
	{
		1
	}

	fn acknowledge(&mut self, message:AcknowledgeMessage)
	//fn acknowledge(&mut self, _virtual_channel:usize)
	{
		//self.available_size+=1;
		self.available_size=message.set_available_size.expect("there is no set_avilable_size in the message");
	}

	fn notify_outcoming_phit(&mut self, _virtual_channel: usize, _cycle:usize)
	{
		self.available_size-=1;
	}

	fn can_transmit(&self, phit:&Rc<Phit>, _virtual_channel:usize)->bool
	{
		if phit.is_begin()
		{
			self.available_size>=self.size_to_send
		}
		else
		{
			true
		}
	}
	
	fn can_transmit_whole_packet(&self, _phit:&Rc<Phit>, _virtual_channel:usize)->bool
	{
		false
	}

	fn known_available_space_for_virtual_channel(&self,_virtual_channel:usize)->Option<usize>
	{
		Some(self.available_size)
	}

	fn get_last_transmission(&self)->usize
	{
		unimplemented!()
	}
}

///A mechanism to receive phits from a server.
pub struct TransmissionFromServer
{
	///Number of buffers in the receptor.
	buffer_amount: usize,
	///Size of each buffer of the receptor.
	buffer_size: usize,
	///Required available space in the receptor before sendind a packet.
	size_to_send: usize,
}

impl TransmissionFromServer
{
	pub fn new(buffer_amount:usize, buffer_size:usize, size_to_send:usize) -> TransmissionFromServer
	{
		TransmissionFromServer{
			buffer_amount,
			buffer_size,
			size_to_send,
		}
	}
}

///A simple collection of buffers. The selected virtual channel of the emissor is ignored, the packet is inserted at random in any in which it fits.
pub struct AgnosticParallelBuffers
{
	///The phits in the transit queue that came from the previous router/server
	buffers: Vec<Buffer>,
	///The size of each buffer.
	buffer_size: usize,
	///The buffer in which we are injecting the current packet.
	currently_selected: usize,
}

impl SpaceAtReceptor for AgnosticParallelBuffers
{
	fn insert(&mut self, phit:Rc<Phit>, rng: &RefCell<StdRng>) -> Result<(),()>
	{
		if phit.is_begin()
		{
			let good:Vec<usize>=self.buffers.iter().enumerate().filter_map(|(index,buffer)|{
				let available = self.buffer_size - buffer.len();
				if available >= phit.packet.size
				{
					Some(index)
				}
				else
				{
					None
				}
			}).collect();
			if good.len()==0
			{
				panic!("There is no space for the packet. packet.size={} available={:?}",phit.packet.size,self.buffers.iter().map(|buffer|self.buffer_size-buffer.len()).collect::<Vec<usize>>());
			}
			let r=rng.borrow_mut().gen_range(0,good.len());
			self.currently_selected=good[r]
		}
		let index = self.currently_selected;
		*phit.virtual_channel.borrow_mut()=Some(index);
		//let current_vc=*phit.virtual_channel.borrow();
		self.buffers[index].push(phit);
		Ok(())
	}
	
	fn front_iter(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		Box::new(self.buffers.iter().filter_map(|b|b.front()).collect::<Vec<_>>().into_iter())
	}
	
	///Note that although we ignore the virtual channel for the emissor we consider the buffer index to be the current virtual channel.
	fn front_virtual_channel(&self,virtual_channel:usize) -> Option<Rc<Phit>>
	{
		self.buffers[virtual_channel].front()
	}
	
	fn extract(&mut self, virtual_channel:usize) -> Result<(Rc<Phit>,Option<AcknowledgeMessage>),()>
	{
		//self.buffers[virtual_channel].pop().ok_or(())
		match self.buffers[virtual_channel].pop()
		{
			Some(phit) =>
			{
				let available_size = self.buffers.iter().map(|b|self.buffer_size - b.len()).max().expect("no buffers");
				//FIXME: we have to correct by link delay somewhere. Assuming delay=1 cycle here.
				let available_size = if available_size>=1
				{
					available_size - 1
				}
				else
				{
					0
				};
				let message=AcknowledgeMessage::ack_fix_available_size(available_size);
				Ok((phit,Some(message)))
			},
			_ => Err(()),
		}
	}
	
	fn iter_phits(&self) -> Box<dyn Iterator<Item=Rc<Phit>>>
	{
		Box::new(self.buffers.iter().flat_map(|buffer|buffer.iter_phits()).collect::<Vec<_>>().into_iter())
	}
	fn available_dedicated_space(&self, virtual_channel:usize) -> Option<usize>
	{
		Some(self.buffer_size - self.buffers[virtual_channel].len())
	}
	fn occupied_dedicated_space(&self, virtual_channel:usize) -> Option<usize>
	{
		Some(self.buffers[virtual_channel].len())
	}
}


impl TransmissionMechanism for TransmissionFromServer
{
	type StatusAtEmissor = StatusAtServer;
	type SpaceAtReceptor = AgnosticParallelBuffers;
	//type AcknowledgeMessage = AckFixAvailableSize;
	
	fn new_status_at_emissor(&self)-> StatusAtServer
	{
		StatusAtServer{
			available_size: self.buffer_size,
			size_to_send: self.size_to_send,
		}
	}

	fn new_space_at_receptor(&self)-> AgnosticParallelBuffers
	{
		AgnosticParallelBuffers{
			buffers: (0..self.buffer_amount).map(|_|Buffer{phits: VecDeque::new()}).collect(),
			buffer_size: self.buffer_size,
			currently_selected:0,
		}
	}
}


