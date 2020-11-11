
extern crate proc_macro;
//#[macro_use]
extern crate quote;
extern crate syn;
extern crate synstructure;

use syn::{parse_macro_input, DeriveInput};
use syn::spanned::Spanned;
use quote::{quote, quote_spanned};


// Look https://github.com/dtolnay/syn/blob/master/examples/heapsize/heapsize_derive/src/lib.rs for a more recent similar derive.

//WITH proc_macro
// #[proc_macro_derive(Quantifiable)]
// pub fn quantifiable_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
// 	//let ast = syn::parse(input).unwrap();
// 	let mut ast = syn::parse_macro_input(&input.to_string()).unwrap();
// 	//let mut ast = syn::parse_macro_input!(input);
// 	let style = synstructure::BindStyle::Ref.into();
// 	//Collect all the fields into a total_memory method.
// 	let total_memory_body = synstructure::each_field(&mut ast, &style, |binding| {
// 			Some(quote! {
// 					sum += ::quantify::Quantifiable::total_memory(#binding);
// 					})
// 			});
// 	//Collect all the fields into a forecast_total_memory method.
// 	let forecast_total_memory_body = synstructure::each_field(&mut ast, &style, |binding| {
// 			Some(quote! {
// 					sum += ::quantify::Quantifiable::forecast_total_memory(#binding);
// 					})
// 			});
// 	//The name of the type for which we are implementing Quantifiable.
// 	let name = &ast.ident;
// 	let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
// 	let mut where_clause = where_clause.clone();
// 	//We added the where condition member_name:Quantifiable for each member.
// 	for param in &ast.generics.ty_params {
// 		where_clause.predicates.push(syn::WherePredicate::BoundPredicate(syn::WhereBoundPredicate {
// 			bound_lifetimes: Vec::new(),
// 			bounded_ty: syn::Ty::Path(None, param.ident.clone().into()),
// 			//bounds: vec![syn::TypeParamBound::Trait(
// 			bounds: vec![syn::TyParamBound::Trait(
// 				syn::PolyTraitRef {
// 					bound_lifetimes: Vec::new(),
// 					trait_ref: syn::parse_path("::quantifiable::Quantifiable").unwrap(),
// 				},
// 				syn::TraitBoundModifier::None
// 			)],
// 		}))
// 		//where_clause.predicates.push(syn::WherePredicate::Type(syn::PredicateType {
// 		//	lifetimes: None,
// 		//	bounded_ty: syn::Ty::Path(None, param.ident.clone().into()),
// 		//	bounds: syn::parse_path("::quantifiable::Quantifiable").unwrap(),
// 		//}))
// 	}
// 	//Build the token sequence.
// 	let tokens = quote! {
// 		impl #impl_generics ::quantify::Quantifiable for #name #ty_generics #where_clause {
// #[inline]
// #[allow(unused_variables, unused_mut, unreachable_code)]
// 			fn total_memory(&self) -> usize {
// 				let mut sum = 0;
// 				match *self {
// 					#total_memory_body
// 				}
// 				sum
// 			}
// 			fn print_memory_breakdown(&self)
// 			{
// 				unimplemented!();
// 			}
// 			fn forecast_total_memory(&self) -> usize
// 			{
// 				let mut sum = 0;
// 				match *self {
// 					#forecast_total_memory_body
// 				}
// 				sum
// 			}
// 		}
// 	};
// 	//tokens
// 	tokens.to_string().parse().unwrap()
// }

