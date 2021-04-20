
use std::collections::{BTreeMap};

use crate::config_parser::{ConfigurationValue,Expr};

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
pub fn combine(configuration:&ConfigurationValue, result:&ConfigurationValue) -> ConfigurationValue
{
	ConfigurationValue::Object(String::from("Context"),vec![
		(String::from("configuration"),configuration.clone()),
		(String::from("result"),result.clone()),
	])
}

///Evaluates an expression given in a context.
///For example the expression `=Alpha.beta` will return 42 for the context `Alpha{beta:42}`.
pub fn evaluate(expr:&Expr, context:&ConfigurationValue) -> ConfigurationValue
{
	match expr
	{
		&Expr::Equality(ref a,ref b) =>
		{
			let va=evaluate(a,context);
			let vb=evaluate(b,context);
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
			let value=evaluate(expr,context);
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
		&Expr::Parentheses(ref expr) => evaluate(expr,context),
		&Expr::Name(ref expr) =>
		{
			let value=evaluate(expr,context);
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
								first=Some(evaluate(val,context));
							},
							"second" =>
							{
								second=Some(evaluate(val,context));
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
								condition=Some(evaluate(val,context));
							},
							"true_expression" =>
							{
								true_expression=Some(evaluate(val,context));
							},
							"false_expression" =>
							{
								false_expression=Some(evaluate(val,context));
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
				"add" =>
				{
					let mut first=None;
					let mut second=None;
					for (key,val) in arguments
					{
						match key.as_ref()
						{
							"first" =>
							{
								first=Some(evaluate(val,context));
							},
							"second" =>
							{
								second=Some(evaluate(val,context));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let first=first.expect("first argument of and not given.");
					let second=second.expect("second argument of and not given.");
					let first=match first
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("first argument of add evaluated to a non-number ({}:?)",first),
					};
					let second=match second
					{
						ConfigurationValue::Number(x) => x,
						_ => panic!("second argument of add evaluated to a non-number ({}:?)",second),
					};
					ConfigurationValue::Number(first+second)
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
								container=Some(evaluate(val,context));
							},
							"position" =>
							{
								position=Some(evaluate(val,context));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let container=container.expect("container argument of at not given.");
					let position=position.expect("position argument of at not given.");
					let container=match container
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({}:?)",container),
					};
					let position=match position
					{
						ConfigurationValue::Number(x) => x as usize,
						_ => panic!("position argument of lt evaluated to a non-number ({}:?)",position),
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
								data=Some(evaluate(val,context));
							},
							"width" =>
							{
								width=Some(evaluate(val,context));
							},
							_ => panic!("unknown argument `{}' for function `{}'",key,function_name),
						}
					}
					let data=data.expect("data argument of at not given.");
					let width=width.expect("width argument of at not given.");
					let data=match data
					{
						ConfigurationValue::Array(a) => a,
						_ => panic!("first argument of at evaluated to a non-array ({}:?)",data),
					};
					let width=match width
					{
						ConfigurationValue::Number(x) => x as usize,
						_ => panic!("width argument of lt evaluated to a non-number ({}:?)",width),
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
								x => panic!("AverageBins received {:?}",x),
							}
						}
						ConfigurationValue::Number(total/width as f64)
					}).collect();
					ConfigurationValue::Array(result)
				}
				_ => panic!("Unknown function `{}'",function_name),
			}
		}
	}
}

/// Evaluate some expressions inside a ConfigurationValue
pub fn reevaluate(value:&ConfigurationValue, context:&ConfigurationValue) -> ConfigurationValue
{
	//if let &ConfigurationValue::Expression(ref expr)=value
	//{
	//	evaluate(expr,context)
	//}
	//else
	//{
	//	value.clone()
	//}
	match value
	{
		&ConfigurationValue::Expression(ref expr) => evaluate(expr,context),
		&ConfigurationValue::Array(ref l) => ConfigurationValue::Array(l.iter().map(|e|reevaluate(e,context)).collect()),
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


