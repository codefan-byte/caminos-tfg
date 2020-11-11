
use std::rc::{Rc,Weak};
use std::cell::RefCell;
use std::mem::{size_of};
use crate::{Phit,Simulation};
use crate::topology::Location;
use crate::quantify::Quantifiable;
use crate::router::{AcknowledgeMessage};
use quantifiable_derive::Quantifiable;//the derive macro

///A trait to be implemented for generic objects to be inserted in the event queue.
pub trait Eventful
{
	///Method to ve called to process the events.
	fn process(&mut self, simulation:&Simulation) -> Vec<EventGeneration>;
	///Number of pending events.
	fn pending_events(&self)->usize;
	///Mark the eventful as having another pending event. It should also be added to some queue.
	fn add_pending_event(&mut self);
	///Mark the eventful as having no pending events. Perhaps it is not necessary, since it is being done by the `process` method.
	fn clear_pending_events(&mut self);
	///Extract the eventful from the implementing class. Required since `as Rc<RefCell<Eventful>>` does not work.
	fn as_eventful(&self)->Weak<RefCell<dyn Eventful>>;
}

///The events stored in the event queue.
#[derive(Clone)]
pub enum Event
{
	//PhitToRouter{
	//	phit: Rc<Phit>,
	//	previous: Location,
	//	router: usize,
	//	port: usize,
	//},
	PhitToLocation{
		phit: Rc<Phit>,
		previous: Location,
		new: Location,
	},
	//PhitClearAcknowledge{
	Acknowledge{
		///Location by which the phit was sent, contaning the transmission status to be informed (such as credit counter).
		location: Location,
		// ///The virtual channel assigned to the phit for this hop
		// virtual_channel: usize,
		message: AcknowledgeMessage,
	},
	Generic(Rc<RefCell<dyn Eventful>>),
}

impl Quantifiable for Event
{
	fn total_memory(&self) -> usize
	{
		let mut total= size_of::<Self>();
		match self
		{
			&Event::PhitToLocation{
				ref phit,
				previous: _,
				new: _,
			} => total+=phit.as_ref().total_memory(),
			_ => (),
		}
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

///This is used to sort the processing of the events inside a cycle.
///If some event occurs at Begin then its result will be visible for events at End. Specifically, we ensure that all the phits have arrived before arbitring.
///Currently at Being: phit movements and clears.
///Currently at End: Generics.
pub enum CyclePosition
{
	Begin,
	End,
}

///Encapsulates a request for insertion on the event queue.
pub struct EventGeneration
{
	///To insert the event after `delay` cycles.
	pub delay: usize,
	///Whether the event should be processed at the begin or the end of its cycle.
	pub position: CyclePosition,
	///The actual event to be inserted.
	pub event: Event,
}

///The event queue structure managing the insertion and extraction of events.
#[derive(Quantifiable)]
pub struct EventQueue
{
	//Would be better to have `Vec<(Vec<Event>,Vec<Event>)>` ?
	event_begin_circle: Vec<Vec<Event>>,//Events to be processed at the beginning of a cycle (mostly arrivals of phits)
	event_end_circle: Vec<Vec<Event>>,//Events to be processed at the end of a cycle (mostly decisions on where to send phits)
	current: usize,
}

//impl Quantifiable for EventQueue
//{
//	fn total_memory(&self) -> usize
//	{
//		return self.event_begin_circle.total_memory()+self.event_end_circle.total_memory()+size_of::<usize>();
//	}
//	fn print_memory_breakdown(&self)
//	{
//		unimplemented!();
//		//println!("event : {}",size_of::<Event>());
//		//let mut count=0;
//		//for el in self.event_begin_circle.iter()
//		//{
//		//	count+=el.len();
//		//}
//		//for el in self.event_end_circle.iter()
//		//{
//		//	count+=el.len();
//		//}
//		//println!("number of events : {}",count);
//	}
//	fn forecast_total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//}

impl EventQueue
{
	///Creates a new EventQueue. `size` should be greater than any possible delay.
	pub fn new (size:usize) -> EventQueue
	{
		EventQueue{
			event_begin_circle: vec![ vec![] ; size ],
			event_end_circle: vec![ vec![] ; size ],
			current:0,
		}
	}
	///Advances the queue by a cycle. This drops the events in the finished cycle.
	pub fn advance(&mut self)
	{
		//self.event_begin_circle[self.current].clear();
		//self.event_end_circle[self.current].clear();
		//Better to drop the old Vec; otherwise their capcity is covering a lot of memory.
		self.event_begin_circle[self.current]=Vec::new();
		self.event_end_circle[self.current]=Vec::new();
		self.current=(self.current+1)%self.event_begin_circle.len();
	}
	///Access to the event in the `ievent` index of the events to be executed at the begin of the cycle.
	pub fn access_begin(&self, ievent:usize) -> Option<&Event>
	{
		let v=&self.event_begin_circle[self.current];
		if ievent<v.len()
		{
			Some(&v[ievent])
		}
		else
		{
			None
		}
	}
	///Access to the event in the `ievent` index of the events to be executed at the end of the cycle.
	pub fn access_end(&self, ievent:usize) -> Option<&Event>
	{
		let v=&self.event_end_circle[self.current];
		if ievent<v.len()
		{
			Some(&v[ievent])
		}
		else
		{
			None
		}
	}
	///Adds an event to the list of events to be executed at the begin of the cycle `current_cycle + delay`.
	pub fn enqueue_begin(&mut self, event:Event, delay: usize)
	{
		if delay>=self.event_begin_circle.len()
		{
			panic!("Delay too long");
		}
		let position=(self.current+delay)%self.event_begin_circle.len();
		self.event_begin_circle[position].push(event);
	}
	///Adds an event to the list of events to be executed at the end of the cycle `current_cycle + delay`.
	pub fn enqueue_end(&mut self, event:Event, delay: usize)
	{
		if delay>=self.event_end_circle.len()
		{
			panic!("Delay too long");
		}
		let position=(self.current+delay)%self.event_end_circle.len();
		self.event_end_circle[position].push(event);
	}
	///Adds an event as it requests.
	pub fn enqueue(&mut self, event_generation:EventGeneration)
	{
		match event_generation.position
		{
			CyclePosition::Begin => self.enqueue_begin(event_generation.event,event_generation.delay),
			CyclePosition::End => self.enqueue_end(event_generation.event,event_generation.delay),
		};
	}
}

