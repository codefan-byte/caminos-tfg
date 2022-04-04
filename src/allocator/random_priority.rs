use std::cell::RefCell;

use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::prelude::SliceRandom;
use std::ops::DerefMut;

//use quantifiable_derive::Quantifiable;//the derive macro
use crate::allocator::{Allocator, Request, GrantedRequests, AllocatorBuilderArgument};
use crate::config_parser::ConfigurationValue;
use crate::match_object_panic;

pub struct Resource {
    /// Index of the client that has the resource (or None if the resource is free)
    client: Option<usize>,
}

/// A random allocator that randomly allocates requests to resources
pub struct RandomPriorityAllocator {
    /// The max number of outputs of the router crossbar
    num_resources: usize,
    /// The max number of inputs of the router crossbar
    num_clients: usize,
    /// The requests of the clients
    requests: Vec<Request>,
    /// The RNG or None if the seed is not set
    rng: Option<StdRng>,
}

impl RandomPriorityAllocator {
    /// Create a new random priority allocator
    /// # Parameters
    /// * `args` - The arguments for the allocator
    /// # Returns
    /// * `RandomPriorityAllocator` - The new random priority allocator
    pub fn new(args: AllocatorBuilderArgument) -> RandomPriorityAllocator {
        // Check if the arguments are valid
        if args.num_clients <= 0 || args.num_resources <= 0 {
            panic!("Invalid arguments")
        }
        // Get the seed from the configuration
        let mut seed = None;
        match_object_panic!(args.cv, "RandomAllocator", value,
        "seed" => match value
        {
            &ConfigurationValue::Number(s) => seed = Some(s as u64),
            _ => panic!("Bad value for seed"),
        }
        );
        let rng = seed.map(|s| StdRng::seed_from_u64(s));
        // Create the allocator
        RandomPriorityAllocator {
            num_resources: args.num_resources,
            num_clients: args.num_clients,
            requests: Vec::new(),
            rng,
        }
    }

    /// Check if the request is valid
    /// # Arguments
    /// * `request` - The request to check
    /// # Returns
    /// * `bool` - True if the request is valid, false otherwise
    /// # Remarks
    /// The request is valid if
    /// the client is in the range [0, num_clients) and
    /// the resource is in the range [0, num_resources) and
    /// the priority is is not None
    fn is_valid_request(&self, _request: &Request) -> bool {
        if _request.client >= self.num_clients || _request.resource >= self.num_resources || _request.priority.is_none() {
            return false
        }
        true
    }
}

impl Allocator for RandomPriorityAllocator {
    /// Add a request to the allocator
    /// # Arguments
    /// * `request` - The request to add
    /// # Remarks
    /// The request is valid if the client is in the range [0, num_clients) and the resource is in the range [0, num_resources) and the priority is is not None
    fn add_request(&mut self, request: Request) {
        // Check if the request is valid
        if !self.is_valid_request(&request) {
            panic!("Invalid request");
        }
        self.requests.push(request);
    }

    /// Perform the allocation
    /// # Arguments
    /// * `rng` - The RNG to use if the seed is not set
    /// # Returns
    /// * `GrantedRequests` - The granted requests
    /// # Remarks
    /// If the seed is not set, the passed RNG is used to generate the random numbers
    /// The granted requests are sorted by priority (from low to high)
    fn perform_allocation(&mut self, rng : &RefCell<StdRng>) -> GrantedRequests {
        // Create the granted requests vector
        let mut gr = GrantedRequests { granted_requests: Vec::new() };
        
        // The resources allocated to the clients
        let mut resources: Vec<Resource> = Vec::new();
        
        // Fill the resources vector with the free resources
        (0..self.num_resources).for_each(|_| {
            if let Some(client) = self.requests.first().map(|r| r.client) {
                resources.push(Resource { client: Some(client) });
            } else {
                resources.push(Resource { client: None });
            }
        });

        // Shuffle the requests using the RNG passed as parameter
        // Except if the seed is set, in which case we use it
        let mut borrowed_rng = rng.borrow_mut();
        let rng = self.rng.as_mut().unwrap_or(borrowed_rng.deref_mut());
        self.requests.shuffle(rng);

        // Sort the requests by priority (least is first)
        self.requests.sort_by(|a, b| a.priority.unwrap().cmp(&b.priority.unwrap()));

        // Allocate the requests with an iterator
        for Request{ref resource, ref client, ref priority } in self.requests.iter() {
            // Check if the wanted resource is available
            if resources[*resource].client.is_none() {
                // Add the request to the granted requests
                gr.granted_requests.push(Request{
                    client: *client,
                    resource: *resource,
                    priority: *priority,
                });
                // Allocate the resource
                resources[*resource].client = Some(*client);
            } else {
                // The resource is not available, so we can't grant the request
                continue;
            }
        }
        // Clear the requests and resources
        self.requests.clear();
        resources.clear();
        // Return the granted requests
        gr
    }
    /// Check if the allocator supports the intransit priority option
    fn support_intransit_priority(&self) -> bool {
        true
    }
}

