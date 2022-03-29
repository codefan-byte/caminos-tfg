
use std::cell::RefCell;

use rand::{StdRng, Rng};

//use quantifiable_derive::Quantifiable;//the derive macro
use crate::allocator::{Allocator, Request, GrantedRequests, AllocatorBuilderArgument};

/// A random allocator that randomly allocates requests to resources
pub struct RandomAllocator {
    /// The max number of outputs of the router crossbar
    num_resources: usize,
    /// The max number of inputs of the router crossbar
    num_clients: usize,
    /// The requests of the clients
    requests: Vec<Request>,
}

pub struct Resource {
    /// Index of the client that has the resource (or None if the resource is free)
    client: Option<usize>,
}

impl Allocator for RandomAllocator {
    fn add_request(&mut self, request: Request) {
        // Check if the request is valid
        if !self.is_valid_request(&request) {
            panic!("Invalid request");
        }
        self.requests.push(request);
    }

    fn perform_allocation(&mut self, rng : &RefCell<StdRng>) -> GrantedRequests {
        // Create the granted requests vector
        let mut gr = GrantedRequests { granted_requests: Vec::new() };
        
        // The resources allocated to the clients
        let mut resources: Vec<Resource> = Vec::new();

        // Fill the resources vector with the free resources
        for _ in 0..self.num_resources {
            if let Some(client) = self.requests.first().map(|r| r.client) {
                resources.push(Resource { client: Some(client) });
            } else {
                resources.push(Resource { client: None });
            }
        }

        // Shuffle the requests using the rng passed as argument
        rng.borrow_mut().shuffle(&mut self.requests);
        
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
}

impl RandomAllocator {
    /// Create a new random allocator
    /// # Parameters
    /// * `args` - The arguments for the allocator
    /// # Returns
    /// * `RandomAllocator` - The new random allocator
    pub fn new(args: AllocatorBuilderArgument) -> RandomAllocator {
        // Check if the arguments are valid
        if args.num_clients <= 0 || args.num_resources <= 0 {
            panic!("Invalid arguments")
        }
        RandomAllocator {
            num_resources: args.num_resources,
            num_clients: args.num_clients,
            requests: Vec::new(),
        }
    }

    /// Check if the request is valid
    /// # Arguments
    /// * `request` - The request to check
    /// # Returns
    /// * `bool` - True if the request is valid, false otherwise
    /// # Remarks
    /// The request is valid if the client is in the range [0, num_clients) and the resource is in the range [0, num_resources)
    fn is_valid_request(&self, _request: &Request) -> bool {
        if _request.client >= self.num_clients || _request.resource >= self.num_resources {
            return false
        }
        true
    }
}