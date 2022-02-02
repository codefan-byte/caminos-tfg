# Change Log

## next [0.3.2]

### 2022-02-02
Added the MapEntryVC meta-policy to build rules dependant on the virtual channel with which the packet entered the router.
git commit -m "MapEntryVC policy, div config function, and ordinate_post_expression field."

### 2022-01-31
Added div config function.
Added `ordinate_post_expression`to `Plotkind`.
Changed sbatch job name to CAMINOS.

### 2022-01-12
New pattern ConstantShuffle.
git commit -m "New pattern ConstantShuffle."

### 2021-12-16
New meta routing option `SumRoutingPolicy::EscapeToSecond`.
New `VirtualChannelPolicy::{ArgumentVC,Either}`.
git commit -m "Added an escape policy"
git commit -m "Added the Either channel policy to keep candidates satisfying any of several policies."

### 2021-12-09
Read remote binary.results when initializing remote.
Pull now try first to pull from binary.results.
Added config functions `slice`, `sort`, `last`, `number_or`, and `filter`.
Stop using tikz symbolic coordinates and use instead just natural coordinates with textual labels.
Improved the code to manage the plots.
Plots requiring symbols can now use absicssa limits.
git commit -m "Added several config functions and latex output improvements."
git commit -m "Added Diplay for FunctionCall expressions."

### 2021-12-07
Avoid making the runx directories when they are not required.
Added action `Pack`, to pack current results into binary.results and delete the raw ones.
git commit -m "New action pack"
git commit -m "moved a canonicalize out of the main path to avoid requiring the runx directories."
git commit -m "Added a canonicalize to the parent runs path"
git commit -m "bugfix on packet statistics: only track the leading phit of packets."
Some fixes to detect non-numbers before averaging.
Added `latex_make_symbol` to protect symbolic coordinates.

### 2021-12-04
Added `PacketExtraInfo` to `Packet` to store additional statistics for `statistics_packet_definitions`.
git commit -m "Added to statistics_packet_definitions the members link_classes, entry_virtual_channels, and cycle_per_hop"
git commit -m "The stat entry_virtual_channels now sets None value when a VC was not forced, as from the server"
git commit -m "Changed NONE VALUE to None"

### 2021-12-03
Removed an underflow when averaging consumption queues of the server in the Basic router.
New policy `MapHop` that applies a different policy to each hop number.
git commit -m "Added MapHop policy and diff"
Added user definied statistics for consumed packets. Define with `configuration.statistics_packet_definitions` and receive into`result.packet_defined_statistics`.

### 2021-12-01
git commit -m "fixed MapLabel: above and below were swapped in filter."

### 2021-12-01
Added crate `diff` to the dependencies.
Show differences on the configurations when there are any with the remote file.

### 2021-11-30
Added config functions `map` and `log`.

### 2021-11-29
git commit -m "relaxed Topology::check_adjacency_consistency for non-regular topologies."
git commit -m "Implemented distance method for Mesh topology."

### 2021-11-26
Fixed entry `ShiftEntryVC` on `new_virtual_channel_policy`.
git commit -m "fixed entry on new_virtual_channel_policy"
git commit -m "Added information on new_virtual_channel_policy panic"

### 2021-11-25
One point is enough is bar/boxplot graphs to consider them good plots.
Added `VirtualChannelPolicy::{Identity,MapLabel,ShiftEntryVC}`.
Breaking change: Added requirement `VirtualChannelPolicy: Debug`.
git commit -m "Policies are now required to implement Debug. New policies Identity, MapLabel and ShiftEntryVC."

### 2021-11-22
git commit -m "return from routings changed to RoutingNextCandidates and added idempotence checks."

### 2021-11-18
Refer to `texlive-pictures` in the README.md.
Adding `Action::Shell`.
Added documentation to `output.rs` and made it public to actually have docs to be generated.
git commit -m "boxplots, preprocsessing output files, improvements on documentations, shell action, and more."
Breaking change: routings now return `RoutingNextCandidates`. In addition to the vector of candidates it contains an `idempotent` field to allow some checks and optimizations.
Basic router now check idempotence of the routing to panic when there are no candidates for some packet.

### 2021-11-17
Added `Sequence` traffic.
New policy `SumRoutingPolicy::SecondWhenFirstEmpty` to complete a routing with another when the first does not find any candidates.

### 2021-11-10
Made `preprocessArgMax` work with incomplete data.
Fixed a bit the documentation syntax.

### 2021-11-01
Added preprocessing outputs: `PreprocessArgMax`.
New config functions `mul` and `FileExpression`.
Added `path` argument to `config::{evaluate,reevaluate}`.
File `create_output` and similar now receive in its `results` argument also the experiment indices.

### 2021-10-28
Improved style of Box Plots.

### 2021-10-28
Added option to generate Box Plots.