//WITH proc_macro2 and syn-1.0
// https://doc.rust-lang.org/proc_macro/index.html
// https://docs.rs/proc-macro2/1.0.24/proc_macro2/index.html
// https://docs.rs/syn/1.0.44/syn/index.html
#[proc_macro_derive(Quantifiable)]
pub fn quantifiable_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	//let ast = syn::parse(input).unwrap();
	//let mut ast = syn::parse_macro_input(&input.to_string()).unwrap();
	//let input = proc_macro2::TokenStream::from(input);
	let ast = parse_macro_input!(input as DeriveInput);
	//let mut ast = syn::parse_macro_input!(input);
	//let style = synstructure::BindStyle::Ref.into();
	//Collect all the fields into a total_memory method.
	//let total_memory_body = synstructure::each_field(&mut ast, &style, |binding| {
	//		Some(quote! {
	//				sum += ::quantify::Quantifiable::total_memory(#binding);
	//				})
	//		});
	////Collect all the fields into a forecast_total_memory method.
	//let forecast_total_memory_body = synstructure::each_field(&mut ast, &style, |binding| {
	//		Some(quote! {
	//				sum += ::quantify::Quantifiable::forecast_total_memory(#binding);
	//				})
	//		});
	let total_memory_body = quantifiable_total_memory_expression(&ast.data);
	let forecast_total_memory_body = quantifiable_forecast_total_memory_expression(&ast.data);
	//The name of the type for which we are implementing Quantifiable.
	let name = &ast.ident;
	let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
	let where_clause = where_clause.clone();
	//We added the where condition member_name:Quantifiable for each member.
	// for param in &ast.generics.ty_params {
	// 	where_clause.predicates.push(syn::WherePredicate::BoundPredicate(syn::WhereBoundPredicate {
	// 		bound_lifetimes: Vec::new(),
	// 		bounded_ty: syn::Ty::Path(None, param.ident.clone().into()),
	// 		//bounds: vec![syn::TypeParamBound::Trait(
	// 		bounds: vec![syn::TyParamBound::Trait(
	// 			syn::PolyTraitRef {
	// 				bound_lifetimes: Vec::new(),
	// 				trait_ref: syn::parse_path("::quantifiable::Quantifiable").unwrap(),
	// 			},
	// 			syn::TraitBoundModifier::None
	// 		)],
	// 	}))
	// 	//where_clause.predicates.push(syn::WherePredicate::Type(syn::PredicateType {
	// 	//	lifetimes: None,
	// 	//	bounded_ty: syn::Ty::Path(None, param.ident.clone().into()),
	// 	//	bounds: syn::parse_path("::quantifiable::Quantifiable").unwrap(),
	// 	//}))
	// }
	//Build the token sequence.
	let tokens = quote! {
		impl #impl_generics crate::quantify::Quantifiable for #name #ty_generics #where_clause {
#[inline]
#[allow(unused_variables, unused_mut, unreachable_code)]
			fn total_memory(&self) -> usize {
				#total_memory_body
			}
			fn print_memory_breakdown(&self)
			{
				unimplemented!();
			}
			fn forecast_total_memory(&self) -> usize
			{
				#forecast_total_memory_body
			}
		}
	};
	//tokens
	//tokens.to_string().parse().unwrap()
	proc_macro::TokenStream::from(tokens)
}

