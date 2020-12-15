
use std::path::{Path,PathBuf};
use std::fs::{self,File};
use std::io::{self,Write,Read};
use std::cmp::Ordering;
use std::process::Command;
use std::collections::HashSet;
use std::rc::Rc;

use crate::config_parser::{ConfigurationValue,Expr};
use crate::config::{combine,evaluate,reevaluate,values_to_f32};
use crate::get_git_id;

#[derive(Debug)]
pub enum BackendError
{
	CouldNotGenerateFile{
		filepath: PathBuf,
		io_error: Option<io::Error>,
	}
}

///Creates some output using an output description object as guide.
pub fn create_output(description: &ConfigurationValue, results: &Vec<(ConfigurationValue,ConfigurationValue)>, total_experiments:usize, path:&Path)
	-> Result<(),BackendError>
{
	if let &ConfigurationValue::Object(ref name, ref _attributes) = description
	{
		match name.as_ref()
		{
			"CSV" =>
			{
				println!("Creating a CSV...");
				return create_csv(description,results,path);
			},
			"Plots" =>
			{
				println!("Creating a plot...");
				return create_plots(description,results,total_experiments,path);
			},
			_ => panic!("unrecognized output description object {}",name),
		};
	}
	else
	{
		panic!("Output description is not an object.");
	};
}

///Creates a csv file using filename and field given in `description`.
fn create_csv(description: &ConfigurationValue, results: &Vec<(ConfigurationValue,ConfigurationValue)>, path:&Path)
	-> Result<(),BackendError>
{
	let mut fields=None;
	let mut filename=None;
	if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=description
	{
		if cv_name!="CSV"
		{
			panic!("A CSV must be created from a `CSV` object not `{}`",cv_name);
		}
		for &(ref name,ref value) in cv_pairs
		{
			match name.as_ref()
			{
				"fields" => match value
				{
					&ConfigurationValue::Array(ref a) => fields=Some(a.iter().map(|v|match v{
						&ConfigurationValue::Expression(ref expr) => expr.clone(),
						_ => panic!("bad value for fields"),
					}).collect::<Vec<Expr>>()),
					_ => panic!("bad value for fields"),
				}
				"filename" => match value
				{
					&ConfigurationValue::Literal(ref s) => filename=Some(s.to_string()),
					_ => panic!("bad value for filename ({:?})",value),
				}
				_ => panic!("Nothing to do with field {} in CSV",name),
			}
		}
	}
	else
	{
		panic!("Trying to create a CSV from a non-Object");
	}
	let fields=fields.expect("There were no fields");
	let filename=filename.expect("There were no filename");
	println!("Creating CSV with name \"{}\"",filename);
	let output_path=path.join(filename);
	let mut output_file=File::create(&output_path).expect("Could not create output file.");
	let header=fields.iter().map(|e|format!("{}",e)).collect::<Vec<String>>().join(", ");
	writeln!(output_file,"{}",header).unwrap();
	for &(ref configuration,ref result) in results.iter()
	{
		let context=combine(configuration,result);
		let row=fields.iter().map(|e| format!("{}",evaluate(e,&context)) ).collect::<Vec<String>>().join(", ");
		writeln!(output_file,"{}",row).unwrap();
	}
	Ok(())
}

///The raw `ConfigurationValue`s to be used in a plot. Before being averaged.
#[derive(PartialEq,PartialOrd,Debug)]
struct RawRecord
{
	///The selector refers to some Figure
	selector: ConfigurationValue,
	///The legend refers to the line inside a Figure
	legend: ConfigurationValue,
	///The parameter refers to some point in a line, for which the average is being made.
	parameter: ConfigurationValue,
	///The value in the abscissas (a.k.a., x-axis).
	abscissa: ConfigurationValue,
	///The value in the ordinates (a.k.a., x-axis).
	ordinate: ConfigurationValue,
	///The git_id of the binary that produced the simulation.
	git_id: ConfigurationValue,
}

///The `f32`-averaged values to be used in a plot.
#[derive(Debug)]
struct AveragedRecord
{
	///The selector refers to some Figure
	selector: ConfigurationValue,
	///The legend refers to the line inside a Figure
	legend: ConfigurationValue,
	//The parameter refers to some point in a line, for which the average is being made.
	//It does not seem to be needed at the present moment.
	//parameter: ConfigurationValue,
	///The average value and standard deviation in the abscissas (a.k.a., x-axis).
	abscissa: (Option<f32>,Option<f32>),
	///The average value and standard deviation in the ordinates (a.k.a., x-axis).
	ordinate: (Option<f32>,Option<f32>),
	///Set of involved `git_id`s.
	git_set: HashSet<String>,
}

