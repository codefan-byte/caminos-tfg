
use std::io::{self,Write,Read,Seek};
use std::collections::{BTreeMap};
use std::convert::TryInto;
use std::path::Path;
use std::fs::File;

use crate::config_parser::{self,ConfigurationValue,Expr};

///Given a list of vectors, `[A1,A2,A3,A4,...]`, `Ai` beging a `Vec<T>` and second vector `b:&Vec<T>=[b1,b2,b3,b4,...]`, each `bi:T`.
///It creates a list of vectors with each combination Ai+bj.
fn vec_product<T:Clone>(a:&Vec<Vec<T>>,b:&Vec<T>) -> Vec<Vec<T>>
{
	let mut r=vec![];
	for ae in a.iter()
	{
		for be in b.iter()
		{
			let mut new=ae.clone();
			new.push(be.clone());
			r.push(new);
		}
	}
	r
}

///Expands all the inner ConfigurationValue::Experiments given out a single ConfigurationValue::Experiments
///whose elements are free of them.
pub fn flatten_configuration_value(value:&ConfigurationValue) -> ConfigurationValue
{
	let mut names = BTreeMap::new();//name -> range
	let experiments = flatten_configuration_value_gather_names(value, &mut names);
	//println!("got names {:?}",names);
	expand_named_experiments_range(experiments,&names)
}


fn flatten_configuration_value_gather_names(value:&ConfigurationValue, names:&mut BTreeMap<String,usize>) -> ConfigurationValue
{
	match value
	{
		&ConfigurationValue::Object(ref name, ref list) =>
		{
			let mut r=vec![ vec![] ];
			for &(ref name, ref v) in list
			{
				let fv=flatten_configuration_value_gather_names(v,names);
				if let ConfigurationValue::Experiments(vlist) = fv
				{
					let factor=vlist.iter().map(|x|(name.clone(),x.clone())).collect::<Vec<(String,ConfigurationValue)>>();
					r=vec_product(&r,&factor);
				}
				else
				{
					for x in r.iter_mut()
					{
						x.push((name.clone(),fv.clone()));
					}
				}
			}
			ConfigurationValue::Experiments(r.iter().map(|values|ConfigurationValue::Object(name.clone(),values.clone())).collect())
		},
		&ConfigurationValue::Array(ref list) =>
		{
			let mut r=vec![ vec![] ];
			for ref v in list
			{
				let fv=flatten_configuration_value_gather_names(v,names);
				if let ConfigurationValue::Experiments(vlist) = fv
				{
					//let factor=vlist.iter().map(|x|x.clone()).collect::<Vec<ConfigurationValue>>();
					//r=vec_product(&r,&factor);
					r=vec_product(&r,&vlist);
				}
				else
				{
					for x in r.iter_mut()
					{
						x.push(fv.clone());
					}
				}
			}
			ConfigurationValue::Experiments(r.iter().map(|values|ConfigurationValue::Array(values.clone())).collect())
		},
		&ConfigurationValue::Experiments(ref experiments) =>
		{
			let mut r:Vec<ConfigurationValue>=vec![];
			for experiment in experiments
			{
				let flat=flatten_configuration_value_gather_names(experiment,names);
				if let ConfigurationValue::Experiments(ref flist) = flat
				{
					r.extend(flist.iter().map(|x|x.clone()));
				}
				else
				{
					r.push(flat);
				}
			}
			ConfigurationValue::Experiments(r)
		},
		&ConfigurationValue::NamedExperiments(ref name, ref experiments) =>
		{
			if let Some(&size) = names.get(name)
			{
				if size != experiments.len()
				{
					panic!("{}! has different lengths {} vs {}",name,size,experiments.len());
				}
			}
			else
			{
				names.insert(name.to_string(),experiments.len());
			}
			value.clone()
		},
		&ConfigurationValue::Where(ref v, ref _expr) =>
		{
			flatten_configuration_value_gather_names(v,names)//FIXME, filterby expr
		},
		_ => value.clone(),
	}
}

fn expand_named_experiments_range(experiments:ConfigurationValue, names:&BTreeMap<String,usize>) -> ConfigurationValue
{
	let mut r = experiments;
	for name in names.keys()
	{
		let size=*names.get(name).unwrap();
		//r=ConfigurationValue::Experiments((0..size).map(|index|{
		//	let mut context = BTreeMap::new();
		//	context.insert(key,index);
		//	match particularize_named_experiments_selected(experiments,&context)
		//	{
		//		ConfigurationValue::Experiments(ref exps) => exps,
		//		x => &vec![x],
		//	}.iter()
		//}).flatten().collect());
		let collected : Vec<Vec<ConfigurationValue>>= (0..size).map(|index|{
			let mut context : BTreeMap<String,usize> = BTreeMap::new();
			context.insert(name.to_string(),index);
			match particularize_named_experiments_selected(&r,&context)
			{
				ConfigurationValue::Experiments(exps) => exps,
				x => vec![x],
			}
		}).collect();
		r=ConfigurationValue::Experiments(collected.into_iter().map(|t|t.into_iter()).flatten().collect());
	}
	r
}

