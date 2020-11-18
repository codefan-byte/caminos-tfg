
pub mod cartesian;
pub mod neighbourslists;
pub mod dragonfly;
pub mod projective;
pub mod slimfly;

use std::cell::{RefCell};
use ::rand::{StdRng};
use quantifiable_derive::Quantifiable;//the derive macro
use self::cartesian::{Mesh,Torus,CartesianData,Hamming};
use self::neighbourslists::NeighboursLists;
use self::dragonfly::CanonicDragonfly;
use self::projective::{Projective,LeviProjective};
use self::slimfly::SlimFly;
use crate::config_parser::ConfigurationValue;
use crate::matrix::Matrix;
use crate::quantify::Quantifiable;
use crate::Plugs;

///A location where a phit can be inserted.
///None is used for disconnected ports, for example in the `Mesh` topology.
#[derive(Clone,Debug,Quantifiable)]
pub enum Location
{
	RouterPort{
		router_index: usize,
		router_port: usize,
	},
	ServerPort(usize),
	None,
}

///A topology describes how routers and servers are connected.
///The router `index` has `ports(index)` neighbours. The first `degree(index)` must be other routers.
pub trait Topology : Quantifiable + std::fmt::Debug
{
	fn num_routers(&self) -> usize;
	fn num_servers(&self) -> usize;
	fn num_arcs(&self) -> usize;
	///Neighbours of a router: Location+link class index
	///Routers should be before servers
	fn neighbour(&self, router_index:usize, port:usize) -> (Location,usize);
	///The neighbour of a server: Location+link class index
	//FIXME: What to do with BCube and similar?
	fn server_neighbour(&self, server_index:usize) -> (Location,usize);
	///the greatest distance from server to server
	fn diameter(&self) -> usize;
	///from servers to different servers
	fn average_distance(&self) -> f32;
	///Distance from a router to another.
	fn distance(&self,origin:usize,destination:usize) -> usize;
	///Number of shortest paths from a router to another.
	fn amount_shortest_paths(&self,origin:usize,destination:usize) -> usize;
	///Average number of shortest paths from a router to another.
	fn average_amount_shortest_paths(&self) -> f32;
	//fn arc_uniformity(&self) -> f32;
	//fn throughput(&self) -> f32;
	//fn get_arc_betweenness_matrix(&self) -> ??
	fn distance_distribution(&self,origin:usize) -> Vec<usize>;
	//fn eigenvalue_powerdouble(&self) -> f32
	fn maximum_degree(&self) -> usize;
	fn minimum_degree(&self) -> usize;
	/// Number of ports used to other routers. FIXME: Should iclude NULL ports?
	fn degree(&self, router_index: usize) -> usize;
	fn ports(&self, router_index: usize) -> usize;
	//std::vector<std::vector<length> >* nonEdgeDistances()const;
	//length girth()const;
	
	///Specific for some toologies, but must be checkable for anyone
	fn cartesian_data(&self) -> Option<&CartesianData>;
	///Specific for some toologies, but must be checkable for anyone
	fn coordinated_routing_record(&self, coordinates_a:&Vec<usize>, coordinates_b:&Vec<usize>)->Vec<i32>;
	///Specific for some toologies, but must be checkable for anyone
	/// Indicates if going from input_port to output_port implies a direction change. Used for the bubble routing.
	fn is_direction_change(&self, router_index:usize, input_port: usize, output_port: usize) -> bool;

	///Breadth First Search to compute distances from a router to all others.
	///It may use weights, but it there are multiple paths with different distances it may give a non-minimal distance, since it is not Dijkstra.
	fn bfs(&self, origin:usize, class_weight:Option<&[usize]>) -> Vec<usize>
	{
		//Adapted from my code for other software.
		let n=self.num_routers();
		#[allow(non_snake_case)]
		let mut R=vec![<usize>::max_value();n];
		R[origin]=0;
		//let mut queue=vec![0;n];
		let queue_len=match class_weight
		{
			Some(ref v)=> n*v.len(),
			None => n,
		};
		let mut queue=vec![0;queue_len];
		let mut queue_read_index=0;//Next to read
		let mut queue_write_index=1;//Next to write
		queue[0]=origin;
		//while queue_read_index<n
		while queue_read_index<queue_write_index
		{
			let best=queue[queue_read_index];
			queue_read_index+=1;
			//let alt=R[best]+1;
			//let alt=R[best].saturating_add(1);
			//std::vector<vertex_index> nbor=neighbours(best);
			let degree=self.degree(best);
			for i in 0..degree
			{
				match self.neighbour(best,i)
				{
					(Location::RouterPort{
						router_index,
						router_port: _,
					},link_class) =>
					{
						let weight= if let Some(ref v)=class_weight
						{
							if link_class>=v.len()
							{
								continue//next neighbour
							}
							let x=v[link_class];
							if x==<usize>::max_value()
							{
								continue//next neighbour
							}
							x
						}
						else
						{
							1
						};
						let alt=R[best].saturating_add(weight);
						if alt<R[router_index]
						{
							//println!("router_index={} n={} queue_write_index={} queue_read_index={}",router_index,n,queue_write_index,queue_read_index);
							R[router_index]=alt;
							queue[queue_write_index]=router_index;
							queue_write_index+=1;
						}
					}
					_ => panic!("what?"),
				}
			}
		}
		return R;
	}
	