///A description of how to build a plot.
#[derive(Debug)]
struct Plotkind<'a>
{
	parameter: Option<&'a ConfigurationValue>,
	abscissas: Option<&'a ConfigurationValue>,
	ordinates: Option<&'a ConfigurationValue>,
	histogram: Option<&'a ConfigurationValue>,
	array: Option<&'a ConfigurationValue>,
	label_abscissas: String,
	label_ordinates: String,
	min_ordinate: Option<f32>,
	max_ordinate: Option<f32>,
}

impl<'a> Plotkind<'a>
{
	fn new(description: &'a ConfigurationValue)->Plotkind<'a>
	{
		let mut parameter=None;
		let mut abscissas=None;
		let mut ordinates=None;
		let mut histogram=None;
		let mut array=None;
		let mut label_abscissas=None;
		let mut label_ordinates=None;
		let mut min_ordinate=None;
		let mut max_ordinate=None;
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=description
		{
			if cv_name!="Plotkind"
			{
				panic!("A Plotkind must be created from a `Plotkind` object not `{}`",cv_name);
			}
			for &(ref name,ref value) in cv_pairs
			{
				match name.as_ref()
				{
					"parameter" => parameter=Some(value),
					"abscissas" => abscissas=Some(value),
					"ordinates" => ordinates=Some(value),
					"histogram" => histogram=Some(value),
					"array" => array=Some(value),
					"label_abscissas" => match value
					{
						&ConfigurationValue::Literal(ref s) => label_abscissas=Some(s.to_string()),
						_ => panic!("bad value for label_abscissas ({:?})",value),
					},
					"label_ordinates" => match value
					{
						&ConfigurationValue::Literal(ref s) => label_ordinates=Some(s.to_string()),
						_ => panic!("bad value for label_ordinates ({:?})",value),
					},
					"min_ordinate" => match value
					{
						&ConfigurationValue::Number(f) => min_ordinate=Some(f as f32),
						_ => panic!("bad value for min_ordinate"),
					}
					"max_ordinate" => match value
					{
						&ConfigurationValue::Number(f) => max_ordinate=Some(f as f32),
						_ => panic!("bad value for max_ordinate"),
					}
					_ => panic!("Nothing to do with field {} in Plotkind",name),
				}
			}
		}
		//let parameter=parameter.expect("There were no parameter");
		//let abscissas=abscissas.expect("There were no abscissas");
		//let ordinates=ordinates.expect("There were no ordinates");
		let label_abscissas=label_abscissas.expect("There were no label_abscissas");
		let label_ordinates=label_ordinates.expect("There were no label_ordinates");
		Plotkind{
			parameter,
			abscissas,
			ordinates,
			histogram,
			array,
			label_abscissas,
			label_ordinates,
			min_ordinate,
			max_ordinate,
		}
	}
}

///Create plots according to a `Plots` object.
fn create_plots(description: &ConfigurationValue, results: &Vec<(ConfigurationValue,ConfigurationValue)>, total_experiments:usize, path:&Path)
	-> Result<(),BackendError>
{
	let mut selector=None;
	let mut legend=None;
	let mut backend=None;
	let mut prefix=None;
	let mut kind:Option<Vec<Plotkind>>=None;
	if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=description
	{
		if cv_name!="Plots"
		{
			panic!("A series of Plots must be created from a `Plots` object not `{}`",cv_name);
		}
		for &(ref name,ref value) in cv_pairs
		{
			match name.as_ref()
			{
				"selector" => selector=Some(value),
				"legend" => legend=Some(value),
				"backend" => backend=Some(value),
				"kind" => match value
				{
					&ConfigurationValue::Array(ref pks) => kind=Some(pks.iter().map(Plotkind::new).collect()),
					_ => panic!("bad value for kind"),
				},
				"prefix" => match value
				{
					&ConfigurationValue::Literal(ref s) => prefix=Some(s.to_string()),
					_ => panic!("bad value for prefix"),
				},
				_ => panic!("Nothing to do with field {} in Plots",name),
			}
		}
	}
	else
	{
		panic!("Trying to create a Plots from a non-Object");
	}
	let selector=selector.expect("There were no selector");
	let legend=legend.expect("There were no legend");
	let kind=kind.expect("There were no kind");
	let backend=backend.expect("There were no backend");
	let prefix=prefix.unwrap_or_else(||"noprefix".to_string());
	println!("Creating plots");
	let mut avgs:Vec<Vec<AveragedRecord>>=Vec::with_capacity(kind.len());
	//let git_id_expr = Expr::Ident("git_id".to_string());
	let git_id_expr = Expr::Member( Rc::new(Expr::Ident("result".to_string())) , "git_id".to_string() );
	for pk in kind.iter()
	{
		println!("averaging plot {:?}",pk);
		//Each plot should be a map? `plot` with plot[legend_value][abscissa_value]=(ordinate_value,abscissa_deviation,ordinate_deviation)
		//And there should be a plot for each value of selector. Should this be a list or another map?
		//But first we compute (selector,legend_value,abscissa_value,ordinate_value) for each result.
		let mut records=Vec::with_capacity(results.len());
		let array = if let Some(ref data) = pk.histogram { Some(data) }
			else if let Some(ref data) = pk.array { Some(data)} else {None};
		//if let Some(histogram)=pk.histogram
		if let Some(data)=array
		{
			for &(ref configuration,ref result) in results.iter()
			{
				let context=combine(configuration,result);
				let histogram_values=reevaluate(data,&context);
				let selector=reevaluate(&selector,&context);
				let legend=reevaluate(&legend,&context);
				let git_id = evaluate(&git_id_expr,&context);
				if let ConfigurationValue::Array(ref l)=histogram_values
				{
					//let total:f64 = l.iter().map(|cv|match cv{
					//	ConfigurationValue::Number(x) => x,
					//	_ => panic!("adding an array of non-numbers for a histogram"),
					//}).sum();
					let factor:Option<f64> = if let Some(_)=pk.histogram
					{
						let total:f64 = l.iter().map(|cv|match cv{
							ConfigurationValue::Number(x) => x,
							_ => panic!("adding an array of non-numbers for a histogram"),
						}).sum();
						Some(1f64 / total)
					} else { None };
					for (h_index,h_value) in l.iter().enumerate()
					{
						let ordinate=if let &ConfigurationValue::Number(amount)=h_value
						{
							if let Some(factor)=factor
							{
								ConfigurationValue::Number(amount * factor)
							}
							else
							{
								ConfigurationValue::Number(amount)
							}
						}
						else
						{
							panic!("A histogram count/array value should be a number");
						};
						let record=RawRecord{
							selector:selector.clone(),
							legend:legend.clone(),
							parameter: ConfigurationValue::Number(h_index as f64),
							abscissa: ConfigurationValue::Number(h_index as f64),
							//ordinate: h_value.clone(),
							ordinate,
							git_id: git_id.clone(),
						};
						records.push(record);
					}
				}
				else
				{
					panic!("histogram/array from non-Array");
				}
			}
		}
		else
		{
			for &(ref configuration,ref result) in results.iter()
			{
				let context=combine(configuration,result);
				let record=RawRecord{
					selector:reevaluate(&selector,&context),
					legend:reevaluate(&legend,&context),
					parameter:reevaluate(&pk.parameter.unwrap(),&context),
					abscissa:reevaluate(&pk.abscissas.unwrap(),&context),
					ordinate:reevaluate(&pk.ordinates.unwrap(),&context),
					git_id: evaluate(&git_id_expr,&context),
				};
				//println!("{:?}",record);
				records.push(record);
			}
		}
		records.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));
		//println!("ordered as");
		//for record in records.iter()
		//{
		//	println!("{:?}",record);
		//}
		let mut averaged=Vec::with_capacity(records.len());
		let mut index=0;
		while index<records.len()
		{
			//let &(ref selector_value,ref legend_value,ref parameter_value,_,_)=&records[index];
			let &RawRecord{selector:ref selector_value,legend:ref legend_value,parameter:ref parameter_value,..}=&records[index];
			let mut current_abscissas:Vec<ConfigurationValue>=Vec::with_capacity(records.len());
			let mut current_ordinates:Vec<ConfigurationValue>=Vec::with_capacity(records.len());
			let mut git_set : HashSet<String> = HashSet::new();
			while index<records.len() && records[index].selector==*selector_value && records[index].legend==*legend_value && records[index].parameter==*parameter_value
			{
				//let &(_,_,_,ref abscissa_value,ref ordinate_value) = &records[index];
				let &RawRecord{abscissa:ref abscissa_value,ordinate:ref ordinate_value, git_id: ref git_value,..} = &records[index];
				current_abscissas.push(abscissa_value.clone());
				current_ordinates.push(ordinate_value.clone());
				index+=1;
				if let ConfigurationValue::Literal(ref git_str)=git_value
				{
					git_set.insert(git_str.to_string());
				}
			}
			//averaged.push( (selector_value.clone(),legend_value.clone(),standard_deviation(&current_abscissas),standard_deviation(&current_ordinates)) );
			averaged.push( AveragedRecord{selector:selector_value.clone(),legend:legend_value.clone(),abscissa:standard_deviation(&current_abscissas),ordinate:standard_deviation(&current_ordinates),git_set} );
		}
		//println!("averaged as");
		//for average in averaged.iter()
		//{
		//	println!("{:?}",average);
		//}
		avgs.push(averaged);
	}
	if let &ConfigurationValue::Object(ref name, ref _attributes) = backend
	{
		match name.as_ref()
		{
			//"Tikz" => tikz_backend(backend,averaged,&label_abscissas,&label_ordinates,min_ordinate,max_ordinate,path),
			"Tikz" => return tikz_backend(backend,avgs,kind,(results.len(),total_experiments),prefix,path),
			_ => panic!("unrecognized backend object {}",name),
		};
	}
	else
	{
		panic!("backend is not an object.");
	};
}

