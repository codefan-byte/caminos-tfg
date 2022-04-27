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
            //TODO: is that the correct way to inform the user?
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
        //FIXME: igual hay que cambiar el tipo GrantedRequests para que sea un alias de un vector de Request
        let mut gr = GrantedRequests { granted_requests: Vec::new() };

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

                    if grants[resource] == Some (client) {
                        self.out_match[resource] = Some(client);
                        self.in_match[client] = Some(resource);
                        break; // break the inner loop
                    }

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
                        gr.granted_requests.push(req);
                        
                        // only update pointers if accepted during the 1st iteration
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


    /// OLD VERSION
/*     ///  for islip_iter in 0..self.num_iterations {
            // Grant phase
            for output in 0..self.num_resources {
                // if self.out_match[output].is_some() || self.output_requests[output].is_empty() {
                //     continue;
                // }
                
                // skip if the output is already matched OR if there is no request for the output
                if self.out_match[output].is_some() || self.out_requests[output].is_empty() {
                    continue;
                }
                
                // a round-robin arbiter between the input requests
                input_offset = self.g_ptrs[output];

                // The Booksim version, using funny C++ iterators syntax
                // iter = _out_req[output].begin( );
                // while( ( iter != _out_req[output].end( ) ) && ( iter->second.port < input_offset ) ) { iter++; }

                // The Rust version
                iter = self.output_requests[output].iter().peekable();
                while let Some((_, ref req)) = iter.peek() {
                    if req.port < Some(input_offset) {
                        iter.next(); // skip the request
                    } else {
                        break; // found the first request that can be granted
                    }
                }
                wrapped = false;
                while !wrapped || iter.peek().map_or(false, |(k, _)| *k < &input_offset) {
                    if iter.peek().is_none() {
                        if wrapped {
                            break;
                        }
                        // p is valid here because empty lists
                        // are skipped (above)
                        iter = self.output_requests[output].iter().peekable();
                        wrapped = true;
                    }
                    // get the input
                    input = *iter.peek().unwrap().0;
                    // we know the output is free (above) and
                    // if the input is free, grant request
                    if self.in_match[input].is_none() {
                        grants[output] = Some(input);
                        break;
                    }
                    iter.next();
                }
            } // end of the GRANT phase

            // Accept phase
            for input in 0..self.num_clients {
                if self.input_requests[input].is_empty() {
                    continue;
                }
                // a round-robin arbiter between the output requests
                output_offset = self.a_ptrs[input];

                // TODO: This will be the same?
                // while iter.peek().map_or(false, |(k, _)| *k < Some(output_offset) ) {
                //    iter.next();
                // }
                iter = self.input_requests[input].iter().peekable();
                while let Some((_, ref req)) = iter.peek() {
                    if req.port < Some(output_offset) {
                        iter.next(); // skip the request
                    } else {
                        break; // found the first request that can be accepted
                    }
                }
                wrapped = false;
                while !wrapped || iter.peek().map_or(false, |(k, _)| *k < &output_offset) {
                    if iter.peek().is_none() {
                        if wrapped {
                            break;
                        }
                        // iter is valid here because empty lists
                        // are skipped (above)
                        iter = self.input_requests[input].iter().peekable();
                        wrapped = true;
                    }
                    // get the output
                    output = *iter.peek().unwrap().0;

                    // we know the output is free (above) and
                    // if the input is free, grant request
                    if grants[output] == Some(input) {
                        // accept
                        self.in_match[input] = Some(output);
                        self.out_match[output] = Some(input);
                        // only update pointers if accepted during the 1st iteration
                        if islip_iter == 0 {
                            self.g_ptrs[output] = (input + 1) % self.num_clients;
                            self.a_ptrs[input] = (output + 1) % self.num_resources;
                        }
                        break;
                    }
                    iter.next();
                }
            } // end of the ACCEPT phase
        }

        let mut gr = GrantedRequests {
            granted_requests: vec![],
        };
        // fill the granted requests vector
        for output in 0..self.num_resources {
            if grants[output].is_some() {
                let req = Request {
                    client: grants[output].unwrap(),
                    resource: output,
                    priority: Some(0),
                };
                gr.granted_requests.push(req);
            }
        }
        gr
    } */


    fn support_intransit_priority(&self) -> bool {
        false
    }
}
