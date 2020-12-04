
use std::cell::{RefCell};
use ::rand::{Rng,StdRng};
use std::fs::File;
use std::io::{BufRead,BufReader};
use quantifiable_derive::Quantifiable;//the derive macro
use crate::config_parser::ConfigurationValue;
use crate::topology::cartesian::CartesianData;//for CartesianTransform
use crate::topology::{Topology,Location};
use crate::quantify::Quantifiable;
use crate::Plugs;

///A `Pattern` describes how a set of entities decides destinations into another set of entities.
///The entities are initially servers, but after some operators it may mean router, rows/columns, or other agrupations.
///The source and target set may be or not be the same. Or even be of different size.
///Thus, a `Pattern` is a generalization of the mathematical concept of function.
pub trait Pattern : Quantifiable + std::fmt::Debug
{
	//Indices are either servers or virtual things.
	///Fix the input and output size, providing the topology and random number generator.
	///Careful with using toology in sub-patterns. For example, it may be misleading to use the dragonfly topology when
	///building a pattern among groups or a pattern among the ruters of a single group.
	///Even just a pattern of routers instead of a pattern of servers can lead to mistakes.
	///Read the documentation of the traffic or meta-pattern using the pattern to know what its their input and output.
	fn initialize(&mut self, source_size:usize, target_size:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>);
	///Obtain a destination of a source. This will be called repeteadly as the traffic requires destination for its messages.
	fn get_destination(&self, origin:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)->usize;
}

///The argument to a builder funtion of patterns.
#[non_exhaustive]
#[derive(Debug)]
pub struct PatternBuilderArgument<'a>
{
	///A ConfigurationValue::Object defining the pattern.
	pub cv: &'a ConfigurationValue,
	///The user defined plugs. In case the pattern needs to create elements.
	pub plugs: &'a Plugs,
}


///Build a new pattern.
pub fn new_pattern(arg:PatternBuilderArgument) -> Box<dyn Pattern>
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=arg.cv
	{
		match arg.plugs.patterns.get(cv_name)
		{
			Some(builder) => return builder(arg),
			_ => (),
		};
		match cv_name.as_ref()
		{
			"Uniform" => Box::new(UniformPattern::new(arg)),
			"RandomPermutation" => Box::new(RandomPermutation::new(arg)),
			"RandomInvolution" => Box::new(RandomInvolution::new(arg)),
			"FileMap" => Box::new(FileMap::new(arg)),
			"Product" => Box::new(ProductPattern::new(arg)),
			"Components" => Box::new(ComponentsPattern::new(arg)),
			"CartesianTransform" => Box::new(CartesianTransform::new(arg)),
			"Composition" => Box::new(Composition::new(arg)),
			"Pow" => Box::new(Pow::new(arg)),
			_ => panic!("Unknown pattern {}",cv_name),
		}
	}
	else
	{
		panic!("Trying to create a Pattern from a non-Object");
	}
}

///Each destination request will be uniform random among all the range `0..size` minus the `origin`.
///Independently of past requests, decisions or origin.
///TODO: for some meta-patterns it would be useful to allow self-messages.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct UniformPattern
{
	size: usize,
}

//impl Quantifiable for UniformPattern
//{
//	fn total_memory(&self) -> usize
//	{
//		return size_of::<Self>();
//	}
//	fn print_memory_breakdown(&self)
//	{
//		unimplemented!();
//	}
//	fn forecast_total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//}

impl Pattern for UniformPattern
{
	fn initialize(&mut self, source_size:usize, target_size:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
		self.size=target_size;
		if source_size!=target_size
		{
			unimplemented!("Different sizes are not yet implemented for UniformPattern");
		}
	}
	fn get_destination(&self, origin:usize, _topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)->usize
	{
		//let mut rng = thread_rng();//FIXME use seed
		loop
		{
			let r=rng.borrow_mut().gen_range(0,self.size);
			if r!=origin
			{
				return r;
			}
		}
	}
}