fn particularize_named_experiments_selected(value:&ConfigurationValue, names:&BTreeMap<String,usize>) -> ConfigurationValue
{
	match value
	{
		&ConfigurationValue::Object(ref name, ref list) =>
		{
			let plist = list.iter().map(|(key,x)|(key.to_string(),particularize_named_experiments_selected(x,names))).collect();
			ConfigurationValue::Object(name.to_string(),plist)
		},
		&ConfigurationValue::Array(ref list) =>
		{
			let plist = list.iter().map(|x|particularize_named_experiments_selected(x,names)).collect();
			ConfigurationValue::Array(plist)
		},
		&ConfigurationValue::Experiments(ref list) =>
		{
			let plist = list.iter().map(|x|particularize_named_experiments_selected(x,names)).collect();
			ConfigurationValue::Experiments(plist)
		},
		&ConfigurationValue::NamedExperiments(ref name, ref list) =>
		{
			if let Some(&index) = names.get(name)
			{
				list[index].clone()
			}
			else
			{
				value.clone()
			}
		},
		//&ConfigurationValue::Where(ref v, ref _expr) =>
		//{
		//	flatten_configuration_value_gather_names(v,names)//FIXME, filterby expr
		//},
		_ => value.clone(),
	}
}


///Just returns a `Context{configuration:<configuration>, result:<result>}`.
pub fn combine(experiment_index:usize, configuration:&ConfigurationValue, result:&ConfigurationValue) -> ConfigurationValue
{
	ConfigurationValue::Object(String::from("Context"),vec![
		(String::from("index"),ConfigurationValue::Number(experiment_index as f64)),
		(String::from("configuration"),configuration.clone()),
		(String::from("result"),result.clone()),
	])
}

