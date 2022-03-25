/*!
 * A Allocator defines the interface for an allocation strategy for a router crossbar
*/

pub mod random;
//pub mod islip;
//pub mod separable_input_first;

use std::cell::{RefCell};
use ::rand::{StdRng};

use crate::quantify::Quantifiable;
use crate::Plugs;
use crate::config_parser::ConfigurationValue;
use random::RandomAllocator;

/// A client (input of crossbar) want a resource (output of crossbar) with a certain priority.
pub struct Request {
    /// The input of the crossbar
    pub client: usize,
    /// The output of the crossbar
    pub resource: usize,
    /// The priority of the request
    pub priority: usize,
}

/// A collection of granted requests (i.e. the requests that have been granted)
pub struct GrantedRequests {
    pub granted_requests: Vec<Request>,
}

pub trait Allocator : Quantifiable {
    /// Get number of clients
    fn num_clients(&self) -> usize;
    /// Get number of resources
    fn num_resources(&self) -> usize;


    /// Add a request
    fn add_request(&mut self, request: Request);
    /// Returns the granted requests and clear the client's requests
    fn perform_allocation(&mut self) -> GrantedRequests;
 
}

pub struct AllocatorBuilderArgument<'a>
{
    /// A ConfigurationValue::Object defining the allocator
    pub cv : &'a ConfigurationValue,
    /// The number of outputs of the router crossbar
    pub num_resources : usize,
    /// The number of inputs of the router crossbar
    pub num_clients : usize,

    /// A reference to the Plugs object
    pub plugs : &'a Plugs,
    /// The random number generator to use
    pub rng : &'a RefCell<StdRng>,
}

pub fn new_allocator(arg:AllocatorBuilderArgument) -> Box<dyn Allocator>
{
    if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=arg.cv
    {
/*         match arg.plugs.allocators.get(cv_name)
        {
            Some(builder) => return builder(arg),
            _ => (),
        }; */
        match cv_name.as_ref()
        {
            "Random" => return Box::new(RandomAllocator::new(arg)),
          //  "Islip" => return Box::new(IslipAllocator::new(arg)),
          //  "SeparableInputFirst" => return Box::new(SeparableInputFirstAllocator::new(arg)),
            _ => panic!("Unknown allocator: {}", cv_name),
        }
    }
    else
    {
        panic!("Allocator must be an object");
    }
}