impl UniformPattern
{
	fn new(arg:PatternBuilderArgument) -> UniformPattern
	{
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Uniform"
			{
				panic!("A UniformPattern must be created from a `Uniform` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"legend_name" => (),
					//"pattern" => pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					//"servers" => match value
					//{
					//	&ConfigurationValue::Number(f) => servers=Some(f as usize),
					//	_ => panic!("bad value for servers"),
					//}
					//"load" => match value
					//{
					//	&ConfigurationValue::Number(f) => load=Some(f as f32),
					//	_ => panic!("bad value for load"),
					//}
					//"message_size" => (),
					_ => panic!("Nothing to do with field {} in UniformPattern",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a UniformPattern from a non-Object");
		}
		UniformPattern{
			size:0,
		}
	}
}

///Build a random permutation on initialization, which is then kept constant.
///This allows self-messages; with a reasonable probability of having one.
///See `RandomInvolution` and `FileMap`.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct RandomPermutation
{
	permutation: Vec<usize>,
}

//impl Quantifiable for RandomPermutation
//{
//	fn total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//	fn print_memory_breakdown(&self)
//	{
//		unimplemented!();
//	}
//	fn forecast_total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//}

impl Pattern for RandomPermutation
{
	fn initialize(&mut self, source_size:usize, target_size:usize, _topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		if source_size!=target_size
		{
			panic!("In a permutation source_size({}) must be equal to target_size({}).",source_size,target_size);
		}
		self.permutation=(0..source_size).collect();
		//let mut rng = thread_rng();//FIXME use seed
		rng.borrow_mut().shuffle(&mut self.permutation);
	}
	fn get_destination(&self, origin:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)->usize
	{
		self.permutation[origin]
	}
}

impl RandomPermutation
{
	fn new(arg:PatternBuilderArgument) -> RandomPermutation
	{
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="RandomPermutation"
			{
				panic!("A RandomPermutation must be created from a `RandomPermutation` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"pattern" => pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in RandomPermutation",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a RandomPermutation from a non-Object");
		}
		RandomPermutation{
			permutation: vec![],
		}
	}
}

///Build a random involution on initialization, which is then kept constant.
///An involution is a permutation that is a pairing/matching; if `a` is the destination of `b` then `b` is the destination of `a`.
///It will panic if given an odd size.
///See `Randompermutation`.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct RandomInvolution
{
	permutation: Vec<usize>,
}

//impl Quantifiable for RandomInvolution
//{
//	fn total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//	fn print_memory_breakdown(&self)
//	{
//		unimplemented!();
//	}
//	fn forecast_total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//}

impl Pattern for RandomInvolution
{
	fn initialize(&mut self, source_size:usize, target_size:usize, _topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		if source_size!=target_size
		{
			panic!("In a permutation source_size({}) must be equal to target_size({}).",source_size,target_size);
		}
		//self.permutation=(0..source_size).collect();
		//rng.borrow_mut().shuffle(&mut self.permutation);
		self.permutation=vec![source_size;source_size];
		//for index in 0..source_size
		//{
		//	if self.permutation[index]==source_size
		//	{
		//		//Look for a partner
		//	}
		//}
		assert!(source_size%2==0);
		//Todo: annotate this weird algotihm.
		let iterations=source_size/2;
		let mut max=2;
		for _iteration in 0..iterations
		{
			let first=rng.borrow_mut().gen_range(0,max);
			let second=rng.borrow_mut().gen_range(0,max-1);
			let (low,high) = if second>=first
			{
				(first,second+1)
			}
			else
			{
				(second,first)
			};
			let mut rep_low = max-2;
			let mut rep_high = max-1;
			if high==rep_low
			{
				rep_high=high;
				rep_low=max-1;
			}
			let mut mate_low=self.permutation[low];
			let mut mate_high=self.permutation[high];
			if mate_low != source_size
			{
				if mate_low==high
				{
					mate_low=rep_high;
				}
				self.permutation[rep_low]=mate_low;
				self.permutation[mate_low]=rep_low;
			}
			if mate_high != source_size
			{
				if mate_high==low
				{
					mate_high=rep_low;
				}
				self.permutation[rep_high]=mate_high;
				self.permutation[mate_high]=rep_high;
			}
			self.permutation[low]=high;
			self.permutation[high]=low;
			max+=2;
		}
	}
	fn get_destination(&self, origin:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)->usize
	{
		self.permutation[origin]
	}
}

impl RandomInvolution
{
	fn new(arg:PatternBuilderArgument) -> RandomInvolution
	{
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="RandomInvolution"
			{
				panic!("A RandomInvolution must be created from a `RandomInvolution` object not `{}`",cv_name);
			}
			for &(ref name,ref _value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"pattern" => pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in RandomInvolution",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a RandomInvolution from a non-Object");
		}
		RandomInvolution{
			permutation: vec![],
		}
	}
}


///Use a permutation given via a file.
///See `RandomPermutation`.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct FileMap
{
	permutation: Vec<usize>,
}

//impl Quantifiable for FileMap
//{
//	fn total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//	fn print_memory_breakdown(&self)
//	{
//		unimplemented!();
//	}
//	fn forecast_total_memory(&self) -> usize
//	{
//		unimplemented!();
//	}
//}

impl Pattern for FileMap
{
	fn initialize(&mut self, _source_size:usize, _target_size:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
		//self.permutation=(0..size).collect();
		//rng.borrow_mut().shuffle(&mut self.permutation);
	}
	fn get_destination(&self, origin:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)->usize
	{
		self.permutation[origin]
	}
}

impl FileMap
{
	fn new(arg:PatternBuilderArgument) -> FileMap
	{
		let mut filename=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="FileMap"
			{
				panic!("A FileMap must be created from a `FileMap` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"filename" => match value
					{
						&ConfigurationValue::Literal(ref s) => filename=Some(s[1..s.len()-1].to_string()),
						_ => panic!("bad value for filename"),
					},
					//"pattern" => pattern=Some(new_pattern(value)),
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in FileMap",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a FileMap from a non-Object");
		}
		let filename=filename.expect("There were no filename");
		let file=File::open(&filename).expect("could not open pattern file.");
		let reader = BufReader::new(&file);
		let mut lines=reader.lines();
		let mut permutation=Vec::new();
		while let Some(rline)=lines.next()
		{
			let line=rline.expect("Some problem when reading the traffic pattern.");
			let mut words=line.split_whitespace();
			let origin=words.next().unwrap().parse::<usize>().unwrap();
			let destination=words.next().unwrap().parse::<usize>().unwrap();
			while permutation.len()<=origin || permutation.len()<=destination
			{
				permutation.push((-1isize) as usize);//which value use as filler?
			}
			permutation[origin]=destination;
		}
		FileMap{
			permutation,
		}
	}
}

///A pattern given by blocks. The elements are divided by blocks of size `block_size`. The `global_pattern` is used to describe the communication among different blocks and the `block_pattern` to describe the communication inside a block.
///Seen as a graph, this is the Kronecker product of the block graph with the global graph.
///Thus the origin a position `i` in the block `j` will select the destination at position `b(i)` in the block `g(j)`, where `b(i)` is the destination via the `block_pattern` and `g(j)` is the destination via the `global_pattern`.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct ProductPattern
{
	block_size: usize,
	block_pattern: Box<dyn Pattern>,
	global_pattern: Box<dyn Pattern>,
}

impl Pattern for ProductPattern
{
	fn initialize(&mut self, source_size:usize, target_size:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		if source_size!=target_size
		{
			unimplemented!("Different sizes are not yet implemented for ProductPattern");
		}
		self.block_pattern.initialize(self.block_size,self.block_size,topology,rng);
		let global_size=source_size/self.block_size;
		self.global_pattern.initialize(global_size,global_size,topology,rng);
	}
	fn get_destination(&self, origin:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)->usize
	{
		let local=origin % self.block_size;
		let global=origin / self.block_size;
		let local_dest=self.block_pattern.get_destination(local,topology,rng);
		let global_dest=self.global_pattern.get_destination(global,topology,rng);
		global_dest*self.block_size+local_dest
	}
}

impl ProductPattern
{
	fn new(arg:PatternBuilderArgument) -> ProductPattern
	{
		let mut block_size=None;
		let mut block_pattern=None;
		let mut global_pattern=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Product"
			{
				panic!("A ProductPattern must be created from a `Product` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"block_pattern" => block_pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					"global_pattern" => global_pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					"block_size" => match value
					{
						&ConfigurationValue::Number(f) => block_size=Some(f as usize),
						_ => panic!("bad value for block_size"),
					}
					//"load" => match value
					//{
					//	&ConfigurationValue::Number(f) => load=Some(f as f32),
					//	_ => panic!("bad value for load"),
					//}
					//"message_size" => (),
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in ProductPattern",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ProductPattern from a non-Object");
		}
		let block_size=block_size.expect("There were no block_size");
		let block_pattern=block_pattern.expect("There were no block_pattern");
		let global_pattern=global_pattern.expect("There were no global_pattern");
		ProductPattern{
			block_size,
			block_pattern,
			global_pattern,
		}
	}
}

///Divide the topology according to some given link classes, considering the graph components if the other links were removed.
///Then apply the `global_pattern` among the components and select randomly inside the destination comonent.
///Note that this uses the topology and will cause problems if used as a sub-pattern.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct ComponentsPattern
{
	component_classes: Vec<usize>,
	//block_pattern: Box<dyn Pattern>,//we would need patterns between places of different extent.
	global_pattern: Box<dyn Pattern>,
	components: Vec<Vec<usize>>,
}

impl Pattern for ComponentsPattern
{
	fn initialize(&mut self, _source_size:usize, _target_size:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		let mut allowed_components=vec![];
		for link_class in self.component_classes.iter()
		{
			if *link_class>=allowed_components.len()
			{
				allowed_components.resize(*link_class+1,false);
			}
			allowed_components[*link_class]=true;
		}
		self.components=topology.components(&allowed_components);
		//for (i,component) in self.components.iter().enumerate()
		//{
		//	println!("component {}: {:?}",i,component);
		//}
		self.global_pattern.initialize(self.components.len(),self.components.len(),topology,rng);
	}
	fn get_destination(&self, origin:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)->usize
	{
		//let local=origin % self.block_size;
		//let global=origin / self.block_size;
		//let n=topology.num_routers();
		let router_origin=match topology.server_neighbour(origin).0
		{
			Location::RouterPort{
				router_index,
				router_port: _,
			} => router_index,
			_ => panic!("what origin?"),
		};
		let mut global=self.components.len();
		for (g,component) in self.components.iter().enumerate()
		{
			if component.contains(&router_origin)
			{
				global=g;
				break;
			}
		}
		if global==self.components.len()
		{
			panic!("Could not found component of {}",router_origin);
		}
		let global_dest=self.global_pattern.get_destination(global,topology,rng);
		//let local_dest=self.block_pattern.get_destination(local,topology,rng);
		let r_local=rng.borrow_mut().gen_range(0,self.components[global_dest].len());
		let dest=self.components[global_dest][r_local];
		let radix=topology.ports(dest);
		let mut candidate_stack=Vec::with_capacity(radix);
		for port in 0..radix
		{
			match topology.neighbour(dest,port).0
			{
				Location::ServerPort(destination) => candidate_stack.push(destination),
				_ => (),
			}
		}
		let rserver=rng.borrow_mut().gen_range(0,candidate_stack.len());
		candidate_stack[rserver]
	}
}

impl ComponentsPattern
{
	fn new(arg:PatternBuilderArgument) -> ComponentsPattern
	{
		let mut component_classes=None;
		//let mut block_pattern=None;
		let mut global_pattern=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Components"
			{
				panic!("A ComponentsPattern must be created from a `Components` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"block_pattern" => block_pattern=Some(new_pattern(value,plugs)),
					"global_pattern" => global_pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					"component_classes" => match value
					{
						&ConfigurationValue::Array(ref a) => component_classes=Some(a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in component_classes"),
						}).collect()),
						_ => panic!("bad value for component_classes"),
					}
					//"load" => match value
					//{
					//	&ConfigurationValue::Number(f) => load=Some(f as f32),
					//	_ => panic!("bad value for load"),
					//}
					//"message_size" => (),
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in ComponentsPattern",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a ComponentsPattern from a non-Object");
		}
		let component_classes=component_classes.expect("There were no component_classes");
		//let block_pattern=block_pattern.expect("There were no block_pattern");
		let global_pattern=global_pattern.expect("There were no global_pattern");
		ComponentsPattern{
			component_classes,
			//block_pattern,
			global_pattern,
			components:vec![],//filled at initialize
		}
	}
}



