use std::cell::RefCell;
use std::vec;

use rand::rngs::StdRng;

use crate::allocator::{Allocator, AllocatorBuilderArgument, GrantedRequests, Request};
use crate::config_parser::ConfigurationValue;
use crate::match_object_panic;

#[derive(Clone, Debug)]
struct RoundVec {
    /// The client with the highest priority in the round.
    pointer : usize,
    /// Have the indices of the clients that have been requested
    pub clients : Vec<usize>,
    /// The number of clients
    n : usize,
}

impl RoundVec {
    fn new(size : usize) -> RoundVec {
        RoundVec {
            pointer : 0,
            clients : Vec::with_capacity(size),
            n : size,
        }
    }
    /// Add an element to the round vector
    fn add (&mut self, element : usize) {
        self.clients.push(element);
    }
    /// Increment the pointer
    fn increment_pointer(&mut self) {
        self.pointer = (self.pointer + 1) % self.n;
    }
    /// Sort the clients using the pointer as the pivot
    fn sort(&mut self) {
        // We need to extract the pivot and the size because we only can have one mutable reference
        let pointer = self.pointer;
        let size = self.size();
        self.clients.sort_unstable_by_key(| k|
            if *k < pointer {
                *k + size
            } else {
                *k
            }
        );
    }
    /// Get the size of the round vector
    fn size (&self) -> usize {
        self.n
    }
    /// Check if the round vector is empty
    /// # Returns
    /// `true` if the round vector is empty, `false` otherwise
    fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }
}


/// An iSLIP allocator, more info 'https://doi.org/10.1109/90.769767'
pub struct IslipAllocator {
    /// The max number of inputs ports
    num_clients: usize,
    /// The number of outputs ports
    num_resources: usize,
    /// The number of iterations to perform
    num_iterations: usize,
    /// The input match
    in_match: Vec<Option<usize>>,
    /// The output match
    out_match: Vec<Option<usize>>,
    /// The input requests (RoundVec)
    in_requests: Vec<RoundVec>,
    /// The output requests (RoundVec)
    out_requests: Vec<RoundVec>,
}

impl IslipAllocator {
    /// Creates a new iSLIP allocator
    /// # Arguments
    /// * `args` - The arguments of the allocator
    /// # Returns
    /// * `IslipAllocator` - The new iSLIP allocator
    pub fn new(args: AllocatorBuilderArgument) -> IslipAllocator {
        // Check if the arguments are valid
        if args.num_clients == 0 || args.num_resources == 0 {
            panic!("Invalid arguments for IslipAllocator");
        }
        // Get the number of iterations to perform
        let mut num_iterations = None;
        match_object_panic!(args.cv, "islip", value,
        "num_iter" => match value
        {
            &ConfigurationValue::Number(i) => num_iterations = Some(i as usize),
            _ => panic!("Bad value for num_iter"),
        }
        );
        if num_iterations.is_none() {
            // Warn the user that the default value will be used
            println!("Warning: num_iter for the iSLIP allocator is not specified in the configuration file, the default value (1) will be used");
        }
        let num_iterations = num_iterations.unwrap_or(1);
        let in_match = vec![None; args.num_clients];
        let out_match = vec![None; args.num_resources];
        IslipAllocator {
            num_clients: args.num_clients,
            num_resources: args.num_resources,
            num_iterations,
            in_match,
            out_match,
            in_requests: vec![RoundVec::new(args.num_resources); args.num_clients],
            out_requests: vec![RoundVec::new(args.num_clients); args.num_resources],
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
            return false;
        }
        true
    }
}

impl Allocator for IslipAllocator {
    /// Add a request to the allocator
    /// # Arguments
    /// * `request` - The request to add
    /// # Remarks
    /// The request is valid if the client is in the range [0, num_clients) and the resource is in the range [0, num_resources)
    /// Asume that the request is is not already in the allocator
    /// # Panics
    /// If the request is not valid
    fn add_request(&mut self, request: Request) {
        // Check if the request is valid
        if !self.is_valid_request(&request) {
            panic!("The request is not valid");
        }
        
        // The in_requests and out_requests are indexed by the client/resource respectively
        // We need to get the index of the client/resource
        let client = request.client;
        let resource = request.resource;

        // Add the request to the input requests and output requests
        // Asume that the request is not already in the requests vectors
        self.in_requests[client].add(resource);
        self.out_requests[resource].add(client);
    }

    /// Perform the allocation
    /// # Arguments
    /// * `_rng` - NOT USED on this allocator
    /// # Returns
    /// * `GrantedRequests` - The granted requests
    fn perform_allocation(&mut self, _rng: &RefCell<StdRng>) -> GrantedRequests {

        // Create the granted requests vector
        let mut gr = GrantedRequests::default();
        
        // Sort the input requests
        for client in 0..self.num_clients {
            self.in_requests[client].sort();
        }
        // Sort the output requests
        for resource in 0..self.num_resources {
            self.out_requests[resource].sort();
        }

        // Reset the in_match and out_match vectors
        for client in 0..self.num_clients {
            self.in_match[client] = None;
        }
        for resource in 0..self.num_resources {
            self.out_match[resource] = None;
        }
        
        for islip_iter in 0..self.num_iterations {
            // the granted requests vector
            // (Indexed by the resource)
            let mut grants = vec![None; self.num_resources];

            // Grant phase
            for resource in 0..self.num_resources {
                // skip if the output is already matched OR if there is no any request for the output
                if self.out_match[resource].is_some() || self.out_requests[resource].is_empty() {
                    continue;
                }
                
                for request in &self.out_requests[resource].clients {
                    let client = *request;
                    // know that the output is not matched (see above) and 
                    // if the input is free (no match) then grant the request
                    if self.in_match[client].is_none() {
                        grants[resource] = Some(client);
                        break; // break the inner loop
                    }
                }
            } // end of the GRANT phase

            // Accept phase
            for client in 0..self.num_clients {
                // skip if the there is no any request for the input
                if self.in_requests[client].is_empty() {
                    continue;
                }
                
                for request in &self.in_requests[client].clients {
                    let resource = *request;

                    // we know the output is free (above) and
                    // if the input is free, grant request
                    if grants[resource] == Some(client) {
                        // accept
                        self.in_match[client] = Some(resource);
                        self.out_match[resource] = Some(client);
                        // add the request to the granted requests
                        let req = Request {
                            client,
                            resource,
                            priority: Some(0),
                        };
                        gr.add_granted_request(req);
                        
                        // only update pointers if accepted during the 1st iteration
                        // (This is to avoid starvation, see the iSLIP paper)
                        if islip_iter == 0 {
                            // update the input/output requests lists pointers
                            self.in_requests[client].increment_pointer();
                            self.out_requests[resource].increment_pointer();
                        }
                        break;
                    }
                }
            } // end of the ACCEPT phase
        } // end of the ITERATIONS phase
        // return the granted requests
        gr
    }

    fn support_intransit_priority(&self) -> bool {
        false
    }
}
