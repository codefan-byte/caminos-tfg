/*!
 * A Allocator defines the interface for an allocation strategy for a router crossbar
*/

pub mod random;
//pub mod islip;
//pub mod separable_input_first;

use std::cell::RefCell;
use ::rand::StdRng;

//use crate::quantify::Quantifiable;
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

/// A collection of granted requests
pub struct GrantedRequests {
    /// The granted requests
    pub granted_requests: Vec<Request>,
}

pub trait Allocator {
    /// Get number of clients.
    /// This is the number of inputs of the router crossbar.
    /// # Returns
    /// The number of clients in the allocator
    fn num_clients(&self) -> usize;
    /// Get number of resources
    /// This is the number of resources that can be allocated to clients, not the number of resources that are actually allocated to clients
    /// # Returns
    /// The number of resources
    fn num_resources(&self) -> usize;


    /// Add a new request to the allocator.
    /// (It assumes that the request is not already in the allocator)
    /// # Arguments
    /// * `request` - The request to add
    fn add_request(&mut self, request: &Request);
    /// Returns the granted requests and clear the client's requests
    /// # Parameters
    /// * `rng` - The random number generator to use
    /// # Returns
    /// * `GrantedRequests` - The granted requests
    fn perform_allocation(&mut self, rng : &RefCell<StdRng>) -> GrantedRequests;
 
}

/// Arguments for the allocator builder
#[non_exhaustive]
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