///Create an expression for the total_memory method.
fn quantifiable_total_memory_expression(data: &syn::Data) -> proc_macro2::TokenStream
{
	match *data
	{
		syn::Data::Struct(ref data) =>
		{
			match data.fields
			{
				syn::Fields::Named(ref fields) => 
				{
					//An struct with named fields. Blah{name1:type1, name2:type2, ...}
					let fit=fields.named.iter().map(|field|{
						let name=&field.ident;
						quote_spanned!{ field.span() =>
							crate::quantify::Quantifiable::total_memory(&self.#name)
						}
					});
					quote!{ 0 #(+ #fit)* }
				}
				syn::Fields::Unnamed(ref fields) =>
				{
					let fit=fields.unnamed.iter().enumerate().map(|(i,field)|{
						let index=syn::Index::from(i);
						quote_spanned!{ field.span() =>
							crate::quantify::Quantifiable::total_memory(&self.#index)
						}
					});
					quote!{ 0 #(+ #fit)* }
				}
				syn::Fields::Unit => quote!(0),
			}
		},
		syn::Data::Enum(ref data) =>
		{
			//https://docs.rs/syn/1.0.44/syn/struct.DataEnum.html
			let vit = data.variants.iter().map(|variant|{
				let vname = &variant.ident;
				match variant.fields
				{
					syn::Fields::Named(ref fields) => 
					{
						//An struct with named fields. Blah{name1:type1, name2:type2, ...}
						let fit=fields.named.iter().map(|field|{
							let name=&field.ident;
							quote_spanned!{ field.span() =>
								crate::quantify::Quantifiable::total_memory(#name)
							}
						});
						let sum =quote!{ 0 #(+ #fit)* };
						let fit=fields.named.iter().map(|field|{
							let name=&field.ident;
							quote_spanned!{ field.span() =>
								#name
							}
						});
						let args = quote!{ #(#fit,)* };
						quote_spanned!{ variant.span() =>
							Self::#vname{#args} => #sum,
						}
					},
					syn::Fields::Unnamed(ref fields) =>
					{
						let fit=fields.unnamed.iter().enumerate().map(|(i,field)|{
							//let index=syn::Index::from(i);
							let arg=quote::format_ident!("u{}",i);
							quote_spanned!{ field.span() =>
								crate::quantify::Quantifiable::total_memory(#arg)
							}
						});
						let sum = quote!{ 0 #(+ #fit)* };
						let fit=fields.unnamed.iter().enumerate().map(|(i,field)|{
							//let index=syn::Index::from(i);
							let arg=quote::format_ident!("u{}",i);
							quote_spanned!{ field.span() =>
								#arg
							}
						});
						let args = quote!{ #(#fit,)* };
						quote_spanned!{ variant.span() =>
							Self::#vname(#args) => #sum,
						}
					},
					syn::Fields::Unit =>
					{
						//quote!(0),
						quote!(Self::#vname => 0,)
					},
				}
			});
			//FIXME: should be max instead of plus.
			let tokens=quote!{ match self { #( #vit)* } };
			//quote!{compile_error!(stringify!{#tokens});#tokens}
			tokens
		},
		syn::Data::Union(_) => unimplemented!(),
	}
}

fn quantifiable_forecast_total_memory_expression(data: &syn::Data) -> proc_macro2::TokenStream
{
	match *data
	{
		syn::Data::Struct(ref data) =>
		{
			match data.fields
			{
				syn::Fields::Named(ref fields) => 
				{
					//An struct with named fields. Blah{name1:type1, name2:type2, ...}
					let fit=fields.named.iter().map(|field|{
						let name=&field.ident;
						quote_spanned!{ field.span() =>
							crate::quantify::Quantifiable::forecast_total_memory(&self.#name)
						}
					});
					quote!{ 0 #(+ #fit)* }
				}
				syn::Fields::Unnamed(ref fields) =>
				{
					let fit=fields.unnamed.iter().enumerate().map(|(i,field)|{
						let index=syn::Index::from(i);
						quote_spanned!{ field.span() =>
							crate::quantify::Quantifiable::forecast_total_memory(&self.#index)
						}
					});
					quote!{ 0 #(+ #fit)* }
				}
				syn::Fields::Unit => quote!(0),
			}
		},
		syn::Data::Enum(ref data) =>
		{
			//https://docs.rs/syn/1.0.44/syn/struct.DataEnum.html
			let vit = data.variants.iter().map(|variant|{
				let vname = &variant.ident;
				match variant.fields
				{
					syn::Fields::Named(ref fields) => 
					{
						//An struct with named fields. Blah{name1:type1, name2:type2, ...}
						let fit=fields.named.iter().map(|field|{
							let name=&field.ident;
							quote_spanned!{ field.span() =>
								crate::quantify::Quantifiable::total_memory(#name)
							}
						});
						let sum =quote!{ 0 #(+ #fit)* };
						let fit=fields.named.iter().map(|field|{
							let name=&field.ident;
							quote_spanned!{ field.span() =>
								#name
							}
						});
						let args = quote!{ #(#fit,)* };
						quote_spanned!{ variant.span() =>
							Self::#vname{#args} => #sum,
						}
					},
					syn::Fields::Unnamed(ref fields) =>
					{
						let fit=fields.unnamed.iter().enumerate().map(|(i,field)|{
							//let index=syn::Index::from(i);
							let arg=quote::format_ident!("u{}",i);
							quote_spanned!{ field.span() =>
								crate::quantify::Quantifiable::total_memory(#arg)
							}
						});
						let sum = quote!{ 0 #(+ #fit)* };
						let fit=fields.unnamed.iter().enumerate().map(|(i,field)|{
							//let index=syn::Index::from(i);
							let arg=quote::format_ident!("u{}",i);
							quote_spanned!{ field.span() =>
								#arg
							}
						});
						let args = quote!{ #(#fit,)* };
						quote_spanned!{ variant.span() =>
							Self::#vname(#args) => #sum,
						}
					},
					syn::Fields::Unit =>
					{
						//quote!(0),
						quote!(Self::#vname => 0,)
					},
				}
			});
			//FIXME: should be max instead of plus.
			let tokens=quote!{ match self { #( #vit)* } };
			//quote!{compile_error!(stringify!{#tokens});#tokens}
			tokens
		},
		syn::Data::Union(_) => unimplemented!(),
	}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
