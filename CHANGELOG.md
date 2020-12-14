# Change Log

## next? [0.2.0]

### 2020-12-14
git commit -m "Enabled tikz externalization. Let main.cfg handle close."

### 2020-12-11
Enabled tikz externalization.
Added a prefix member to Plots.

### 2020-12-10
Added `ExperimentOptions::message`, intended to be used with `--message=text`, to be written into the journal file.
Removed unnecessary mut requirement of `Experiment::write_journal_entry`.
Removed quotes from the config `LitStr` and `Literal`.
git commit -m "Added shifts to CartesianTransform. Added a message option. Removed surrounding quotes of parsed literals."
git commit -m "Actually removed quoted from compiled grammar."
git commit -m "Added quotes when printing literals."
git commit -m "Removed quotes around git_id when building a literal."
Added enum BackendError and improved error managing on output generation.

### 2020-12-09
Added shift argument to CartesianTransform.
Renamed CartesianTornado as CartesianFactor. I messed up, this is not a generalization of tornado but something else entirely. The tornado pattern is just a shift by `(side-1)*0.5`, which can be written as `CartesianTransform{sides:[whole],shift:[halfside]}`, with `whole=side*side` and `halfside=(side-1)/2`.
Added `O1TURN::reserved_virtual_channels_order{01,10}` parameters to control the usage of virtual channels.

### 2020-12-07
Implemented the CartesianTornado pattern.
git commit -m "Implemented the CartesianTornado pattern."

### 2020-12-04
Added patterns `Composition` and `Pow`.
git commit -m "Added neighbour_router_iter to topologies to avoid misusing degree. Added patterns Composition and Pow."

### 2020-12-03
Ordering code on NeighboursLists.
Added `Topology::{write_adjacencies_to_file,neighbour_router_iter}`.
Removed `non_exhaustive` from TopologyBuilderArgument.
Use `neighbour_router_iter` always instead of `0..degree()`. `degree`  does not give valid ranges when having non-connected ports.

### 2020-12-01
Added the config if, add functions.
Allow to use "legend_name" directly in the simulation config root. This helps to use named experiment indices.
git commit -m "Added configuration functions"

### 2020-11-30
Added to the grammar function calls. To be used as "=functionname{key1:expr1, key2:expr2,}".
Added the config function `lt`.
Added member `ExperimentOptions::where_clause` to receive --where parameters.
The output::evaluate funtion made public.
Added `config_parser::parse_expression` to be used to capture the --where=expr clause.
git commit -m "Improved grammar: added named experiments and function calls."
New file config.rs to encapsulate all the processing of ConfigurationValue and expressions.
git commit -m "Moved config-processing aspects into new file config.rs."
Fixed bugs while calculating and showing statistics of the Basic router.
git commit -m "Fixed bugs while calculating and showing statistics of the Basic router."
Set pgfplots option `scaled ticks=false`.
Added Plotkind option `array`. It differs from histogram in that it does not normalize.

### 2020-11-27
Added 'named experiments' to the grammar. This is, to use `key: expa![val1,val2,val3]` and in other place `other_key: expa![wok1,wok2,wok3]`. Intended to get the matches `[{key:val1,other_key_wok1},{key:val2,other_key_wok2},{key:val3,other_key_wok3}]` instead of the whole of combinations.
Changed `flatten_configuration_value` to expand named experiments correctly.

### 2020-11-26
Added methods `Routing::{statistics,reset_statistics}` and `Router::{aggregate_statistics,reset_statistics}` to gather specific statistics of each type.
Added routing annotations.
Added method `Routing::performed_request` to allow routings to make decisions when the router makes a request to a candidate.
Implemented a Stubborn meta routing, what always repeat the same request over and over.
Added `SumRoutingPolicy::TryBoth`.
git commit -m "Added statistics to routings and routers. Routers now inform routings of the candidate they finally request."
git commit -m "Divided occpation in statistics by number of ports."
git commit -m "Added extra label parameter to SumRouting."
git commit -m "Fixed annotation on SumRouting."
git commit -m "More fixes on SumRouting."

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