/// Interpretate the origin as with cartesian coordinates and apply transformations.
/// May permute the dimensions if they have same side.
/// May complement the dimensions.
/// Order of composition is: first permute -> then complement.
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct CartesianTransform
{
	///The Cartesian interpretation.
	cartesian_data: CartesianData,
	///Optionally how dimensions are permuted.
	///permute=[0,2,1] means to permute dimensions 1 and 2, keeping dimension 0 as is.
	permute: Option<Vec<usize>>,
	///Optionally, which dimensions must be complemented.
	///complement=[true,false,false] means target_coordinates[0]=side-1-coordinates[0].
	complement: Option<Vec<bool>>,
}

impl Pattern for CartesianTransform
{
	fn initialize(&mut self, source_size:usize, target_size:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)
	{
		if source_size!=target_size
		{
			panic!("In a Cartesiantransform source_size({}) must be equal to target_size({}).",source_size,target_size);
		}
		if source_size!=self.cartesian_data.size
		{
			panic!("Sizes do not agree on CartesianTransform.");
		}
	}
	fn get_destination(&self, origin:usize, _topology:&Box<dyn Topology>, _rng: &RefCell<StdRng>)->usize
	{
		let up_origin=self.cartesian_data.unpack(origin);
		let up_permuted=match self.permute
		{
			//XXX Should we panic on side mismatch?
			Some(ref v) => v.iter().map(|&index|up_origin[index]).collect(),
			None => up_origin,
		};
		let up_complemented=match self.complement
		{
			Some(ref v) => up_permuted.iter().enumerate().map(|(index,&value)|if v[index]{self.cartesian_data.sides[index]-1-value}else {value}).collect(),
			None => up_permuted,
		};
		self.cartesian_data.pack(&up_complemented)
	}
}

