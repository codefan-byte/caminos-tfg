use std::cell::RefCell;
use std::collections::HashMap;

use rand::rngs::StdRng;

//use quantifiable_derive::Quantifiable;//the derive macro
use crate::allocator::{Allocator, AllocatorBuilderArgument, GrantedRequests, Request};
use crate::config_parser::ConfigurationValue;
use crate::match_object_panic;


#[derive(Clone)]
struct IslipRequest {
    port: Option<usize>,
}

/// An iSLIP allocator, more info 'https://doi.org/10.1109/90.769767'
pub struct IslipAllocator {
    /// The max number of inputs ports
    num_clients: usize,
    /// The number of outputs ports
    num_resources: usize,
    /// The number of iterations to perform
    num_iterations: usize,
    /// The input requests
    input_requests: Vec<HashMap<usize, IslipRequest>>,
    /// The output requests
    output_requests: Vec<HashMap<usize, IslipRequest>>,
    /// The input match
    in_match: Vec<Option<usize>>,
    /// The output match
    out_match: Vec<Option<usize>>,
    /// The grant pointers
    g_ptrs: Vec<usize>,
    /// The accept pointers
    a_ptrs: Vec<usize>,
}

impl IslipAllocator {
    /// Creates a new iSLIP allocator
    /// # Arguments
    /// * `args` - The arguments of the allocator
    /// # Returns
    /// * `IslipAllocator` - The new iSLIP allocator
    pub fn new(args: AllocatorBuilderArgument) -> IslipAllocator {
        // Check if the arguments are valid
        if args.num_clients <= 0 || args.num_resources <= 0 {
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

        let input_requests = vec![HashMap::new(); args.num_clients];
        let output_requests = vec![HashMap::new(); args.num_resources];
        let in_match = vec![None; args.num_clients];
        let out_match = vec![None; args.num_resources];
        let g_ptrs = vec![0; args.num_clients];
        let a_ptrs = vec![0; args.num_resources];
        IslipAllocator {
            num_clients: args.num_clients,
            num_resources: args.num_resources,
            num_iterations,
            input_requests,
            output_requests,
            in_match,
            out_match,
            g_ptrs,
            a_ptrs,
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
    fn add_request(&mut self, request: Request) {
        // Check if the request is valid
        if !self.is_valid_request(&request) {
            panic!("The request is not valid");
        }
        // Check if the request is already in the allocator
        // FIXME: (Paranoia Mode ON)
        //  Maybe we din't need to check this, because the router will not send the same request twice
        if self.input_requests[request.client].contains_key(&request.resource) {
            panic!("The request is already in the input requests");
        }
        if self.output_requests[request.resource].contains_key(&request.client) {
            panic!("The request is already in the output requests");
        }
        let in_port = request.client;
        let out_port = request.resource;

        // Create the iSLIP request
        let mut req = IslipRequest {
            port: Some(out_port),
        };

        // add the request to the input requests
        self.input_requests[in_port].insert(out_port, req.clone());
        // add the request to the output requests
        req.port = Some(in_port);
        self.output_requests[out_port].insert(in_port, req);
    }

  

    //         Create the granted requests vector and return it
    //         let mut gr = GrantedRequests {
    //             granted_requests: vec![],
    //         };
    //         for output in 0..self.num_resources {
    //             if grants[output].is_some() {
    //                 let req = Request {
    //                     client: grants[output].unwrap(),
    //                     resource: output,
    //                     priority: Some(0),
    //                 };
    //                 gr.granted_requests.push(req);
    //             }
    //         }
    //         // TODO: Clean up things!!
    //         let mut gr = GrantedRequests {
    //             granted_requests: vec![],
    //         };
    //         return gr;
    //     }
    // }

    /// Perform the allocation
    /// # Arguments
    /// * `_rng` - NOT USED on this allocator
    /// # Returns
    /// * `GrantedRequests` - The granted requests
    fn perform_allocation(&mut self, _rng: &RefCell<StdRng>) -> GrantedRequests {
        // the input port
        let mut input: usize;
        // the output port
        let mut output: usize;
        // the input offset
        let mut input_offset: usize;
        // the output offset
        let mut output_offset: usize;
        // the iterator for the input/output requests lists
        let mut iter;
        // the wrapped flag
        // is used to check if we have wrapped around the list
        let mut wrapped;
        // the granted requests vector
        let mut grants = vec![None; self.num_resources];

        for islip_iter in 0..self.num_iterations {
            // Grant phase
            for output in 0..self.num_resources {
                // skip if the output is already matched OR if there is no request for the output
                if self.out_match[output].is_some() || self.output_requests[output].is_empty() {
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
    }

    fn support_intransit_priority(&self) -> bool {
        false
    }
}
