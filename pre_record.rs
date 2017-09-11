// File: module pre-record
// Purpose: This module is used to visit the whole AST and record the 
// 	        imformation of these functions.
//                  Functions of visting the AST should be private in this module
// Author : Ziling Zhou (802414)


use std::collections::HashMap;

use syntax::ast;
use syntax::ast::{VariantData,NodeId,Item,ItemKind,FunctionRetTy,Mac,TyKind,FnDecl};
use syntax::codemap::Span;
use syntax::visit::{self,Visitor,FnKind};

use builtin::Ty;
use syntax::ptr::P;

#[derive(Debug)]
pub struct  FnInfo{
	pub output: Option<Ty>,
	pub input:usize,

}
// Structure that used to store information that are recorded
pub struct PreRecord{
	pub fun_record: HashMap<String, FnInfo>,
	pub enum_record: HashMap<String,Vec<String>>,
	pub struct_record: HashMap<String,HashMap<String,Ty>>
}


// Visit the AST to record enum, sturcture and function that 
// are define in the input function  
impl <'v> Visitor<'v> for PreRecord {

	fn visit_item(&mut self, item: &'v Item){
		match &item.node {
			// Enum type
			& ItemKind::Enum(ref ed,_)=>{
				let mut var_list = vec![];
				for variant in &ed.variants{
					let variant_name=variant.node.name.name.as_str().to_string();
					var_list.push(variant_name);
				}
				self.enum_record.insert(item.ident.name.as_str().to_string(), var_list);
			},
			//Structure
			& ItemKind::Struct(ref variants,_)=>{
				let mut new_struct = HashMap::new();
				match variants{
					&VariantData::Struct(ref fields,_)=>{
						for field in fields{
							let mut name = " ".to_string();
							if let Some(ref field_name) = field.ident{
								name = field_name.name.as_str().to_string();
							}
							let ty = classify(&field.ty);
							new_struct.insert(name,ty);
						}
					},
					_=>(),
				}
				self.struct_record.insert(item.ident.name.as_str().to_string(),new_struct);
				let new_method = item.ident.name.as_str().to_string() + "new";
				let fun_rec = FnInfo{output:Some(Ty::NonPrimitive), input: 0 };
				self.fun_record.insert(new_method, fun_rec);
			}
			_=>print!(""),
		}
		
		visit::walk_item(self,item);
	}
	// Fucntion defined directly or inside the inpl block 
	fn visit_fn(&mut self, fk: FnKind<'v>, fd: &'v FnDecl,_: Span,_: NodeId){	
		match &fk{
			&FnKind::ItemFn( ref ident,_,_,_,_,_,_)|&FnKind::Method(ref ident,_,_,_) => {
				let re_ty = fd.output.clone();
				let mut arg_num = fd.inputs.len();
				match &fk{
					&FnKind::Method(_,_,_,_) => arg_num-=1,
					_=>(),
				}
				let mut fun_rec = FnInfo{output:None, input: arg_num };
				match re_ty{
					FunctionRetTy::Ty(ref t) =>fun_rec.output = Some(classify(t)) ,
					_=>(),
				};
				
				self.fun_record.insert(ident.name.as_str().to_string(), fun_rec);
 
			},
			_ => (),
		}
	}


	 fn visit_mac(&mut self, _mac: &'v Mac){
        		visit::walk_mac (self, _mac)
     	}
} 

// Entry point of this file
// Start to visit the AST 
pub fn get_records(krate:&ast::Crate) -> PreRecord{
	let mut pre_record = 
		PreRecord{ 
			fun_record:HashMap::new(),
			enum_record:HashMap::new(),
			struct_record:HashMap::new(),
		};
	let node_id = NodeId::new(0);

	pre_record.visit_mod(&krate.module,krate.span,node_id);
	pre_record
}

// Classify the return_type that is defined in the ast.rs of Rust
// into the type that is needed in this tool
pub fn classify (re_ty: & P<ast::Ty>) -> Ty{
	let ref return_type = re_ty.node;
	match return_type{
		&TyKind::Path( _ , ref p) =>{
			let ident = p.segments[0].identifier.name.as_str().to_string();
			let i = ident.as_str(); 
			match i{
				"i32" | "bool" | "char"| "f32"| "f64" | "i16" |"i64" |"i8" | "isize" |
				"u16"| "u32" | "u64" |"u8" |"usize" | "i128"| "u128" => Ty::Primitive,
				_ => Ty::NonPrimitive,
			}	
		},
		&TyKind::Rptr( _ , _ ) |&TyKind::Ptr(_) => Ty::Ref,
		&TyKind::Slice(ref t) => classify(t),
		&TyKind::Array(ref t , _ ) => classify(t),
		_ =>Ty::NonPrimitive,
	}
}