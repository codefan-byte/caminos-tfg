
use std::cell::RefCell;

use rand::{StdRng, Rng};

//use quantifiable_derive::Quantifiable;//the derive macro
use crate::allocator::{Allocator, Request, GrantedRequests, AllocatorBuilderArgument};

/// A random allocator that randomly allocates requests to resources
pub struct RandomAllocator {
    /// The number of outputs of the router crossbar
    num_resources: usize,
    /// The number of inputs of the router crossbar
    num_clients: usize,
    /// The requests of the clients
    requests: Vec<Request>,
    /// The resources allocated to the clients
    resources: Vec<Resource>,
}

pub struct Resource {
    /// Index of the client that has the resource (or None if the resource is free)
    client: Option<usize>,
}

impl Allocator for RandomAllocator {
    fn num_clients(&self) -> usize {
        self.num_clients
    }

    fn num_resources(&self) -> usize {
        self.num_resources
    }

    fn add_request(&mut self, _request: &Request) {
        // Check if the request is valid
        if !self.is_valid_request(_request) {
            panic!("Invalid request");
        }
        // TODO: How can I add the request directly to the requests vector?
    //    self.requests.push(*_request);
        self.requests.push(Request{
            client: _request.client,
            resource: _request.resource,
            priority: _request.priority,
        });
    }

    fn perform_allocation(&mut self, rng : &RefCell<StdRng>) -> GrantedRequests {
        // Create the granted requests vector
        let mut gr = GrantedRequests { granted_requests: Vec::new() };

        // Shuffle the requests
        rng.borrow_mut().shuffle(&mut self.requests);
        
        // Allocate the requests with an iterator
        for Request{ref resource, ref client, ref priority } in self.requests.iter() {
            // Check if the wanted resource is available
            if self.resources[resource.to_owned()].client.is_none() {
                // Add the request to the granted requests
                gr.granted_requests.push(Request{
                    client: client.to_owned(),
                    resource: resource.to_owned(),
                    priority: priority.to_owned(),
                });
                // Allocate the resource
                self.resources[resource.to_owned()].client = Some(client.to_owned());
            } else {
                // The resource is not available, so we can't grant the request
                continue;
            }
        }
        // Clear the requests and resources
        self.requests.clear();
        self.resources.clear();
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
    pub fn new(args: &AllocatorBuilderArgument) -> RandomAllocator {
        // Check if the arguments are valid
        if args.num_clients <= 0 || args.num_resources <= 0 {
            panic!("Invalid arguments");
        }
        RandomAllocator {
            num_resources: args.num_resources,
            num_clients: args.num_clients,
            requests: Vec::new(),
            resources: Vec::new(),
        }
    }

    /// Get the random allocator's num clients.
    /// # Returns
    /// The number of clients
    pub fn num_clients(&self) -> usize {
        self.num_clients
    }
    /// Get the random allocator's num resources.
    /// # Returns
    /// The number of resources
    pub fn num_resources(&self) -> usize {
        self.num_resources
    }
    /// Check if the request is valid
    /// # Arguments
    /// * `request` - The request to check
    /// # Returns
    /// * `bool` - True if the request is valid, false otherwise
    /// # Remarks
    /// The request is valid if the client is in the range [0, num_clients) and the resource is in the range [0, num_resources)
    fn is_valid_request(&self, _request: &Request) -> bool {
        if _request.client == 0 || _request.client >= self.num_clients {
            return false;
        }
        if _request.resource == 0 || _request.resource >= self.num_resources {
            return false;
        }
        true
    }
}