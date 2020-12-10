extern crate gramatica;
use std::cmp::Ordering;
use self::gramatica::{Associativity,EarleyKind,State,Parser,ParsingTablesTrait,AmbiguityInfo,ParsingError};
use std::rc::Rc;
use std::fmt::{Display,Formatter,Error};
#[derive(Clone,Debug,PartialEq,PartialOrd)]
pub enum ConfigurationValue{
Literal(String),
Number(f64),
Object(String,Vec<(String,ConfigurationValue)>),
Array(Vec<ConfigurationValue>),
Experiments(Vec<ConfigurationValue>),
NamedExperiments(String,Vec<ConfigurationValue>),
True,
False,
Where(Rc<ConfigurationValue>,Expr),
Expression(Expr),
}

impl ConfigurationValue
{
	fn write(&self, f: &mut Formatter, indent:usize) -> Result<(),Error>
	{
		let is=String::from("\t").repeat(indent);
		write!(f,"{}",is)?;
		match self
		{
			&ConfigurationValue::Literal(ref s) => write!(f,"{}",s)?,
			&ConfigurationValue::Number(v) => write!(f,"{}",v)?,
			&ConfigurationValue::Object(ref name, ref list) =>
			{
				writeln!(f,"{}\n{}{{",name,is)?;
				for &(ref attr_name,ref attr_value) in list.iter()
				{
					writeln!(f,"{}\t{}:",is,attr_name)?;
					attr_value.write(f,indent+1)?;
					writeln!(f,",")?;
				}
				writeln!(f,"{}}}",is)?;
			},
			&ConfigurationValue::Array(ref list) =>
			{
				writeln!(f,"[")?;
				for elem in list.iter()
				{
					elem.write(f,indent+1)?;
					writeln!(f,",")?;
				}
				writeln!(f,"{}]",is)?;
			},
			&ConfigurationValue::Experiments(ref _list) => write!(f,"FIXME")?,
			&ConfigurationValue::NamedExperiments(ref _name, ref _list) => write!(f,"FIXME")?,
			&ConfigurationValue::True => write!(f,"true")?,
			&ConfigurationValue::False => write!(f,"false")?,
			&ConfigurationValue::Where(ref cv, ref _expr) => write!(f,"{} where FIXME",cv)?,
			&ConfigurationValue::Expression(ref e) => write!(f,"= {}",e)?,
		};
		Ok(())
	}
}

impl Display for ConfigurationValue {
fn fmt(&self,f:&mut Formatter)->Result<(),Error>{
self.write(f,0)}

}

#[derive(Clone,Debug,PartialEq,PartialOrd)]
pub enum Expr{
Equality(Rc<Expr>,Rc<Expr>),
Literal(String),
Number(f64),
Ident(String),
Member(Rc<Expr>,String),
Parentheses(Rc<Expr>),
Name(Rc<Expr>),
FunctionCall(String,Vec<(String,Expr)>),
}

impl Display for Expr
{
	fn fmt(&self, f: &mut Formatter) -> Result<(),Error>
	{
		match self
		{
			&Expr::Literal(ref s) => write!(f,"{}",s),
			&Expr::Number(ref v) => write!(f,"{}",v),
			&Expr::Ident(ref s) => write!(f,"{}",s),
			&Expr::Member(ref expr,ref s) => write!(f,"{}.{}",expr,s),
			&Expr::Name(ref expr) => write!(f,"@{}",expr),
			_ => write!(f,"fix this expr <{:?}>",self),
		}
	}
}

pub fn parse(source:& str)->Result<Token,ParsingError>{
Parser::<Token,ParsingTables>::parse(source,None)}

pub fn parse_expression(source:&str) -> Result<Token,ParsingError>
{
	Parser::<Token,ParsingTables>::parse(source,Some(26))
}