///Rewrites text into Latex code that output that text.
fn latex_protect_text(text:&str) -> String
{
	text.chars().map(|c|match c{
		'_' => "\\_".to_string(),
		'%' => "\\%".to_string(),
		x => format!("{}",x),
	}).collect::<String>()
}

///Make a command name trying to include all of `text`, replacing all non-alphabetic characters.
fn latex_make_command_name(text:&str) -> String
{
	text.chars().map(|c|match c{
		'a'..='z' | 'A'..='Z' => format!("{}",c),
		'0' => "R".to_string(),
		'1' => "RI".to_string(),
		'2' => "RII".to_string(),
		'3' => "RIII".to_string(),
		'4' => "RIV".to_string(),
		'5' => "RV".to_string(),
		'6' => "RVI".to_string(),
		'7' => "RVII".to_string(),
		'8' => "RVIII".to_string(),
		'9' => "RIX".to_string(),
		'_' => "u".to_string(),
		':' => "c".to_string(),
		_ => "x".to_string(),
	}).collect::<String>()
}

///Draw a plot using the tikz backend.
///`backend`: contains options to the backend
///`averages[kind_index][point_index]`: contains the data to be plotted. The data is ordered by selector, which is not an index.
///`kind`: the congiguration of the plots
///`amount_experiments`: (experiments_with_results, total) of the experiments
///`path`: the path of the whole experiment
fn tikz_backend(backend: &ConfigurationValue, averages: Vec<Vec<AveragedRecord>>, kind:Vec<Plotkind>, amount_experiments:(usize,usize), prefix:String, path:&Path)
	-> Result<(),BackendError>
{
	let mut tex_filename=None;
	let mut pdf_filename=None;
	if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs)=backend
	{
		if cv_name!="Tikz"
		{
			panic!("A Tikz must be created from a `Tikz` object not `{}`",cv_name);
		}
		for &(ref name,ref value) in cv_pairs
		{
			match name.as_ref()
			{
				"tex_filename" => match value
				{
					&ConfigurationValue::Literal(ref s) => tex_filename=Some(s.to_string()),
					_ => panic!("bad value for tex_filename ({:?})",value),
				},
				"pdf_filename" => match value
				{
					&ConfigurationValue::Literal(ref s) => pdf_filename=Some(s.to_string()),
					_ => panic!("bad value for pdf_filename ({:?})",value),
				},
				_ => panic!("Nothing to do with field {} in Tikz",name),
			}
		}
	}
	else
	{
		panic!("Trying to create a Tikz from a non-Object");
	}
	let tex_filename=tex_filename.expect("There were no tex_filename");
	let pdf_filename=pdf_filename.expect("There were no pdf_filename");
	let tex_path=path.join(tex_filename);
	println!("Creating {:?}",tex_path);
	let mut tex_file=File::create(&tex_path).expect("Could not create tex file.");
	let mut tikz=String::new();
	let ymin:Vec<String>=kind.iter().map(|kd| match kd.min_ordinate{
		None => String::new(),
		Some(x) => format!("ymin={},",x),
	}).collect();
	let ymax:Vec<String>=kind.iter().map(|kd| match kd.max_ordinate{
		None => String::new(),
		Some(x) => format!("ymax={},",x),
	}).collect();
	let mut all_legend_tex_id_vec:Vec<String> = Vec::new();
	let mut all_legend_tex_id_set:HashSet<String> = HashSet::new();
	let mut all_legend_tex_entry:HashSet<String> = HashSet::new();
	//while index<averaged.len()
	//let mut figure_index=0;
	let mut all_git_ids: HashSet<String> = HashSet::new();
	let mut offsets:Vec<usize>=(0..kind.len()).collect();//to keep track of the offset as progressing in selectors.
	//We try to make a figure for each selector. Then in each figure we make a tikzpicture for each PlotKind.
	'figures: loop
	{
		let mut wrote=0;//amount of plotkinds already written. We use this as end condition.
		let mut tracked_selector_value=None;
		let mut figure_tikz=String::new();
		for kind_index in 0..kind.len()
		{
			//println!("averages.len()={}",averages.len());
			//println!("averages[{}].len()={}",kind_index,averages[kind_index].len());
			if offsets[kind_index]>=averages[kind_index].len()
			{
				//There are not any points left.
				//continue;
				break 'figures;
			}
			let kaverages=&averages[kind_index];
			let koffset=&mut offsets[kind_index];
			let kd=&kind[kind_index];
			let selector_value=&kaverages[*koffset].selector;
			let selector_value_to_use = if let Some(value)=tracked_selector_value
			{
				if selector_value != value
				{
					println!("warning: missing data");
					//continue;
					break 'figures;
				}
				value
			}
			else
			{
				tracked_selector_value = Some(selector_value);
				selector_value
			};
			if wrote==0
			{
				let selectorname=latex_make_command_name(&tracked_selector_value.unwrap().to_string());
				figure_tikz.push_str(&format!(r#"
\begin{{experimentfigure}}
	\begin{{center}}
	\bgroup\tikzexternalize[prefix=externalized-legends/]
	\tikzsetnextfilename{{legend-{prefix}-{selectorname}}}
	\ref{{legend-{prefix}-{selectorname}}}\egroup\\"#,selectorname=selectorname,prefix=prefix));
			}
			wrote+=1;
			let mut raw_plots=String::new();
			let mut good_plots=0;
			while *koffset<kaverages.len() && *selector_value_to_use==kaverages[*koffset].selector
			{
				let legend_value=&kaverages[*koffset].legend;
				let legend_tex_entry= latex_protect_text(&legend_value.to_string());
				let legend_tex_id = latex_make_command_name(&legend_value.to_string());
				//all_legend_tex_id.insert(legend_tex_id.clone());//XXX Can we avoid the clone when not necessary?
				if !all_legend_tex_id_set.contains(&legend_tex_id)
				{
					all_legend_tex_id_set.insert(legend_tex_id.clone());
					all_legend_tex_id_vec.push(legend_tex_id.clone());
				}
				all_legend_tex_entry.insert(format!("\\def\\{}text{{{}}}\n",legend_tex_id,legend_tex_entry));
				//all_legend_tex_entry.insert(format!("\\expandafter\\def\\csname {}text\\endcsname{{{}}}\n",legend_tex_id,legend_tex_entry));
				raw_plots.push_str(r"\addplot[");
				raw_plots.push_str(&legend_tex_id);
				raw_plots.push_str(r"] coordinates{");
				let mut drawn_points=0;
				let mut to_draw:Vec<(f32,f32,f32,f32)> = Vec::with_capacity(kaverages.len());
				while *koffset<kaverages.len() && *selector_value_to_use==kaverages[*koffset].selector && *legend_value==kaverages[*koffset].legend
				{
					let (abscissa_average,abscissa_deviation)=kaverages[*koffset].abscissa;
					let (ordinate_average,ordinate_deviation)=kaverages[*koffset].ordinate;
					if let (Some(x),Some(y))=(abscissa_average,ordinate_average)
					{
						let abscissa_deviation=abscissa_deviation.unwrap_or(0f32);
						let ordinate_deviation=ordinate_deviation.unwrap_or(0f32);
						//if abscissa_deviation.abs()>0.01*x.abs() || ordinate_deviation.abs()>0.01*y.abs()
						//{
						//	raw_plots.push_str(&format!("({},{}) +- ({},{})",x,y,abscissa_deviation,ordinate_deviation));
						//}
						//else
						//{
						//	raw_plots.push_str(&format!("({},{})",x,y));
						//}
						to_draw.push( (x,y,abscissa_deviation,ordinate_deviation) );
						drawn_points+=1;
					}
					for git_id in kaverages[*koffset].git_set.iter()
					{
						all_git_ids.insert(git_id.clone());
					}
					*koffset+=1;
				}
				if drawn_points>=1
				{
					let cmp = | x:&f32, y:&f32 | if x==y { std::cmp::Ordering::Equal } else { if x<y {std::cmp::Ordering::Less} else {std::cmp::Ordering::Greater}};
					let to_draw_x_min = to_draw.iter().map(|t|t.0).min_by(cmp).expect("no points");
					let to_draw_x_max = to_draw.iter().map(|t|t.0).max_by(cmp).expect("no points");
					let to_draw_y_min = if let Some(y)=kd.min_ordinate
					{
						y
					}
					else
					{
						to_draw.iter().map(|t|t.1).min_by(cmp).expect("no points")
					};
					let to_draw_y_max = if let Some(y) = kd.max_ordinate
					{
						y
					}
					else
					{
						to_draw.iter().map(|t|t.1).max_by(cmp).expect("no points")
					};
					let x_range = to_draw_x_max - to_draw_x_min;
					let y_range = to_draw_y_max - to_draw_y_min;
					for (x,y,dx,dy) in to_draw
					{
						if dx.abs()*20f32 > x_range || dy.abs()*20f32 > y_range
						{
							raw_plots.push_str(&format!("({},{}) +- ({},{})",x,y,dx,dy));
						}
						else
						{
							raw_plots.push_str(&format!("({},{})",x,y));
						}
					}
				}
				if kind_index==0
				{
					//raw_plots.push_str(r"};\addlegendentry{\csname ");
					//raw_plots.push_str(&legend_tex_id);
					//raw_plots.push_str("text\\endcsname}");// '\addlegendentry{}' does not use a smicolon
					raw_plots.push_str(r"};\addlegendentry{\");
				}
				else
				{
					//raw_plots.push_str(r"};");
					raw_plots.push_str(r"};%\addlegendentry{\");
				}
				raw_plots.push_str(&legend_tex_id);
				raw_plots.push_str("text}\n");// '\addlegendentry{}' does not use a smicolon
				if drawn_points>1
				{
					good_plots+=1;
				}
			}
			if good_plots==0 { figure_tikz.push_str(&format!("skipped bad plot.\\\\")); continue; }
			//\begin{{tikzpicture}}[baseline,trim left=(left trim point),trim axis right,remember picture]
			//\path (yticklabel cs:0) ++(-1pt,0pt) coordinate (left trim point);
			let selectorname=latex_make_command_name(&selector_value_to_use.to_string());
			let tikzname=format!("{}-selector{}-kind{}",prefix,selectorname,kind_index);
			figure_tikz.push_str(&format!(r#"
	\tikzsetnextfilename{{external-{tikzname}}}
	\begin{{tikzpicture}}[baseline,remember picture]
	\begin{{axis}}[
		automatically generated axis,
		{kind_index_style},{legend_to_name},
		%%ybar interval=0.6,
		% ymin=%(ymin)s,
		% ymax=%(ymax)s,
		{ymin_string}%
		{ymax_string}%
		%%enlargelimits=false,
		ymajorgrids=true,
		yminorgrids=true,
		xmajorgrids=true,
		mark options=solid,
		minor y tick num=4,
		% %(xlabel)s%(ylabel)s
		xlabel={{{xlabel_string}}},
		ylabel={{{ylabel_string}}},
		%%legend style={{at={{(1.05,1.0)}},anchor=north west}},
		%%legend style={{opacity=0.7,at={{(0.99,0.99)}},anchor=north east}},
		%%legend style={{at={{(0.00,1.01)}},anchor=south west,font=\scriptsize}},legend columns=3,transpose legend,legend cell align=left,
		%%legend style={{at={{(0.00,1.01)}},anchor=south west,font=\scriptsize}},legend columns=2,legend cell align=left,
		% %(barprop)s
		%%every x tick label/.append style={{anchor=base,yshift=-7}},
	]
{plots_string}	\end{{axis}}
	%\pgfresetboundingbox\useasboundingbox (y label.north west) (current axis.north east) ($(current axis.outer north west)!(current axis.north east)!(current axis.outer north east)$);
	\end{{tikzpicture}}"#,tikzname=tikzname,kind_index_style=if kind_index==0{"first kind,"} else {"posterior kind,"},ymin_string=ymin[kind_index],ymax_string=ymax[kind_index],xlabel_string=kd.label_abscissas,ylabel_string=kd.label_ordinates,plots_string=raw_plots,legend_to_name=if kind_index==0{format!("legend to name=legend-{}-{}",prefix,selectorname)}else{"".to_string()}));
		}
		if wrote==0
		{
			break;
		}
		let selector_tex_caption=Some(latex_protect_text(&tracked_selector_value.unwrap().to_string()));
		figure_tikz.push_str(&format!(r#"
	\end{{center}}
	\caption{{\captionprologue {caption}}}
\end{{experimentfigure}}
"#,caption=selector_tex_caption.unwrap()));
		tikz.push_str(&figure_tikz);
		//figure_index+=1;
	}
	let amount_string=
	{
		let (done,total) = amount_experiments;
		if done==total {format!("all {} done",done)} else {format!("{} of {}",done,total)}
	};
	let folder=path.file_name().unwrap().to_str().unwrap();
	let git_id=get_git_id();
	let title=format!("{}/{} ({})",folder,pdf_filename,amount_string);
	let header=format!("\\tiny {}:{} ({})\\\\pdflatex on \\today\\\\git\\_id={}",latex_protect_text(folder),latex_protect_text(&pdf_filename),amount_string,latex_protect_text(git_id));
	let shared_prelude=format!(r#"
%% -- common pgfplots prelude --
\newenvironment{{experimentfigure}}{{\begin{{figure}}[H]\tikzexternalenable}}{{\tikzexternaldisable\end{{figure}}}}
%\newenvironment{{experimentfigure}}{{\begin{{figure*}}}}{{\end{{figure*}}}}
\pgfplotsset{{compat=newest}}
\pgfplotsset{{minor grid style={{dashed,very thin, color=blue!15}}}}
\pgfplotsset{{major grid style={{very thin, color=black!30}}}}
\pgfplotsset{{automatically generated axis/.style={{
		%default: height=207pt, width=240pt. 240:207 ~~ 7:6
		%height=115pt,%may fit 3figures with 1 line caption
		height=105pt,%may fit 3figures with 2 line caption
		width=174pt,
		scaled ticks=false,
		xticklabel style={{font=\tiny,/pgf/number format/.cd, fixed}},% formattin ticks' labels
		yticklabel style={{font=\tiny,/pgf/number format/.cd, fixed}},% formattin ticks' labels
		x label style={{at={{(ticklabel cs:0.5, -5pt)}},name={{x label}},anchor=north,font=\scriptsize}},
		y label style={{at={{(ticklabel cs:0.5, -5pt)}},name={{y label}},anchor=south,font=\scriptsize}},
	}},
	first kind/.style={{
		%The first axis on each line of plots
		%legend style={{overlay,at={{(0.50,1.05)}},anchor=south,font=\scriptsize,fill=none}},
		%legend style={{at={{(0.00,1.01)}},anchor=south west,font=\scriptsize,fill=none}},
		%legend style={{at={{($(axis description cs:0.00,1.01)!(current page.center)!(axis description cs:1.00,1.01)$)}},anchor=south,font=\scriptsize,fill=none}},
		legend style={{font=\scriptsize,fill=none}},
		legend columns=2,legend cell align=left,
	}},
	posterior kind/.style={{
		%Axis following the first on each line of plots
		%legend style={{at={{(0.50,1.05)}},overlay,anchor=south,font=\tiny,fill=none}},
		legend style={{draw=none}},
	}},
}}
\tikzset{{
	automatically generated plot/.style={{
		%/pgfplots/error bars/.cd,error bar style={{ultra thick}},x dir=both, y dir=both,
		/pgfplots/error bars/x dir=both,
		/pgfplots/error bars/y dir=both,
		/pgfplots/error bars/x explicit,
		/pgfplots/error bars/y explicit,
		/pgfplots/error bars/error bar style={{ultra thin,solid}},
		/tikz/mark options={{solid}},
	}},
	%/pgf/images/aux in dpth=true,
}}"#);
	let mut local_prelude=format!(r#"
%% -- experiment-local prelude
\newcommand\captionprologue{{X: }}
\newcommand\experimenttitle{{{title_string}}}
\newcommand\experimentheader{{{header_string}}}
"#,title_string=title,header_string=header);
	let tikz_colors=["red","green","blue"];
	let tikz_pens=["solid","dashed","dotted"];
	let tikz_marks=["o","square","triangle"];
	let mut color_index=0;
	let mut pen_index=0;
	let mut mark_index=0;
	for legend_tex_id in all_legend_tex_id_vec
	{
		local_prelude.push_str(r"\tikzset{");
		local_prelude.push_str(&legend_tex_id);
		local_prelude.push_str(&format!("/.style={{automatically generated plot,{},{},mark={}}}}}\n",tikz_colors[color_index],tikz_pens[pen_index],tikz_marks[mark_index]));
		color_index+=1;
		pen_index+=1;
		mark_index+=1;
		while color_index>=tikz_colors.len()
		{
			color_index-=tikz_colors.len();
			pen_index+=1;
			mark_index+=1;
		}
		while pen_index>=tikz_pens.len()
		{
			pen_index-=tikz_pens.len();
			mark_index+=1;
		}
		while mark_index>=tikz_marks.len()
		{
			mark_index-=tikz_marks.len();
		}
	}
	for legend_tex_entry in all_legend_tex_entry
	{
		local_prelude.push_str(&legend_tex_entry);
	}
	//writeln!(tex_file,"{}",local_prelude).unwrap();
	//writeln!(tex_file,"{}",tikz).unwrap();
	writeln!(tex_file,r#"{shared_prelude}
\bgroup
{local_prelude}
%% -- henceafter the data
{data_string}
\egroup
"#,shared_prelude=shared_prelude,local_prelude=local_prelude,data_string=tikz).unwrap();
	let pdf_path=path.join(&pdf_filename);
	println!("Creating {:?}",pdf_path);
	let tmp_path=path.join("tikz_tmp");
	if !tmp_path.is_dir()
	{
		fs::create_dir(&tmp_path).expect("Something went wrong when creating the tikz tmp directory.");
	}
	let tmp_path_externalized=tmp_path.join("externalized");
	if !tmp_path_externalized.is_dir()
	{
		fs::create_dir(&tmp_path_externalized).expect("Something went wrong when creating the tikz externalized directory.");
	}
	let tmp_path_externalized_legends=tmp_path.join("externalized-legends");
	if !tmp_path_externalized_legends.is_dir()
	{
		fs::create_dir(&tmp_path_externalized_legends).expect("Something went wrong when creating the tikz externalized-legends directory.");
	}
	let main_cfg_contents=
	{
		let cfg=path.join("main.cfg");
		let mut cfg_file=File::open(&cfg).expect("main.cfg could not be opened");
		let mut cfg_contents = String::new();
		cfg_file.read_to_string(&mut cfg_contents).expect("something went wrong reading main.cfg");
		cfg_contents
	};
	let all_git_formatted=
	{
		let core = all_git_ids.iter().map(|s|format!("\\item {}",latex_protect_text(s))).collect::<Vec<String>>().join("\n");
		format!("\\begin{{itemize}}\n{}\n\\end{{itemize}}",core)
	};
	let whole_tex=format!(r#"
\documentclass[a4paper, 12pt, fleqn]{{article}}
\usepackage[latin1]{{inputenc}}
\usepackage{{amsfonts}}
\usepackage{{amssymb}}
\usepackage{{amsmath}}
\usepackage{{graphicx}}
\usepackage{{amsthm}}
\usepackage{{color}}

%\usepackage[cm]{{fullpage}}
\usepackage[paper=a4paper,margin=1cm,includehead=true]{{geometry}}

\usepackage{{float}}
\usepackage{{tikz}}
\usepackage{{pgfplots}}
\usetikzlibrary{{calc,external}}
\tikzexternaldisable
\tikzexternalize[prefix=externalized/]

\usepackage[bookmarks=true]{{hyperref}}

{shared_prelude}

{local_prelude}

\newcommand\autor{{Cantabrian Agile Modular Interconnection Open Simulator}}
\hypersetup{{
	unicode=false,
	pdftoolbar=true,
	pdfmenubar=true,
	pdffitwindow=true,
	pdftitle={{\experimenttitle}},
	pdfauthor={{\autor}},
	pdfsubject={{}},
	pdfnewwindow=true,
	pdfkeywords={{}},
	pdfpagemode=None,
	colorlinks=false,
	linkcolor=red,
	citecolor=green,
	filecolor=magenta,
	urlcolor=cyan,
	pdfborder={{0 0 0}},
}}
\begin{{document}}
\pagestyle{{myheadings}}
\markright{{\experimentheader}}
{data_string}
\clearpage\tiny
{git_ids}
\begin{{verbatim}}
{cfg_string}
\end{{verbatim}}
\end{{document}}
"#,shared_prelude=shared_prelude,local_prelude=local_prelude,data_string=tikz,git_ids=all_git_formatted,cfg_string=main_cfg_contents);
	let tmpname=format!("{}-tmp",prefix);
	let tmpname_tex=format!("{}.tex",&tmpname);
	let tmpname_pdf=format!("{}.pdf",&tmpname);
	let whole_tex_path=tmp_path.join(&tmpname_tex);
	let mut whole_tex_file=File::create(&whole_tex_path).expect("Could not create whole tex temporal file.");
	writeln!(whole_tex_file,"{}",whole_tex).unwrap();
	for _ in 0..3
	{
		//With `remember picture` we need at least two passes.
		//And externalize with legend to names seems to require three passes.
		let _pdflatex=Command::new("pdflatex")
			.current_dir(&tmp_path)
			.arg("--shell-escape")
			.arg(&tmpname_tex)
			.output()
			.expect("pdflatex failed to start");
	}
	let filepath=tmp_path.join(tmpname_pdf);
	//fs::copy(&tmp_path.join("tmp.pdf"),&pdf_path).expect("copying temporal pdf failed.");
	fs::copy(&filepath,&pdf_path).or_else(|err|Err(BackendError::CouldNotGenerateFile{filepath:filepath,io_error:Some(err)}))?;
	//fs::copy(&filepath,&pdf_path).map_or_else(|amount_copied|Ok(),|err|Err(BackendError::CouldNotGenerateFile{filepath:filepath,io_error:Some(err)}))
	Ok(())
}

/// Calculates the average and deviation of the values in a Vec.
fn standard_deviation(list:&Vec<ConfigurationValue>) -> (Option<f32>,Option<f32>)
{
	let list=values_to_f32(list);
	if list.len()==0
	{
		return (None,None);
	}
	if list.len()==1
	{
		return (Some(list[0]),None);
	}
	let total:f32=list.iter().sum();
	let average=total/list.len() as f32;
	let sum:f32=list.iter().map(|v|{
		let x= v-average;
		x*x
	}).sum();
	//let deviation=((sum/(list.len()-1)) as f64).sqrt() as f32;
	let deviation=
	{
		let x:f32=sum/(list.len()-1) as f32;
		x.sqrt()
	};
	(Some(average),Some(deviation))
}