	//Matrix<length>* Graph::computeDistanceMatrix()
	fn compute_distance_matrix(&self, class_weight:Option<&[usize]>) -> Matrix<usize>
	{
		//return floyd();
		let n=self.num_routers();
		let mut matrix=Matrix::constant(0,n,n);
		for i in 0..n
		{
			let d=self.bfs(i,class_weight);
			for j in 0..n
			{
				*matrix.get_mut(i,j)=d[j];
			}
		}
		return matrix;
	}

	fn floyd(&self) -> Matrix<usize>
	{
		// Implements Floyd–Warshall algorithm. This was adapted from a previous code for another software.
		//printf(">>Graph::computeDistanceMatrix\n");
		let n=self.num_routers();
		//Matrix<length>* matrix=new Matrix<length>(n,n);
		let mut matrix=Matrix::constant(<usize>::max_value()/3,n,n);
		//vertex_index i,j,k;
		//length x;
		//for(i=0;i<n;i++)matrix->get(i,i)=0;
		for i in 0..n
		{
			*matrix.get_mut(i,i)=0;
		}
		//for(i=0;i<n;i++)
		for i in 0..n
		{
			//std::vector<vertex_index> nbor=neighbours(i);
			let degree=self.degree(i);
			//for(j=0;j<nbor.size();j++)
			for j in 0..degree
			{
				//matrix->get(i,nbor[j])=1;
				match self.neighbour(i,j).0
				{
					Location::RouterPort{
						router_index,
						router_port: _,
					} => *matrix.get_mut(i,router_index)=1,
					_ => panic!("what?"),
				}
			}
		}
		//for(k=0;k<n;k++)
		for k in 0..n
		{
			//for(i=0;i<n;i++)
			for i in 0..n
			{
				//for(j=0;j<n;j++)
				for j in 0..n
				{
					//x=matrix->get(i,k)+matrix->get(k,j);
					let x=*matrix.get(i,k)+*matrix.get(k,j);
					//if(matrix->get(i,j)>x)matrix->get(i,j)=x;
					if *matrix.get(i,j)>x
					{
						*matrix.get_mut(i,j)=x;
					}
				}
			}
		}
		//printf("<<Graph::computeDistanceMatrix\n");
		return matrix;
	}
	