impl CartesianTransform
{
	fn new(arg:PatternBuilderArgument) -> CartesianTransform
	{
		let mut sides=None;
		let mut permute=None;
		let mut complement=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="CartesianTransform"
			{
				panic!("A CartesianTransform must be created from a `CartesianTransform` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"sides" => match value
					{
						&ConfigurationValue::Array(ref a) => sides=Some(a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in sides"),
						}).collect()),
						_ => panic!("bad value for sides"),
					}
					"permute" => match value
					{
						&ConfigurationValue::Array(ref a) => permute=Some(a.iter().map(|v|match v{
							&ConfigurationValue::Number(f) => f as usize,
							_ => panic!("bad value in permute"),
						}).collect()),
						_ => panic!("bad value for permute"),
					}
					"complement" => match value
					{
						&ConfigurationValue::Array(ref a) => complement=Some(a.iter().map(|v|match v{
							&ConfigurationValue::True => true,
							&ConfigurationValue::False => false,
							_ => panic!("bad value in complement"),
						}).collect()),
						_ => panic!("bad value for complement"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in CartesianTransform",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a CartesianTransform from a non-Object");
		}
		let sides=sides.expect("There were no sides");
		//let permute=permute.expect("There were no permute");
		//let complement=complement.expect("There were no complement");
		CartesianTransform{
			cartesian_data: CartesianData::new(&sides),
			permute,
			complement,
		}
	}
}