#[derive(Clone,Debug,PartialEq)]
pub enum Token{DummyStart,
True,False,Where,Number(f64),LitStr(String),Ident(String),EqualEqual,LBrace,RBrace,LBracket,RBracket,LPar,RPar,Comma,Colon,Bang,At,Equal,Dot,Value(ConfigurationValue),Object(ConfigurationValue),Members(Vec<(String,ConfigurationValue)>),Pair(String,ConfigurationValue),Array(Vec<ConfigurationValue>),Elements(Vec<ConfigurationValue>),Expression(Expr),FunctionCall(Expr),Arguments(Vec<(String,Expr)>),ExprPair(String,Expr),}
impl Default for Token { fn default()->Self{Token::DummyStart} }
struct ParsingTables { }
impl ParsingTablesTrait<Token> for ParsingTables {
fn initial()->usize { 20 }
#[allow(unused)]
fn match_some(parser: &mut Parser<Token,Self>) -> Option<(usize,Token)> { let source=&parser.source[parser.source_index..];
match { match parser.keyword("true",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::True)), };
match { match parser.keyword("false",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::False)), };
match { match parser.keyword("where",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::Where)), };
match { match parser.re("-?[0-9]*\\.?[0-9]+([eE][-+]?[0-9]+)?",source) { None => None, Some((size,string)) => Some((size,string.parse::<f64>().unwrap() )) } }
{ None => (), Some((size,result)) => return Some((size,Token::Number(result))), };
{ fn _match(parser:&mut Parser<Token,ParsingTables>,source:& str)->Option<(usize,String)>{
let mut ret=None;
let mut characters=source.chars();
if (characters.next()) != (Some('"')) {} else {let mut size=1;
let mut r=String::new();
loop {match characters.next() { None => break, Some('"') => {ret = { Some((size + 1,r))};
break;}, Some('\\') => {match characters.next() { None => break, Some(c) => {r.push('\\');
r.push(c);}, }
;
size += 2;}, Some(c) => {r.push(c);
size += 1;}, }
;}}
ret}

match _match(parser,source) { None=>(), Some((size,result)) => return Some((size,Token::LitStr(result))), } };
match { match parser.re("[a-zA-Z\\x80-\\xff_][a-zA-Z0-9\\x80-\\xff_]*",source) { None => None, Some((size,string)) => Some((size,string.parse::<String>().unwrap() )) } }
{ None => (), Some((size,result)) => return Some((size,Token::Ident(result))), };
match { match parser.re("==",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::EqualEqual)), };
match { match parser.re("\\{",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::LBrace)), };
match { match parser.re("\\}",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::RBrace)), };
match { match parser.re("\\[",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::LBracket)), };
match { match parser.re("\\]",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::RBracket)), };
match { match parser.re("\\(",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::LPar)), };
match { match parser.re("\\)",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::RPar)), };
match { match parser.re(",",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::Comma)), };
match { match parser.re(":",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::Colon)), };
match { match parser.re("!",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::Bang)), };
match { match parser.re("@",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::At)), };
match { match parser.re("=",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::Equal)), };
match { match parser.re("\\.",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::Dot)), };
match { match parser.re("\\s+|\n|//[^\n]*\n|/\\*([^*]|\\*+[^/])*\\*+/",source) { None => None, Some((size,_string)) => Some((size,())) } }
{ None => (), Some((size,_result)) => return Some((size,Token::DummyStart)), };
None }//match_some
fn predict(parser:&mut Parser<Token,Self>,index:usize,state_index:usize,token:usize) { match token {
20 => {
parser.sets[index].predict(State{rule: 1 ,left: 20 ,right:vec![ 5 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 2 ,left: 20 ,right:vec![ 4 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 3 ,left: 20 ,right:vec![ 21 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 4 ,left: 20 ,right:vec![ 24 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 5 ,left: 20 ,right:vec![ 16,24 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 2 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 6 ,left: 20 ,right:vec![ 6,16,24 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 7 ,left: 20 ,right:vec![ 1 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 8 ,left: 20 ,right:vec![ 2 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 9 ,left: 20 ,right:vec![ 20,3,26 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 10 ,left: 20 ,right:vec![ 18,26 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 2 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
21 => {
parser.sets[index].predict(State{rule: 11 ,left: 21 ,right:vec![ 6 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 12 ,left: 21 ,right:vec![ 6,8,9 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 13 ,left: 21 ,right:vec![ 6,8,22,9 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 4 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 14 ,left: 21 ,right:vec![ 6,8,22,14,9 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 5 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
22 => {
parser.sets[index].predict(State{rule: 15 ,left: 22 ,right:vec![ 23 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 16 ,left: 22 ,right:vec![ 22,14,23 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
23 => {
parser.sets[index].predict(State{rule: 17 ,left: 23 ,right:vec![ 6,15,20 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
24 => {
parser.sets[index].predict(State{rule: 18 ,left: 24 ,right:vec![ 10,11 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 2 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 19 ,left: 24 ,right:vec![ 10,25,11 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 20 ,left: 24 ,right:vec![ 10,25,14,11 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 4 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
25 => {
parser.sets[index].predict(State{rule: 21 ,left: 25 ,right:vec![ 20 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 22 ,left: 25 ,right:vec![ 25,14,20 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
26 => {
parser.sets[index].predict(State{rule: 23 ,left: 26 ,right:vec![ 26,7,26 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 24 ,left: 26 ,right:vec![ 5 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 25 ,left: 26 ,right:vec![ 4 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 26 ,left: 26 ,right:vec![ 6 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 27 ,left: 26 ,right:vec![ 26,19,6 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 28 ,left: 26 ,right:vec![ 12,26,13 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 29 ,left: 26 ,right:vec![ 17,26 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 2 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 30 ,left: 26 ,right:vec![ 27 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
27 => {
parser.sets[index].predict(State{rule: 31 ,left: 27 ,right:vec![ 6,8,9 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 32 ,left: 27 ,right:vec![ 6,8,28,9 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 4 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 33 ,left: 27 ,right:vec![ 6,8,28,14,9 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 5 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
28 => {
parser.sets[index].predict(State{rule: 34 ,left: 28 ,right:vec![ 29 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 1 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
parser.sets[index].predict(State{rule: 35 ,left: 28 ,right:vec![ 28,14,29 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
29 => {
parser.sets[index].predict(State{rule: 36 ,left: 29 ,right:vec![ 6,15,26 ],position:0,original_set:index,kind:EarleyKind::Predict(state_index),values:vec![Token::DummyStart; 3 ],computed_value:Token::DummyStart,ambiguity_info:AmbiguityInfo::default(),});
}
_ => panic!(""), } }//predict
#[allow(unused)]
fn compute_value(state:&mut State<Token>) { state.computed_value = match state.rule { 0 => state.values[0].clone(),
1 => match &state.values[0] {
&Token::LitStr(ref s) => Token::Value(ConfigurationValue::Literal(s.clone())),
_ => panic!(""), },
2 => match &state.values[0] {
&Token::Number(v) => Token::Value(ConfigurationValue::Number(v)),
_ => panic!(""), },
3 => match &state.values[0] {
&Token::Object(ref value) => Token::Value(value.clone()),
_ => panic!(""), },
4 => match &state.values[0] {
&Token::Array(ref list) => Token::Value(ConfigurationValue::Array(list.clone())),
_ => panic!(""), },
5 => match (&state.values[0],&state.values[1]) {
(&Token::Bang,&Token::Array(ref list)) => Token::Value(ConfigurationValue::Experiments(list.clone())),
_ => panic!(""), },
6 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Ident(ref name),&Token::Bang,&Token::Array(ref list)) => Token::Value(ConfigurationValue::NamedExperiments(name.clone(),list.clone())),
_ => panic!(""), },
7 => match &state.values[0] {
&Token::True => Token::Value(ConfigurationValue::True),
_ => panic!(""), },
8 => match &state.values[0] {
&Token::False => Token::Value(ConfigurationValue::False),
_ => panic!(""), },
9 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Value(ref value),&Token::Where,&Token::Expression(ref expr)) => Token::Value(ConfigurationValue::Where(Rc::new(value.clone()),expr.clone())),
_ => panic!(""), },
10 => match (&state.values[0],&state.values[1]) {
(&Token::Equal,&Token::Expression(ref e)) => Token::Value(ConfigurationValue::Expression(e.clone())),
_ => panic!(""), },
11 => match &state.values[0] {
&Token::Ident(ref name) => Token::Object(ConfigurationValue::Object(name.clone(),vec![])),
_ => panic!(""), },
12 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Ident(ref name),&Token::LBrace,&Token::RBrace) => Token::Object(ConfigurationValue::Object(name.clone(),vec![])),
_ => panic!(""), },
13 => match (&state.values[0],&state.values[1],&state.values[2],&state.values[3]) {
(&Token::Ident(ref name),&Token::LBrace,&Token::Members(ref list),&Token::RBrace) => Token::Object(ConfigurationValue::Object(name.clone(),list.clone())),
_ => panic!(""), },
14 => match (&state.values[0],&state.values[1],&state.values[2],&state.values[3],&state.values[4]) {
(&Token::Ident(ref name),&Token::LBrace,&Token::Members(ref list),&Token::Comma,&Token::RBrace) => Token::Object(ConfigurationValue::Object(name.clone(),list.clone())),
_ => panic!(""), },
15 => match &state.values[0] {
&Token::Pair(ref s,ref value) => Token::Members(vec![(s . clone () , value . clone ())]),
_ => panic!(""), },
16 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Members(ref list),&Token::Comma,&Token::Pair(ref s,ref value)) => Token::Members({let mut new=(list.clone());
new.push((s.clone(),value.clone())); new}),
_ => panic!(""), },
17 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Ident(ref s),&Token::Colon,&Token::Value(ref value)) => { let (x0,x1)=(s.clone(),value.clone()); Token::Pair(x0,x1) },
_ => panic!(""), },
18 => match (&state.values[0],&state.values[1]) {
(&Token::LBracket,&Token::RBracket) => Token::Array(vec![]),
_ => panic!(""), },
19 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::LBracket,&Token::Elements(ref list),&Token::RBracket) => Token::Array(list.clone()),
_ => panic!(""), },
20 => match (&state.values[0],&state.values[1],&state.values[2],&state.values[3]) {
(&Token::LBracket,&Token::Elements(ref list),&Token::Comma,&Token::RBracket) => Token::Array(list.clone()),
_ => panic!(""), },
21 => match &state.values[0] {
&Token::Value(ref value) => Token::Elements(vec![value . clone ()]),
_ => panic!(""), },
22 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Elements(ref list),&Token::Comma,&Token::Value(ref value)) => Token::Elements({let mut new=(list.clone());
new.push(value.clone()); new}),
_ => panic!(""), },
23 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Expression(ref left),&Token::EqualEqual,&Token::Expression(ref right)) => Token::Expression(Expr::Equality(Rc::new(left.clone()),Rc::new(right.clone()))),
_ => panic!(""), },
24 => match &state.values[0] {
&Token::LitStr(ref s) => Token::Expression(Expr::Literal(s.clone())),
_ => panic!(""), },
25 => match &state.values[0] {
&Token::Number(v) => Token::Expression(Expr::Number(v)),
_ => panic!(""), },
26 => match &state.values[0] {
&Token::Ident(ref s) => Token::Expression(Expr::Ident(s.clone())),
_ => panic!(""), },
27 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Expression(ref path),&Token::Dot,&Token::Ident(ref element)) => Token::Expression(Expr::Member(Rc::new(path.clone()),element.clone())),
_ => panic!(""), },
28 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::LPar,&Token::Expression(ref expr),&Token::RPar) => Token::Expression(Expr::Parentheses(Rc::new(expr.clone()))),
_ => panic!(""), },
29 => match (&state.values[0],&state.values[1]) {
(&Token::At,&Token::Expression(ref expr)) => Token::Expression(Expr::Name(Rc::new(expr.clone()))),
_ => panic!(""), },
30 => match &state.values[0] {
&Token::FunctionCall(ref value) => Token::Expression(value.clone()),
_ => panic!(""), },
31 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Ident(ref name),&Token::LBrace,&Token::RBrace) => Token::FunctionCall(Expr::FunctionCall(name.clone(),vec![])),
_ => panic!(""), },
32 => match (&state.values[0],&state.values[1],&state.values[2],&state.values[3]) {
(&Token::Ident(ref name),&Token::LBrace,&Token::Arguments(ref list),&Token::RBrace) => Token::FunctionCall(Expr::FunctionCall(name.clone(),list.clone())),
_ => panic!(""), },
33 => match (&state.values[0],&state.values[1],&state.values[2],&state.values[3],&state.values[4]) {
(&Token::Ident(ref name),&Token::LBrace,&Token::Arguments(ref list),&Token::Comma,&Token::RBrace) => Token::FunctionCall(Expr::FunctionCall(name.clone(),list.clone())),
_ => panic!(""), },
34 => match &state.values[0] {
&Token::ExprPair(ref s,ref value) => Token::Arguments(vec![(s . clone () , value . clone ())]),
_ => panic!(""), },
35 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Arguments(ref list),&Token::Comma,&Token::ExprPair(ref s,ref value)) => Token::Arguments({let mut new=(list.clone());
new.push((s.clone(),value.clone())); new}),
_ => panic!(""), },
36 => match (&state.values[0],&state.values[1],&state.values[2]) {
(&Token::Ident(ref s),&Token::Colon,&Token::Expression(ref expr)) => { let (x0,x1)=(s.clone(),expr.clone()); Token::ExprPair(x0,x1) },
_ => panic!(""), },
_ => panic!(""), } }//compute_value
fn table_terminal(token_index:usize)->bool { match token_index {
1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16|17|18|19 => true,
0|20|21|22|23|24|25|26|27|28|29 => false,
_ => panic!("table_terminal"), } }//table_terminal
fn table_priority(a:usize, b:usize) -> Option<Ordering> { match (a,b) {
(23,23) => Some(Ordering::Equal),
(23,27) => Some(Ordering::Greater),
(23,29) => Some(Ordering::Greater),
(27,23) => Some(Ordering::Less),
(27,27) => Some(Ordering::Equal),
(27,29) => Some(Ordering::Less),
(29,23) => Some(Ordering::Less),
(29,27) => Some(Ordering::Greater),
(29,29) => Some(Ordering::Equal),
_ => None, } }//table_priority
fn table_associativity(rule:usize) -> Option<Associativity> { match rule {
_ => None, } }//table_associativity
fn to_usize(token:&Token) -> usize { match token { &Token::DummyStart => 0,
&Token::True => 1,
&Token::False => 2,
&Token::Where => 3,
&Token::Number(_) => 4,
&Token::LitStr(_) => 5,
&Token::Ident(_) => 6,
&Token::EqualEqual => 7,
&Token::LBrace => 8,
&Token::RBrace => 9,
&Token::LBracket => 10,
&Token::RBracket => 11,
&Token::LPar => 12,
&Token::RPar => 13,
&Token::Comma => 14,
&Token::Colon => 15,
&Token::Bang => 16,
&Token::At => 17,
&Token::Equal => 18,
&Token::Dot => 19,
&Token::Value(_) => 20,
&Token::Object(_) => 21,
&Token::Members(_) => 22,
&Token::Pair(_,_) => 23,
&Token::Array(_) => 24,
&Token::Elements(_) => 25,
&Token::Expression(_) => 26,
&Token::FunctionCall(_) => 27,
&Token::Arguments(_) => 28,
&Token::ExprPair(_,_) => 29,
} }//to_usize
}//impl