	///Return a pair of matrices `(D,A)` with `D[i,j]` being the distance from `i` to `j`
	///and `A[i,j]` being the number of paths of length `D[i,j]` from `i` to `j`.
	fn compute_amount_shortest_paths(&self) -> (Matrix<usize>,Matrix<usize>)
	{
		//Copied from discrete topologies
		//if(amountMinimumPathsMatrix)return;
		//vertex_index n=size();
		let n=self.num_routers();
		//if(distanceMatrix==NULL)
		//{
		//	distanceMatrix=new Matrix<length>(n,n);
		//}
		let maximum_length=<usize>::max_value()/3;
		let mut distance_matrix=Matrix::constant(maximum_length,n,n);
		let mut amount_matrix=Matrix::constant(1,n,n);
		//amountMinimumPathsMatrix=new Matrix<long>(n,n);
		//for(long i=0;i<n;i++)
		//for(long j=0;j<n;j++)
		//{
		//	distanceMatrix->get(i,j)=LENGTH_MAX;
		//	amountMinimumPathsMatrix->get(i,j)=1;
		//}
		//for(vertex_index origin=0;origin<n;origin++)
		for origin in 0..n
		{
			//distanceMatrix->get(origin,origin)=0;
			*distance_matrix.get_mut(origin,origin)=0;
			//std::vector<vertex_index> queue(n);
			let mut queue=vec![0;n];
			//long queue_read_index=0, queue_write_index=1;
			let mut queue_read_index=0;
			let mut queue_write_index=1;
			queue[0]=origin;
			while queue_read_index<n
			{
				//vertex_index best=queue[queue_read_index++];
				let best=queue[queue_read_index];
				queue_read_index+=1;
				//std::vector<vertex_index> nbor=neighbours(best);
				let degree=self.degree(best);
				//length bd=distanceMatrix->get(origin,best);
				let bd=*distance_matrix.get(origin,best);
				//length alt=bd+1;
				let alt=bd+1;
				//long ba=amountMinimumPathsMatrix->get(origin,best);
				let ba=*amount_matrix.get(origin,best);
				//for(std::vector<vertex_index>::iterator it=nbor.begin();it!=nbor.end();++it)
				for i in 0..degree
				{
					match self.neighbour(best,i).0
					{
						Location::RouterPort{
							router_index,
							router_port: _,
						} =>
						{
							//length old=distanceMatrix->get(origin,*it);
							let old=*distance_matrix.get(origin,router_index);
							if alt<old
							{
								*distance_matrix.get_mut(origin,router_index)=alt;
								*amount_matrix.get_mut(origin,router_index)=ba;
								queue[queue_write_index]=router_index;
								queue_write_index+=1;
							}
							else if alt==old
							{
								//amountMinimumPathsMatrix->get(origin,*it)+=ba;
								*amount_matrix.get_mut(origin,router_index)+=ba;
							}
						}
						_ => panic!("what?"),
					}
				}
			}
		}
		(distance_matrix,amount_matrix)
	}

	/// Find the coponents of the subtopology induced via the allowed links.
	/// Returns vector ret with ret[k] containing the vertices in the k-th component.
	fn components(&self,allowed_classes:&[bool]) -> Vec<Vec<usize>>
	{
		let mut r=vec![];
		let n=self.num_routers();
		let mut found=vec![false;n];
		let weights:Vec<usize>=allowed_classes.iter().map(|a|if *a{1}else {<usize>::max_value()}).collect();
		for i in 0..n
		{
			if ! found[i]
			{
				let rindex=r.len();
				r.push(vec![i]);
				let d=self.bfs(i,Some(&weights));
				for j in 0..n
				{
					if i!=j && d[j]!=<usize>::max_value()
					{
						r[rindex].push(j);
						found[j]=true;
					}
				}
				//println!("Computed component[{}]={:?}",rindex,r[rindex]);
				//println!("Distances({})={:?}",i,d.iter().map(|v|if *v>100{100}else {*v}).collect::<Vec<usize>>());
			}
		}
		return r;
	}
	
	/// returns a cople matrices (N,F) with
	///	N[u,v] = number of neighbours w of v with D(u,v)>D(u,w).
	///	F[u,v] = number of neighbours w of v with D(u,v)<D(u,w).
	/// A router v with F[u,v]=0 is called a boundary vertex of u.
	fn compute_near_far_matrices(&self) -> (Matrix<usize>,Matrix<usize>)
	{
		let n=self.num_routers();
		let mut near_matrix=Matrix::constant(0,n,n);
		let mut far_matrix=Matrix::constant(0,n,n);
		for origin in 0..n
		{
			//  It may be faster with a tuned BFS.
			//let d=self.bfs(i,class_weight);
			//for j in 0..n
			//{
			//	*matrix.get_mut(i,j)=d[j];
			//}
			// But we just check the distance function.
			for target in 0..n
			{
				let degree=self.degree(target);
				for index in 0..degree
				{
					let dist = self.distance(origin,target);
					match self.neighbour(target,index)
					{
						(Location::RouterPort{
							router_index: w,
							router_port: _,
						},_link_class) =>
						{
							let alt = self.distance(origin,w);
							if alt>dist
							{
								*far_matrix.get_mut(origin,target) += 1;
							}
							else if alt<dist
							{
								*near_matrix.get_mut(origin,target) += 1;
							}
						},
						(Location::None,_link_class) => continue,//ignore disconnected ports
						_ => panic!("what?"),
					}
				}
			}
		}
		return (near_matrix,far_matrix);
	}
	
	///Computes the eccentricy of a router. That is, the greatest possible length of a shortest path from that router to any other.
	fn eccentricity(&self, router_index:usize) -> usize
	{
		let n=self.num_routers();
		(0..n).map(|other|self.distance(router_index,other)).max().expect("should have a maximum.")
	}