### 2021-10-27
Added `Statistics.server_percentiles` and configuration `statistics_server_percentiles` to generate in the result file fields such as `server_percentile25` with values of the server in the given percentile.
git commit -m "Added statistics_server_percentiles"
Added `Statistics.{packet_percentiles,packet_statistics}`, struct StatisticPacketMeasurement and configuration `statistics_packet_percentiles` to generate per packet statistics percentile data.
git commit -m "Added statistics_packet_percentiles"
Protect some latex labels.

## [0.3.1]

### 2021-10-19
Updated readme to say 0.3 and `pgfplots`.
Canonicalize path before extracting folder name, to work when a dot is given as path.
Cargo.toml version to 0.3.1.
git tag 0.3.1 -m "v0.3.1"
git commit -m "version update to 0.3.1, fixing using dot as path."

## [0.3.0]

### 2021-10-19
Fixed example configuration in the readme.
git commit -m "Support for bar graphs. meta routing EachLengthSourceAdaptiveRouting. readme fixes."
git tag 0.3.0 -m "v0.3.0"
git commit -m "version update to 0.3.0"
git commit -m "update to 0.3.0 in Cargo.toml"

### 2021-09-17
Implemented `EachLengthSourceAdaptiveRouting` as source routing storing a path of each length.

### 2021-07-16
Added styles for bars.

### 2021-07-15
Updated merge functionality to work with binary results.
Added `eq | equal` config evaluation function.
Added support for symbolic abscissa.

### 2021-07-07
git commit -m "Added hop estimation to Shortest and Valiant candidates."

### 2021-07-05
Set `estimated_remaining_hops` in `SourceRouting`.
Added `use_estimation` to `LowestSinghWeight`.
git commit -m "Generate hop estimations in SourceRouting and use them in LowestSinghWeight"
git commit -m "Enhancing LowestSinghWeight with things in OccupancyFunction"
New pseudo-routing wrapper `SourceAdaptiveRouting`.
git commit -m "New wrapper for source adaptive routings."

### 2021-05-21
git commit -m "fixes on using binary results"

### 2021-05-20
Added `already` count to progress bar message.
Fixed detection of results in binary format.

### 2021-05-18
Read results from binary.results.
Changed in `config_from_binary` things from `usize` to `u32` to clear sizes in binary format.
Pull remote results into memory and then into binary.results, instead of copying the local files.
git commit -m "Pack results into a bianry file"

### 2021-05-12
Implemented `config_to_binary`, `config_from_binary`, and BinaryConfigWriter. Tested to create them, remains to test loading them.

### 2021-05-10
Added field `Packet::cycle_into_network` to allow some additional statistics.
Removed `track_packet_hops` and added functionality to `track_consumed_packet`.
Added `average_packet_network_delay` to statistics at several levels.
git commit -m "Added network delay statistics per packet."

### 2021-05-08
SumRouting attributes converted into arrays to allow indexing.
Split SumRouting policy `TryBoth` into `TryBoth`, `Stubborn`, and `StubbornWhenSecond`.
git commit -m "stubborn policies on SumRouting."

### 2021-05-07
Space marks with tikz backend only when there are many points in drawing range.

### 2021-05-05
git commit -m "Added initialize recursion to Stubborn routing."

### 2021-05-04
git commit -m "Added performed_request recursion to Stubborn routing."

### 2021-05-03
git commit -m "Fixed a bug on the allowed virtual channels in SumRouting."

### 2021-04-30
Added `{min,max}_abscissa` to Plotkind.
Make AverageBins return NANs instead of panicing.
Automatically add `mark repeat` when having too many points within the tikz backend.
Fixed tracking temporal stastistics of given hops and message delay.
git commit -m "Fixes and improvemets for temporal statistics."

### 2021-04-23
git commit -m "Bugfix on WeighedShortest. New routing transformations related to virtual channels."
Removed `non_exhaustive` for Builders.

### 2021-04-22
Added routing `ChannelsPerHopPerLinkClass` and `AscendantChannelsWithLinkClass` and `ChannelMap`.
Routing `WeighedShortest` made to verify the selected link actually belong to the shortest route.
Implemented nesting of `Valiant` routing initialization.

### 2021-04-20
Added routing `ChannelsPerHop`.
git commit -m "Updated grammar tech to manage large files. New routing ChannelsPerHop."

### 2021-04-16
Removed grammar warning.
Use public gramatica-0.2.0.

### 2021-04-15
Updates in grammar technology.
Added `ConfigurationValue::None` to be able to implement `Default` and use `std::mem::take`.

### 2021-04-08
Messing with the grammar to avoid cloning values.
New configuration function `AverageBins`.

### 2021-04-07
Trying experimental gramatica-0.1.6 to solve the stack overflow.

### 2021-03-30
Debugging a stack overflow...

### 2021-03-29
Changed default statistic jain column to ServerGenerationJainIndex.
New traffic TimeSequenced.
Added parameter `cycle` to `Traffic::should_generate`.
Split StatisticMeasurement from the Statistics struct.
Added support to temporal statistics via `statistics_temporal_step`.
git commit -m "Improvements on Valiant routing, matrices, traffics, and statistics. Implemented optional tracking of statistics per cycle."