/**Evaluates an expression given in a context.

For example the expression `=Alpha.beta` will return 42 for the context `Alpha{beta:42}`.

# Available functions

## Comparisons

Arguments `first` and `second`. It evaluates to `ConfigurationValue::{True,False}`.

* eq or equal
* lt

## Arithmetic

* add
* mul

Arguments `first` and `second`. It evaluates to `ConfigurationValue::Number`.

## if

Evaluates to whether its argument `true_expression` or `false_expression` depending on its `condition` argument.

## at

Evaluates to the element at `position` inside the array in `container`.

## AverageBins

Evaluates to an array smaller than the input `data`, as each `width` entries are averaged into a single one.

## FileExpression

Evaluates an `expression` including the file `filename` into the current context under the name `file_data`. For example `FileExpression{filename:"peak.cfg",expression:at{container:file_data,position:index}}.maximum_value` to access into the `peak.cfg` file and evaluating the expression `at{container:file_data,position:index}`. This example assume that `peak.cfg` read into `file_data` is an array and can be accessed by `index`, the entry of the associated execution. The value returned by `FileExpression` is then accessed to its `maximum_value` field.


**/
pub fn evaluate(expr:&Expr, context:&ConfigurationValue, path:&Path) -> ConfigurationValue
{
	match expr
	{
		&Expr::Equality(ref a,ref b) =>
		{
			let va=evaluate(a,context,path);
			let vb=evaluate(b,context,path);
			if va==vb
			{
				ConfigurationValue::True
			}
			else
			{
				ConfigurationValue::False
			}
		},
		&Expr::Literal(ref s) => ConfigurationValue::Literal(s.clone()),
		&Expr::Number(f) => ConfigurationValue::Number(f),
		&Expr::Ident(ref s) => match context
		{
			&ConfigurationValue::Object(ref _name, ref attributes) =>
			{
				for &(ref attr_name,ref attr_value) in attributes.iter()
				{
					if attr_name==s
					{
						return attr_value.clone();
					}
				};
				panic!("There is not attribute {} in {}",s,context);
			},
			_ => panic!("Cannot evaluate identifier in non-object"),
		},
		&Expr::Member(ref expr, ref attribute) =>
		{
			let value=evaluate(expr,context,path);
			match value
			{
				ConfigurationValue::Object(ref _name, ref attributes) =>
				{
					for &(ref attr_name,ref attr_value) in attributes.iter()
					{
						if attr_name==attribute
						{
							return attr_value.clone();
						}
					};
					panic!("There is not member {} in {}",attribute,value);
				},
				_ => panic!("There is no member {} in {}",attribute,value),
			}
		},
		&Expr::Parentheses(ref expr) => evaluate(expr,context,path),
		&Expr::Name(ref expr) =>
		{
			let value=evaluate(expr,context,path);
			match value
			{
				ConfigurationValue::Object(ref name, ref _attributes) => ConfigurationValue::Literal(name.clone()),
				_ => panic!("{} has no name as it is not object",value),
			}
		},
		&Expr::FunctionCall(ref function_name, ref arguments) =>
		{
			match function_name.as_ref()
			{
				"eq" | "equal" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context,path));
							},
							"second" =>
							{
								second=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of lt not given.");
					let second=second.expect("second argument of lt not given.");
					//allow any type
					if first==second { ConfigurationValue::True } else { ConfigurationValue::False }
				}
				"lt" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context,path));
							},
							"second" =>
							{
								second=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of lt not given.");
					let second=second.expect("second argument of lt not given.");
					let first=match first
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("first argument of lt evaluated to a non-number ({}:?)",first),
					};
					let second=match second
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("second argument of lt evaluated to a non-number ({}:?)",second),
					};
					if first<second { ConfigurationValue::True } else { ConfigurationValue::False }
				}
				"if" =>
				{
					let mut condition=None;
					let mut true_expression=None;
					let mut false_expression=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"condition" =>
							{
								condition=Some(evaluate(val,context,path));
							},
							"true_expression" =>
							{
								true_expression=Some(evaluate(val,context,path));
							},
							"false_expression" =>
							{
								false_expression=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let condition=condition.expect("condition argument of if not given.");
					let true_expression=true_expression.expect("true_expression argument of if not given.");
					let false_expression=false_expression.expect("false_expression argument of if not given.");
					let condition = match condition
					{
						ConfigurationValue::True => true,
						ConfigurationValue::False => false,
						_ => panic!("if function condition did not evaluate into a Boolean value."),
					};
					if condition { true_expression } else { false_expression }
				}
				"add" | "plus" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context,path));
							},
							"second" =>
							{
								second=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of and not given.");
					let second=second.expect("second argument of and not given.");
					let first=match first
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("first argument of {} evaluated to a non-number ({}:?)",function_name,first),
					};
					let second=match second
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("second argument of {} evaluated to a non-number ({}:?)",function_name,second),
					};
					ConfigurationValue::Number(first+second)
				}
				"sub" | "minus" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context,path));
							},
							"second" =>
							{
								second=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of and not given.");
					let second=second.expect("second argument of and not given.");
					let first=match first
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("first argument of {} evaluated to a non-number ({}:?)",function_name,first),
					};
					let second=match second
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("second argument of {} evaluated to a non-number ({}:?)",function_name,second),
					};
					ConfigurationValue::Number(first-second)
				}
				"mul" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context,path));
							},
							"second" =>
							{
								second=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of and not given.");
					let second=second.expect("second argument of and not given.");
					let first=match first
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("first argument of {} evaluated to a non-number ({}:?)",function_name,first),
					};
					let second=match second
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("second argument of {} evaluated to a non-number ({}:?)",function_name,second),
					};
					ConfigurationValue::Number(first*second)
				}
				"div" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context,path));
							},
							"second" =>
							{
								second=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of and not given.");
					let second=second.expect("second argument of and not given.");
					let first=match first
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("first argument of {} evaluated to a non-number ({}:?)",function_name,first),
					};
					let second=match second
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("second argument of {} evaluated to a non-number ({}:?)",function_name,second),
					};
					ConfigurationValue::Number(first/second)
				}
				"log" =>
				{
					let mut arg=None;
					let mut base=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"arg" =>
							{
								arg=Some(evaluate(val,context,path));
							},
							"base" =>
							{
								base=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let arg=arg.expect("arg argument of and not given.");
					let arg=match arg
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("arg argument of {} evaluated to a non-number ({}:?)",function_name,arg),
					};
					let base=match base
					{
						None => 1f64.exp(),
						Some(ConfigurationValue::Number(x)) => x,
						Some(other) => panic!("base argument of {} evaluated to a non-number ({}:?)",function_name,other),
					};
					ConfigurationValue::Number(arg.log(base))
				}
				"at" =>
				{
					let mut container=None;
					let mut position=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"container" =>
							{
								container=Some(evaluate(val,context,path));
							},
							"position" =>
							{
								position=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.expect("container argument of at not given.");
					let position=position.expect("position argument of at not given.");
					let container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("conatiner argument of at evaluated to a non-array ({}:?)",container),
					};
					let position=match position
					{
						ConfigurationValue::Number(x) => x as usize,
						_ => panic!("position argument of at evaluated to a non-number ({}:?)",position),
					};
					container[position].clone()
				}
				"AverageBins" =>
				{
					let mut data = None;
					let mut width = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"data" =>
							{
								data=Some(evaluate(val,context,path));
							},
							"width" =>
							{
								width=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let data=data.expect("data argument of at not given.");
					let width=width.expect("width argument of at not given.");
					let data=match data
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of AverageBins evaluated to a non-array ({}:?)",data),
					};
					let width=match width
					{
						ConfigurationValue::Number(x) => x as usize,
						_ => panic!("width argument of AverageBins evaluated to a non-number ({}:?)",width),
					};
					//TODO: do we want to include incomplete bins?
					//let n = (data.len()+width-1)/width;
					let n = data.len()/width;
					//let mut result = Vec::with_capacity(n);
					let mut iter = data.into_iter();
					let result =(0..n).map(|_|{
						let mut total = 0f64;
						for _ in 0..width
						{
							total += match iter.next().unwrap()
							{
								ConfigurationValue::Number(x) => x,
								//x => panic!("AverageBins received {:?}",x),
								_ => std::f64::NAN,
							}
						}
						ConfigurationValue::Number(total/width as f64)
					}).collect();
					ConfigurationValue::Array(result)
				}
				"FileExpression" =>
				{
					let mut filename = None;
					let mut expression = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"filename" =>
							{
								filename=Some(evaluate(val,context,path));
							},
							"expression" =>
							{
								expression = Some(val);
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let filename=filename.expect("filename argument of at not given.");
					let expression=expression.expect("expression argument of at not given.");
					let filename = match filename
					{
						ConfigurationValue::Literal(s) => s,
						_ => panic!("filename argument of FileExpression evaluated to a non-literal ({}:?)",filename),
					};
					let file_path = path.join(filename);
					let file_data={
						let mut data = ConfigurationValue::None;
						let mut file_contents = String::new();
						let mut cfg_file=File::open(&file_path).expect("data file could not be opened");
						let mut try_raw = true;
						let mut try_binary = false;
						if try_raw
						{
							match cfg_file.read_to_string(&mut file_contents)
							{
								Ok(_) => (),
								Err(_e) => {
									//println!("Got error {} when reading",e);//too noisy
									try_raw = false;
									try_binary = true;
								}
							}
						}
						if try_raw
						{
							let parsed_file=match config_parser::parse(&file_contents)
							{
								Err(x) => panic!("error parsing data file {:?}: {:?}",file_path,x),
								Ok(x) => x,
							};
							data = match parsed_file
							{
								config_parser::Token::Value(value) =>
								{
									value
								},
								_ => panic!("Not a value. Got {:?}",parsed_file),
							}
						}
						if try_binary
						{
							let mut contents = vec![];
							cfg_file.rewind().expect("some problem rewinding data file");
							cfg_file.read_to_end(&mut contents).expect("something went wrong reading binary data");
							data=config_from_binary(&contents,0).expect("something went wrong while deserializing binary data");
						}
						data
					};
					let context = match context{
						ConfigurationValue::Object(name, data) =>
						{
							let mut content = data.clone();
							content.push( (String::from("file_data"), file_data ) );
							ConfigurationValue::Object(name.to_string(),content)
						},
						_ => panic!("wrong context"),
					};
					evaluate( expression, &context, path)
				}
				"map" =>
				{
					let mut container = None;
					let mut binding = None;
					let mut expression = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"container" =>
							{
								container=Some(evaluate(val,context,path));
							},
							"binding" =>
							{
								binding=Some(evaluate(val,context,path));
							},
							"expression" =>
							{
								expression = Some(val);
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.expect("container argument of at not given.");
					let expression=expression.expect("expression argument of at not given.");
					let binding=match binding
					{
						None => "x".to_string(),
						Some(ConfigurationValue::Literal(s)) => s.to_string(),
						Some(other) => panic!("{:?} cannot be used as binding variable",other),
					};
					let container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({}:?)",container),
					};
					let container = container.into_iter().map(|item|{
						let context = match context{
							ConfigurationValue::Object(name, data) =>
							{
								let mut content = data.clone();
								content.push( (String::from(binding.clone()), item ) );
								ConfigurationValue::Object(name.to_string(),content)
							},
							_ => panic!("wrong context"),
						};
						evaluate( expression, &context, path)
					}).collect();
					ConfigurationValue::Array(container)
				}
				"slice" =>
				{
					let mut container = None;
					let mut start = None;
					let mut end = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"container" =>
							{
								container=Some(evaluate(val,context,path));
							},
							"start" =>
							{
								start= match evaluate(val,context,path)
								{
									ConfigurationValue::Number(n) => Some(n as usize),
									_ => panic!("the start argument of slice must be a number"),
								};
							},
							"end" =>
							{
								end= match evaluate(val,context,path)
								{
									ConfigurationValue::Number(n) => Some(n as usize),
									_ => panic!("the start argument of slice must be a number"),
								};
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.expect("container argument of at not given.");
					let container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({}:?)",container),
					};
					let start=start.unwrap_or(0);
					let end=match end
					{
						None => container.len(),
						Some(n) => n.min(container.len()),
					};
					let container = container[start..end].to_vec();
					ConfigurationValue::Array(container)
				}
				"sort" =>
				{
					let mut container = None;
					let mut expression = None;
					let mut binding = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"container" =>
							{
								container=Some(evaluate(val,context,path));
							},
							"expression" =>
							{
								expression=Some(val);
							},
							"binding" =>
							{
								binding=Some(evaluate(val, context, path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.expect("container argument of at not given.");
					let mut container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({}:?)",container),
					};
					let f:Box< dyn Fn(&ConfigurationValue,&ConfigurationValue)->std::cmp::Ordering > = match expression
					{
						None => Box::new(|a:&ConfigurationValue,b:&ConfigurationValue|a.partial_cmp(b).unwrap()),
						Some(expr) =>
						{
							let binding=match binding
							{
								None => "x".to_string(),
								Some(ConfigurationValue::Literal(s)) => s.to_string(),
								Some(other) => panic!("{:?} cannot be used as binding variable",other),
							};
							Box::new(move |a,b|{
								let context = match context
								{
									ConfigurationValue::Object(name, data) =>
									{
										let mut content = data.clone();
										content.push( (String::from(binding.clone()), a.clone() ) );
										ConfigurationValue::Object(name.to_string(),content)
									},
									_ => panic!("wrong context"),
								};
								let a = evaluate(expr, &context, path);
								let context = match context
								{
									ConfigurationValue::Object(name, data) =>
									{
										let mut content = data.clone();
										content.push( (String::from(binding.clone()), b.clone() ) );
										ConfigurationValue::Object(name.to_string(),content)
									},
									_ => panic!("wrong context"),
								};
								let b = evaluate(expr, &context, path);
								a.partial_cmp(&b).unwrap()
							})
						},
					};
					//container.sort_by(|a,b|a.partial_cmp(b).unwrap());
					container.sort_by(f);
					ConfigurationValue::Array(container)
				}
				"last" =>
				{
					let mut container = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"container" =>
							{
								container=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.unwrap_or_else(||panic!("container argument of {} not given.",function_name));
					let container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({}:?)",container),
					};
					container.last().expect("there is not last element in the array").clone()
				}
				"number_or" =>
				// Returns the argument unchanged if it is a number, otherwise return the default value.
				{
					let mut arg = None;
					let mut default = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"arg" =>
							{
								arg=Some(evaluate(val,context,path));
							},
							"default" =>
							{
								default=Some(evaluate(val,context,path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let arg=arg.expect("arg argument of number_or not given.");
					let default=default.expect("default argument of number_or not given.");
					match arg
					{
						ConfigurationValue::Number(n) => ConfigurationValue::Number(n),
						_ => default,
					}
				}
				"filter" =>
				{
					let mut container = None;
					let mut expression = None;
					let mut binding = None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"container" =>
							{
								container=Some(evaluate(val,context,path));
							},
							"expression" =>
							{
								expression=Some(val);
							},
							"binding" =>
							{
								binding=Some(evaluate(val, context, path));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.expect("container argument of filter not given.");
					let expression=expression.expect("expression argument of filter not given.");
					let binding = match binding
					{
						None => String::from("x"),
						Some(ConfigurationValue::Literal(s)) => s,
						Some(b) => panic!("binding argument of filter evaluated to a non-literal ({}:?)",b),
					};
					let container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({:?})",container),
					};
					let container = container.into_iter().filter(|item|
					{
						let context = match context{
							ConfigurationValue::Object(name, data) =>
							{
								let mut content = data.clone();
								content.push( (String::from(binding.clone()), item.clone() ) );
								ConfigurationValue::Object(name.to_string(),content)
							},
							_ => panic!("wrong context"),
						};
						let b = evaluate(expression,&context,path);
						match b
						{
							ConfigurationValue::True => true,
							ConfigurationValue::False => false,
							b => panic!("filter expression evaluated to a non-Boolean ({:?})",b),
						}
					}).collect();
					ConfigurationValue::Array(container)
				}
				_ => panic!("Unknown function `{}'",function_name),
			}
		}
	}
}

/// Evaluate some expressions inside a ConfigurationValue
pub fn reevaluate(value:&ConfigurationValue, context:&ConfigurationValue, path:&Path) -> ConfigurationValue
{
	//if let &ConfigurationValue::Expression(ref expr)=value
	//{
	//	evaluate(expr,context,path)
	//}
	//else
	//{
	//	value.clone()
	//}
	match value
	{
		&ConfigurationValue::Expression(ref expr) => evaluate(expr,context,path),
		&ConfigurationValue::Array(ref l) => ConfigurationValue::Array(l.iter().map(|e|reevaluate(e,context,path)).collect()),
		_ => value.clone(),
	}
}

///Get a vector of `f32` from a vector of `ConfigurationValue`s, skipping non-numeric values.
pub fn values_to_f32(list:&Vec<ConfigurationValue>) -> Vec<f32>
{
	list.iter().filter_map(|v|match v{
		&ConfigurationValue::Number(f) => Some(f as f32),
		_ => None
	}).collect()
}

///Get a vector of `f32` from a vector of `ConfigurationValue`s, skipping non-numeric values.
///It also counts the number of good, `None`, and other values.
pub fn values_to_f32_with_count(list:&Vec<ConfigurationValue>) -> (Vec<f32>,usize,usize,usize)
{
	let mut values = Vec::with_capacity(list.len());
	let mut good_count=0;
	let mut none_count=0;
	let mut other_count=0;
	for v in list
	{
		match v
		{
			&ConfigurationValue::Number(f) =>
			{
				values.push(f as f32);
				good_count+=1;
			},
			&ConfigurationValue::None => none_count+=1,
			_ => other_count+=1,
		}
	}
	(values,good_count,none_count,other_count)
}


///Convert a ConfigurationValue into a Vec<u8>.
///Intended to create binary files for result files.
pub fn config_to_binary(value:&ConfigurationValue) -> io::Result<Vec<u8>>
{
	//let mut map:BTreeMap<String,usize> = BTreeMap::new();
	//let mut vector:Vec<u8> = Vec::new();
	//config_into_binary(value,&mut vector,&mut map)?;
	//Ok(vector)
	let mut writer = BinaryConfigWriter::default();
	//writer.insert(value)?;
	writer.insert(value).unwrap_or_else(|e|panic!("error={:?} data={:?}",e,writer.vector));
	Ok(writer.take_vector())
}


#[derive(Debug,Default)]
pub struct BinaryConfigWriter
{
	vector:Vec<u8>,
	name_locations:BTreeMap<String,u32>
}

impl BinaryConfigWriter
{
	pub fn new() -> BinaryConfigWriter
	{
		Self::default()
	}
	pub fn take_vector(self) -> Vec<u8>
	{
		self.vector
	}
	///Append the binary version of a ConfigurationValue into a Vec<u8> using a map from names to locations inside the vector.
	///Returns the location at which it has been appended
	//pub fn config_into_binary(value:&ConfigurationValue, vector:&mut Vec<u8>, name_locations:&mut BTreeMap<String,usize>) -> io::Result<usize>
	pub fn insert(&mut self, value:&ConfigurationValue) -> io::Result<u32>
	{
		//Using little endian for everything, to allow moving the binary files between machines.
		//This is, we use to_le_bytes instead to_ne_bytes.
		let location:u32 = {
			//Align to 4 bytes
			const ALIGNMENT: usize = 4;
			let s:usize = self.vector.len();
			let r = s % ALIGNMENT;
			if r == 0 { s } else {
				let new = s + (ALIGNMENT-r);
				self.vector.resize(new, 0u8);
				new
			}.try_into().unwrap()
		};
		match value
		{
			&ConfigurationValue::Literal(ref name) => {
				self.vector.resize((location+2*4).try_into().unwrap(), 0u8);
				let loc:u32 = self.locate(name)?;
				let mut writer = &mut self.vector[location as usize..];
				writer.write_all(&0u32.to_le_bytes())?;
				writer.write_all(&loc.to_le_bytes())?;
				//match self.name_locations.get(name)
				//{
				//	Some(loc) =>{
				//		self.vector.write_all(&loc.to_le_bytes())?;
				//	},
				//	None =>{
				//		let loc = location+4;
				//		self.name_locations.insert(name.to_string(),loc);
				//		self.vector.write_all(&loc.to_ne_bytes())?;
				//		self.vector.write_all(name.as_bytes())?;
				//	},
				//};
			},
			&ConfigurationValue::Number(f) => {
				self.vector.write_all(&1u32.to_le_bytes())?;
				//using f: f64
				self.vector.write_all(&f.to_le_bytes())?;
			},
			&ConfigurationValue::Object(ref name, ref pairs) =>{
				let n:u32 = pairs.len().try_into().unwrap();
				let end = location + 8*n + 3*4;
				self.vector.resize(end as usize, 0u8);
				let loc:u32 = self.locate(name)?;
				//self.vector[location..].write_all(&2u32.to_le_bytes())?;
				let mut writer = &mut self.vector[location as usize..];
				//Write::write_all(&mut self.vector[location..],&2u32.to_le_bytes())?;
				writer.write_all(&2u32.to_le_bytes())?;
				//let mut writer = &mut self.vector[location + 4..];//this allows a drop for the string write before.
				writer.write_all(&loc.to_le_bytes())?;
				//match self.name_locations.get(name)
				//{
				//	Some(loc) =>{
				//		//self.vector[location+1*4..].write_all(&loc.to_le_bytes())?;
				//		writer.write_all(&loc.to_le_bytes())?;
				//	},
				//	None =>{
				//		let loc = end;
				//		self.name_locations.insert(name.to_string(),loc);
				//		writer.write_all(&loc.to_le_bytes())?;
				//		self.vector.write_all(name.as_bytes())?;
				//	},
				//};
				//let mut writer = &mut self.vector[location + 2*4..];//this allows a drop for the string write before.
				writer.write_all(&n.to_le_bytes())?;
				let base:usize = (location +3*4).try_into().unwrap();
				for (index,(key,val)) in pairs.iter().enumerate(){
					//write key
					let loc:u32 = self.locate(key)?;
					let mut writer = &mut self.vector[base + index*2*4 ..];
					writer.write_all(&loc.to_le_bytes())?;
					//match self.name_locations.get(key)
					//{
					//	Some(loc) =>{
					//		let mut writer = &mut self.vector[base + index*2*4 ..];
					//		writer.write_all(&loc.to_le_bytes())?;
					//	},
					//	None =>{
					//		let loc = self.vector.len();
					//		self.name_locations.insert(key.to_string(),loc);
					//		let mut writer = &mut self.vector[base + index*2*4 ..];
					//		writer.write_all(&loc.to_le_bytes())?;
					//		self.vector.write_all(key.as_bytes())?;
					//	},
					//};
					//write value
					//let loc = config_into_binary(val,self.vector,name_locations)?;
					let loc:u32 = self.insert(val)?;
					let mut writer = &mut self.vector[base + index*2*4 +4 ..];
					writer.write_all(&loc.to_le_bytes())?;
				}
			},
			&ConfigurationValue::Array(ref a) => {
				let n:u32 = a.len().try_into().unwrap();
				let end = location + 4*n + 2*4;
				self.vector.resize(end as usize, 0u8);
				let mut writer = &mut self.vector[location as usize..];
				writer.write_all(&3u32.to_le_bytes())?;
				writer.write_all(&n.to_le_bytes())?;
				let base:usize = (location +2*4).try_into().unwrap();
				for (index,val) in a.iter().enumerate(){
					let loc = self.insert(val)?;
					let mut writer = &mut self.vector[base + index*4 ..];
					writer.write_all(&loc.to_le_bytes())?;
				}
			},
			&ConfigurationValue::Experiments(ref list) => {
				let n:u32 = list.len().try_into().unwrap();
				let end = location + 4*n + 2*4;
				self.vector.resize(end as usize, 0u8);
				let mut writer = &mut self.vector[location as usize..];
				writer.write_all(&4u32.to_le_bytes())?;
				writer.write_all(&n.to_le_bytes())?;
				let base:usize = (location +2*4).try_into().unwrap();
				for (index,val) in list.iter().enumerate(){
					let loc = self.insert(val)?;
					let mut writer = &mut self.vector[base  + index*4 ..];
					writer.write_all(&loc.to_le_bytes())?;
				}
			},
			&ConfigurationValue::NamedExperiments(ref name, ref list) => {
				let n:u32 = list.len().try_into().unwrap();
				let end = location + 4*n + 3*4;
				self.vector.resize(end as usize, 0u8);
				let loc = self.locate(name)?;
				let mut writer = &mut self.vector[location as usize ..];
				writer.write_all(&5u32.to_le_bytes())?;
				writer.write_all(&loc.to_le_bytes())?;
				writer.write_all(&n.to_le_bytes())?;
				let base:usize = (location +3*4).try_into().unwrap();
				for (index,val) in list.iter().enumerate(){
					let loc = self.insert(val)?;
					let mut writer = &mut self.vector[base + index*4 ..];
					writer.write_all(&loc.to_le_bytes())?;
				}
			},
			&ConfigurationValue::True => self.vector.write_all(&6u32.to_le_bytes())?,
			&ConfigurationValue::False => self.vector.write_all(&7u32.to_le_bytes())?,
			&ConfigurationValue::Where(ref _id, ref _expr) => {
				//TODO: This is not yet supported
				//its id=8 is reserved
				self.vector.write_all(&8u32.to_le_bytes())?;
			},
			&ConfigurationValue::Expression(ref _expr) => {
				//TODO: This is not yet supported
				//its id=9 is reserved
				self.vector.write_all(&9u32.to_le_bytes())?;
			},
			&ConfigurationValue::None => self.vector.write_all(&10u32.to_le_bytes())?,
		}
		Ok(location.try_into().unwrap())
	}
	///Get a location with the name given. Insert it in the map and vector if necessary.
	fn locate(&mut self, name:&str) -> io::Result<u32>
	{
		Ok(match self.name_locations.get(name)
		{
			Some(loc) =>{
				*loc
			},
			None =>{
				let loc:u32 = self.vector.len().try_into().unwrap();
				self.name_locations.insert(name.to_string(),loc);
				self.vector.write_all(&(name.len() as u32).to_le_bytes())?;
				self.vector.write_all(name.as_bytes())?;
				loc
			},
		})
	}
}

///Read the value from the input at the given offset.
pub fn config_from_binary(data:&[u8], offset:usize) -> Result<ConfigurationValue,std::string::FromUtf8Error>
{
	let magic = u32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
	//println!(">>config_from_binary data.len={} offset={} magic={}",data.len(),offset,magic);
	match magic{
		0 => {
			let loc:usize = u32::from_le_bytes(data[offset+4..offset+8].try_into().unwrap()).try_into().unwrap();
			let size:usize = u32::from_le_bytes(data[loc..loc+4].try_into().unwrap()).try_into().unwrap();
			Ok(ConfigurationValue::Literal(String::from_utf8(data[loc+4..loc+4+size].to_vec())?))
		},
		1 => {
			let f= f64::from_le_bytes(data[offset+4..offset+4+8].try_into().unwrap());
			Ok(ConfigurationValue::Number(f))
		},
		2 => {
			let loc:usize = u32::from_le_bytes(data[offset+4..offset+2*4].try_into().unwrap()).try_into().unwrap();
			let n:usize = u32::from_le_bytes(data[offset+2*4..offset+3*4].try_into().unwrap()).try_into().unwrap();
			let size:usize = u32::from_le_bytes(data[loc..loc+4].try_into().unwrap()).try_into().unwrap();
			let name = String::from_utf8(data[loc+4..loc+4+size].to_vec())?;
			let mut pairs = Vec::with_capacity(n);
			for index in 0..n
			{
				let item_offset = offset+3*4 +index*2*4;
				let loc:usize = u32::from_le_bytes(data[item_offset..item_offset+4].try_into().unwrap()).try_into().unwrap();
				let size:usize = u32::from_le_bytes(data[loc..loc+4].try_into().unwrap()).try_into().unwrap();
				let key = String::from_utf8(data[loc+4..loc+4+size].to_vec())?;
				let loc:usize = u32::from_le_bytes(data[item_offset+4..item_offset+2*4].try_into().unwrap()).try_into().unwrap();
				let val = config_from_binary(data,loc)?;
				pairs.push( (key,val) );
			}
			Ok(ConfigurationValue::Object(name,pairs))
		},
		3 => {
			let n:usize = u32::from_le_bytes(data[offset+1*4..offset+2*4].try_into().unwrap()).try_into().unwrap();
			let mut a = Vec::with_capacity(n);
			for index in 0..n
			{
				let item_offset = offset+2*4 +index*4;
				let loc:usize = u32::from_le_bytes(data[item_offset..item_offset+4].try_into().unwrap()).try_into().unwrap();
				let val = config_from_binary(data,loc)?;
				a.push( val );
			}
			Ok(ConfigurationValue::Array(a))
		},
		4 => {
			let n:usize = u32::from_le_bytes(data[offset+1*4..offset+2*4].try_into().unwrap()).try_into().unwrap();
			let mut list = Vec::with_capacity(n);
			for index in 0..n
			{
				let item_offset = offset+2*4 +index*4;
				let loc:usize = u32::from_le_bytes(data[item_offset..item_offset+4].try_into().unwrap()).try_into().unwrap();
				let val = config_from_binary(data,loc)?;
				list.push( val );
			}
			Ok(ConfigurationValue::Experiments(list))
		},
		5 => {
			let loc:usize = u32::from_le_bytes(data[offset+4..offset+2*4].try_into().unwrap()).try_into().unwrap();
			let n:usize = u32::from_le_bytes(data[offset+2*4..offset+3*4].try_into().unwrap()).try_into().unwrap();
			let size:usize = u32::from_le_bytes(data[loc..loc+4].try_into().unwrap()).try_into().unwrap();
			let name = String::from_utf8(data[loc+4..loc+4+size].to_vec())?;
			let mut list = Vec::with_capacity(n);
			for index in 0..n
			{
				let item_offset = offset+3*4 +index*4;
				let loc:usize = u32::from_le_bytes(data[item_offset..item_offset+4].try_into().unwrap()).try_into().unwrap();
				let val = config_from_binary(data,loc)?;
				list.push( val );
			}
			Ok(ConfigurationValue::NamedExperiments(name,list))
		},
		6 => Ok(ConfigurationValue::True),
		7 => Ok(ConfigurationValue::False),
		8 => panic!("binary format of where clauses is not yet supported"),
		9 => panic!("binary format of expressions is not yet supported"),
		10 => Ok(ConfigurationValue::None),
		_ => panic!("Do not know what to do with magic={}",magic),
	}
}


///Rewrites the value in-place.
///If `edition` is `term=new_value` where `term` can be interpreted as a left-value then replace its content with `new_value`.
///returns `true` is something in `value` has been changed.
pub fn rewrite_eq(value:&mut ConfigurationValue, edition:&Expr, path:&Path) -> bool
{
	match edition
	{
		Expr::Equality(left,right) =>
		{
			let new_value = evaluate(right,&value,path);
			//rewrite_pair(value,left,new_value)
			if let Some(ptr) = config_mut_into(value,left)
			{
				*ptr = new_value;
				true
			} else {
				false
			}
		}
		_ => false,
	}
}

///Rewrites the value in-place.
///If `path_expr` can be interpreted as a left-value then replace its content with `new_value`.
///returns `true` is something in `value` has been changed.
pub fn rewrite_pair(value:&mut ConfigurationValue, path_expr:&Expr, new_value:&Expr, path:&Path) -> bool
{
	let new_value = evaluate(new_value,&value,path);
	if let Some(ptr) = config_mut_into(value,path_expr)
	{
		*ptr = new_value;
		true
	} else {
		false
	}
}

///Rewrites the value in-place.
///If `path_expr` can be interpreted as a left-value then replace its content with `new_value`.
///returns `true` is something in `value` has been changed.
pub fn rewrite_pair_value(value:&mut ConfigurationValue, path_expr:&Expr, new_value:ConfigurationValue) -> bool
{
	if let Some(ptr) = config_mut_into(value,path_expr)
	{
		*ptr = new_value;
		true
	} else {
		false
	}
}

///Tries to access to a given path inside a ConfigurationValue
///Returns `None` if the path is not found.
pub fn config_mut_into<'a>(value:&'a mut ConfigurationValue, expr_path:&Expr) -> Option<&'a mut ConfigurationValue>
{
	match expr_path
	{
		Expr::Ident(ref name) =>
		{
			match value
			{
				ConfigurationValue::Object(ref _object_name,ref mut arr) =>
				{
					for (key,val) in arr.iter_mut()
					{
						if key==name
						{
							return Some(val);
						}
					}
					None
				}
				_ => None,
			}
		}
		Expr::Member(ref parent, ref field_name) =>
		{
			match config_mut_into(value,parent)
			{
				Some(into_parent) => config_mut_into(into_parent,&Expr::Ident(field_name.clone())),
				None => None,
			}
		}
		_ =>
		{
			None
		}
	}
}

/// Less strict than PartialEq
/// Ignores the fields `legend_name`, and `launch_configurations`.
pub fn config_relaxed_cmp(a:&ConfigurationValue, b:&ConfigurationValue) -> bool
{
	use ConfigurationValue::*;
	let ignore = |key| key == "legend_name" || key == "launch_configurations";
	match (a,b)
	{
		(Literal(sa),Literal(sb)) => sa==sb,
		(Number(xa),Number(xb)) => xa==xb,
		(Object(na,xa),Object(nb,xb)) =>
		{
			//na==nb && xa==xb,
			if na != nb { return false; }
			//do we want to enforce order of the fields?
			for ( (ka,va),(kb,vb) ) in
				xa.iter().filter(|(key,_)| !ignore(key) ).zip(
				xb.iter().filter(|(key,_)| !ignore(key)  ) )
			{
				if ka != kb { return false; }
				if !config_relaxed_cmp(va,vb) { return false; }
			}
			return true;
		}
		(Array(xa),Array(xb)) =>
		{
			//xa==xb
			for (va,vb) in
				xa.iter().zip(
				xb.iter() )
			{
				if !config_relaxed_cmp(va,vb) { return false; }
			}
			return true;
		}
		(Experiments(xa),Experiments(xb)) =>
		{
			//xa==xb
			for (va,vb) in
				xa.iter().zip(
				xb.iter() )
			{
				if !config_relaxed_cmp(va,vb) { return false; }
			}
			return true;
		}
		(NamedExperiments(na,xa),NamedExperiments(nb,xb)) =>
		{
			//na==nb && xa==xb,
			if na != nb { return false; }
			for (va,vb) in
				xa.iter().zip(
				xb.iter() )
			{
				if !config_relaxed_cmp(va,vb) { return false; }
			}
			return true;
		}
		(True,True) => true,
		(False,False) => true,
		(Where(xa,ea),Where(xb,eb)) => xa==xb && ea==eb,
		(Expression(xa),Expression(xb)) => xa==xb,
		(None,None) => true,
		_ => false,
	}
}


/// match arms agains the keys of an object
/// first argument, `$cv:expr`, is the ConfigurationValue expected to be the object
/// second argument, `$name:literal`, is the name the Object should have.
/// third argument, `$valueid:ident`, is the variable name capturing the value in the object's elements
///    and can be used in the arms
/// the remaining arguments are the arms of the match.
#[macro_export]
macro_rules! match_object{
	//($cv:expr, $name:literal, $valueid:ident, $($key:literal => $arm:tt)* ) => {{
	($cv:expr, $name:literal, $valueid:ident, $($arm:tt)* ) => {{
		//Error::$kind( source_location!(), $($args),* )
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs) = $cv
		{
			if cv_name!= $name
			{
				panic!("A Pow must be created from a `{}` object not `{}`",$name,cv_name);
			}
			for &(ref name,ref $valueid) in cv_pairs
			{
				//match name.as_ref()
				match AsRef::<str>::as_ref(&name)
				{
					//"pattern" => pattern=Some(new_pattern(PatternBuilderArgument{cv:value,..arg})),
					$( $arm )*
					"legend_name" => Ok(()),
					//_ => panic!("Nothing to do with field {} in {}",name,$name),
					_ => Err(error!(ill_formed_configuration,$cv).with_message(format!("Nothing to do with field {} in {}",name,$name)))?,
				}
			}
		}
		else
		{
			//panic!("Trying to create a {} from a non-Object",$name);
			Err(error!(ill_formed_configuration,$cv).with_message(format!("Trying to create a {} from a non-Object",$name)))?
		}
	}};
}
///Like `match_object!` but panicking on errors.
#[macro_export]
macro_rules! match_object_panic{
	($cv:expr, $name:literal, $valueid:ident, $($arm:tt)* ) => {{
		if let &ConfigurationValue::Object(ref cv_name, ref cv_pairs) = $cv
		{
			if cv_name!= $name
			{
				panic!("A Pow must be created from a `{}` object not `{}`",$name,cv_name);
			}
			for &(ref name,ref $valueid) in cv_pairs
			{
				match AsRef::<str>::as_ref(&name)
				{
					$( $arm )*
					"legend_name" => (),
					_ => panic!("Nothing to do with field {} in {}",name,$name),
				}
			}
		}
		else
		{
			panic!("Trying to create a {} from a non-Object",$name);
		}
	}};
}


