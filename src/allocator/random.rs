
use quantifiable_derive::Quantifiable;//the derive macro
use crate::allocator::{Allocator, Request, GrantedRequests, AllocatorBuilderArgument};

#[derive(Quantifiable)]
pub struct RandomAllocator {
    /// The number of outputs of the router crossbar
    num_resources: usize,
    /// The number of inputs of the router crossbar
    num_clients: usize,
}

impl Allocator for RandomAllocator {
    fn num_clients(&self) -> usize {
        self.num_clients
    }

    fn num_resources(&self) -> usize {
        self.num_resources
    }

    fn add_request(&mut self, _request: Request) {
        todo!();
    }

    fn perform_allocation(&mut self) -> GrantedRequests {
        todo!();
    }
}

impl RandomAllocator {
    pub fn new(args: AllocatorBuilderArgument) -> RandomAllocator {
        RandomAllocator {
            num_resources: args.num_resources,
            num_clients: args.num_clients,
        }
    }
}