### 2021-03-26
Documentation improvements.
Derive Debug for RequestInfo.

### 2021-03-23
Documentation fix.

### 2021-03-22
Starting with ExplicitUpDown: implemented UpDownStar construct.
Added methods `Matrix::{get_rows,get_columns}`.

### 2021-03-18
Added to Valiant routing the optional parameters `{first,second}_reserved_virtual_channels: Vec<usize>` to easy defining a Valiant over MultiStage topologies using UpDown first with some virtual channel and later with other.

### 2021-03-18
Removed some `dbg!` statements from MultiStage.
git commit -m "Removed some debug statements."
Added optional parameter `selection_exclude_indirect_routers` to Valiant routing.
Added warning message when generating traffic over a different amount of servers than the whole of the topology.

### 2021-03-15
Converting `MultiStage::up_down_distances` from `Vec<Vec<Option<(usize,usize)>>>` into `Matrix<Option<(u8,u8)>>`.
Added `Matrix::map` to ease working with matrices over different types.
Converting `MultiStage::flat_distance_matrix` from `Matrix<usize>` into `Matrix<u8>`.
git commit -m "Reduced memory usage of multistage topologies."
Converted `dragonfly::distance_matrix` to `u8`.

## [0.2.0]

### 2021-03-12
git commit -m "Preparing to publish version 0.2."
git commit -m "Track multistage.rs"

### 2021-03-10
Added plugs for stages.
Attributes of `LevelRequirements` made public.
Removed from the `Topology` interfaz the never used methods `num_arcs`, `average_distance`, `distance_distribution`.
git commit -m "Added multistage topologies. Cleanup on Topology interfaz."
Added method `up_down_distance` to `Topology`.
Splitting up/down distance table in MultiStage into a up-component and a down-component. Removed its pure up-distance table.
New routing `UpDown`.
Replaced several `.expect(&format!(...))` by `.wrap_or_else(|_|panic!(...))`, to avoid formatting strings except when reporting errors.
Added a bit of documentation.

### 2021-03-09
Changed `WidenedStage` to use a boxed `base` as to be able to build it.
Added a `new` method to each stage.

### 2021-03-05
MultiStage sizes computed via LevelRequirements.
New stages ExplicitStage, WidenedStage.

### 2021-03-03
New file multistage.rs definining MultiStage topologies in terms of Stages connecting pairs of levels of routers.
Projective types Geometry, SelfDualGeometry, FlatGeometry, and FlatGeometryCache made public. And used in multistage for the OFT.
Added requirement FlatGeometry:Clone.
Implemented stages FatStage and ProjectiveStage, upon which the topologies XGFT and OFT are built.

### 2021-03-02
git tag 0.2.0 -m "v0.2.0"
git commit -m "tag to v0.2"

### 2021-02-12
Updating documentation.
Set version 0.2.0 in Cargo.toml.

### 2021-02-09
git commit -m "implemented Hotspots and RandomMix patterns."
Added the `at` config-function to access arrays in output description files.
git commit -m "Fix on RandomMix probability. Added the at config-function."

### 2021-02-03
git commit -m "Self-messages in Burst traffics now substract a pending message, allowing completion when there are fixed points in the pattern."

### 2021-02-01
Fixed 2021 dates in this changelog...
Correctly manage self-messages in burst traffic.
git commit -m "Correctly manage self-messages in burst traffic. Improvements on tikz backend."

### 2021-01-28
Completed `Stubborn::update_routing_info`, which had the recursion over its sub-routing missing.
git commit -m "Fixed Stubborn routing. Show journal messages."
Moved tikz back externalization plots from `externalization` to `externalization-plots`.
Protected the tikz backend externalization against some collisions.

### 2021-01-27
Added a `cycle` value to the result files. For the sake of burst simulations.
git commit -m "Added cycle to the result files."
Show journal messages with every action.

### 2021-01-25
Added dependence on crate procfs.
Report status.vmhwm, stat.utime, and stat.stime in the result.
git commit -m "Report process status at the end. Improved style of the tikz backend."

### 2021-01-12
A few color changes in the tikz backend.

### 2020-12-22
Added more colors, pens, and marks to tikz backend.

### 2020-12-21
Fixed a bug in ValiantDOR where the DOR part were sometimes non-minimal.
git commit -m "Fixed a bug in ValiantDOR where the DOR part were sometimes non-minimal."

### 2020-12-18
Added check to detect overflowing output buffers.
git commit -m "Added check to detect overflowing output buffers."

### 2020-12-16
Externalization fixes.

### 2020-12-15
Externalization of legends moved to a different folder.
Fixed bubble to actually reserve space for the current packet plus a maximum packet size.
git commit -m "Fixed bubble to actually reserve space for the current packet plus a maximum packet size."

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