	///Check pairs (port,vc) with
	/// * non-matching endpoint (this is, going backwards a wire you should return to the same router/server)
	/// * breaking the servers-last rule
	/// * optionally check that the link class is within bounds.
	fn check_adjacency_consistency(&self,amount_link_classes: Option<usize>)
	{
		let n=self.num_routers();
		let mut max_link_class=0;
		for router_index in 0..n
		{
			let deg = self.degree(router_index);
			for port_index in 0..self.ports(router_index)
			{
				let (neighbour_location, link_class) = self.neighbour(router_index, port_index);
				if let Some(bound) = amount_link_classes
				{
					assert!(link_class<bound,"link class {} out of bound {} for port {} of router {}",link_class,bound,port_index,router_index);
				}
				if link_class>max_link_class
				{
					max_link_class=link_class;
				}
				match neighbour_location
				{
					Location::RouterPort{
						router_index: neighbour_router,
						router_port: neighbour_port,
					} =>
					{
						if let Some(bound) = amount_link_classes
						{
							if link_class+1==bound
							{
								println!("WARNING: using last link class ({}) for a router to router link.",link_class);
							}
						}
						let (rev_location, rev_link_class) = self.neighbour(neighbour_router, neighbour_port);
						match rev_location
						{
							Location::RouterPort{
								router_index: rev_router,
								router_port: rev_port,
							} =>
							{
								if router_index!=rev_router || port_index!=rev_port
								{
									panic!("Non-matching port ({},{}) to ({},{}) non-returns to ({},{}).",router_index,port_index,neighbour_router,neighbour_port,rev_router,rev_port);
								}
							},
							_ => panic!("It does not even return to a router"),
						};
						if link_class!=rev_link_class
						{
							panic!("port {} at router {} has non-matching link class {} vs {}",port_index,router_index,link_class,rev_link_class);
						}
						if port_index>=deg
						{
							panic!("port {} at router {} connects to another router and it is >=degree={}",port_index,router_index,deg);
						}
					},
					Location::ServerPort(server_index) =>
					{
						let (rev_location, rev_link_class) = self.server_neighbour(server_index);
						match rev_location
						{
							Location::RouterPort{
								router_index: rev_router,
								router_port: rev_port,
							} =>
							{
								if router_index!=rev_router || port_index!=rev_port
								{
									panic!("Non-matching port ({},{}) to server {} non-returns to ({},{}).",router_index,port_index,server_index,rev_router,rev_port);
								}
							},
							_ => panic!("It does not even return to a router"),
						};
						if link_class!=rev_link_class
						{
							panic!("port {} at router {} has non-matching link class {} vs {}",port_index,router_index,link_class,rev_link_class);
						}
						if port_index<deg
						{
							panic!("port {} at router {} connects to a server and it is <degree={}",port_index,router_index,deg);
						}
					},
					Location::None => println!("WARNING: disconnected port {} at router {}",port_index,router_index),
				}
			}
		}
		if let Some(bound)=amount_link_classes
		{
			if bound!=max_link_class+1
			{
				println!("WARNING: quering {} link classes when the topology has {}",bound,max_link_class+1);
			}
		}
	}
}

#[non_exhaustive]
pub struct TopologyBuilderArgument<'a>
{
	///A ConfigurationValue::Object defining the topology.
	pub cv: &'a ConfigurationValue,
	///The user defined plugs. In case the topology needs to create elements.
	pub plugs: &'a Plugs,
	///The random number generator to use.
	pub rng: &'a RefCell<StdRng>,
}

///Build a topology.
pub fn new_topology(arg:TopologyBuilderArgument) -> Box<dyn Topology>
{
	if let &ConfigurationValue::Object(ref cv_name, ref _cv_pairs)=arg.cv
	{
		match arg.plugs.topologies.get(cv_name)
		{
			Some(builder) => return builder(arg),
			_ => (),
		};
		match cv_name.as_ref()
		{
			"Mesh" => Box::new(Mesh::new(arg.cv)),
			"Torus" => Box::new(Torus::new(arg.cv)),
			"RandomRegularGraph" | "File" => Box::new(NeighboursLists::new_cfg(arg.cv,arg.rng)),
			"Hamming" => Box::new(Hamming::new(arg.cv)),
			"CanonicDragonfly" => Box::new(CanonicDragonfly::new(arg.cv)),
			"Projective" => Box::new(Projective::new(arg)),
			"LeviProjective" => Box::new(LeviProjective::new(arg)),
			"SlimFly" => Box::new(SlimFly::new(arg)),
			_ => panic!("Unknown topology {}",cv_name),
		}
	}
	else
	{
		panic!("Trying to create a topology from a non-Object");
	}
}