///The pattern resulting of composing a list of patterns.
///destination=patterns[len-1]( patterns[len-2] ( ... (patterns[1] ( patterns[0]( origin ) )) ) ).
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct Composition
{
	patterns: Vec<Box<dyn Pattern>>,
}

impl Pattern for Composition
{
	fn initialize(&mut self, source_size:usize, target_size:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		for pattern in self.patterns.iter_mut()
		{
			pattern.initialize(source_size,target_size,topology,rng);
		}
	}
	fn get_destination(&self, origin:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)->usize
	{
		let mut destination=origin;
		for pattern in self.patterns.iter()
		{
			destination=pattern.get_destination(destination,topology,rng);
		}
		destination
	}
}

impl Composition
{
	fn new(arg:PatternBuilderArgument) -> Composition
	{
		let mut patterns=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Composition"
			{
				panic!("A Composition must be created from a `Composition` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"patterns" => match value
					{
						&ConfigurationValue::Array(ref l) => patterns=Some(l.iter().map(|pcv|new_pattern(PatternBuilderArgument{cv:pcv,..arg})).collect()),
						_ => panic!("bad value for patterns"),
					}
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Composition",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a Composition from a non-Object");
		}
		let patterns=patterns.expect("There were no patterns");
		Composition{
			patterns,
		}
	}
}



///The pattern resulting of composing a pattern with itself a number of times..
#[derive(Quantifiable)]
#[derive(Debug)]
pub struct Pow
{
	pattern: Box<dyn Pattern>,
	exponent: usize,
}

impl Pattern for Pow
{
	fn initialize(&mut self, source_size:usize, target_size:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)
	{
		self.pattern.initialize(source_size,target_size,topology,rng);
	}
	fn get_destination(&self, origin:usize, topology:&Box<dyn Topology>, rng: &RefCell<StdRng>)->usize
	{
		let mut destination=origin;
		for _ in 0..self.exponent
		{
			destination=self.pattern.get_destination(destination,topology,rng);
		}
		destination
	}
}

impl Pow
{
	fn new(arg:PatternBuilderArgument) -> Pow
	{
		let mut pattern=None;
		let mut exponent=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=arg.cv
		{
			if cv_name!="Pow"
			{
				panic!("A Pow must be created from a `Pow` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					"pattern" => pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					"exponent" => match value
					{
						&ConfigurationValue::Number(x) => exponent=Some(x as usize),
						_ => panic!("bad value for exponent"),
					},
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in Pow",name),
				}
			}
		}
		else
		{
			panic!("Trying to create a Pow from a non-Object");
		}
		let pattern=pattern.expect("There were no pattern");
		let exponent=exponent.expect("There were no exponent");
		Pow{
			pattern,
			exponent,
		}
	}
}
