# Change Log

## next? [0.2.0]

### 2020-11-26
Added methods `Routing::{statistics,reset_statistics}` and `Router::{aggregate_statistics,reset_statistics}` to gather specific statistics of each type.
Added routing annotations.
Added method `Routing::performed_request` to allow routings to make decisions when the router makes a request to a candidate.
Implemented a Stubborn meta routing, what always repeat the same request over and over.
Added `SumRoutingPolicy::TryBoth`.
git commit -m "Added statistics to routings and routers. Routers now inform routings of the candidate they finally request."
git commit -m "Divided occpation in statistics by number of ports."
git commit -m "Added extra label parameter to SumRouting."

## [0.1.0] 

### 2020-11-24
git tag 0.1.0 -m "v0.1.0"
git commit -m "Updated metdata for publication."

### 2020-11-23
Changed `Topology::coordinated_routing_record` to optionally receive a random number generator.
The torus topology now uses the random number generator to generate fair routing records to the opposing location for even sides.
git commit -m "Balanced routng records for torus."

### 2020-11-19

Implemented `{Mesh,Torus}::diameter`.
git commit -m "Provided diameter for meshes and tori."
New member `CandidateEgress::router_allows: Option<bool>` to capture whether the router consider the egress to satisfy the flow-control.
Moved pre-request checking of flow-control into a new `EnforceFlowControl` policy.
git commit -m "Moved pre-request checking of flow-control into a new EnforceFlowControl policy."

### 2020-11-18

git commit -m "Added slimfly and proyective topologies for prime fields."

### 2020-11-12

Some fixes for topologies with non-connected ports.
Got the projective topology implemented.
Also implemented the LeviProjective and the SlimFly topologies.
`Topology::check_adjacency_consistency` now also optionally checks a given number of link classes.
Added Quantifiable to `[T;2]`.

### 2020-11-11

More documentation.
Code cleanup.
git commit -m "First commit in the new caminos-lib git."
Begining to write the projective networks.

### 2020-11-09

Created repository `caminos-lib` with content copied from a private version.
Split into `caminos-lib` and `caminos`.
Created CHANGELOG.md and README.md
And using now edition=2018.

