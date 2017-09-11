 // File: This file is the main file of the whole Borrowing advisor.
 // Purpose: Functions defined in this file are mainly used for parse
 //                 the input program into AST and traverse the whole AST 
 //                 in order to record all the needed information of the 
 //                 program into symbol table 
 // Author : Ziling Zhou (802414)

 #![feature(box_syntax)]
 #![feature(rustc_private)]
extern crate syntax;

mod pre_record;
mod builtin;
mod analyzer;
mod resolve;

use std::collections::{HashMap};
use std::env;
use std::path::Path;
use std::mem;
use std::clone::Clone;
use std::ops::DerefMut;

use syntax::ast;
use syntax::ast::{Stmt,StmtKind,PatKind,NodeId,Block,ExprKind,Mac,Expr};
use syntax::codemap::{CodeMap, Span};
use syntax::errors::DiagnosticBuilder;
use syntax::parse::{self, ParseSess};
use syntax::parse::token::Token;
use syntax::visit::{self,Visitor};
use syntax::tokenstream::{TokenTree};

use builtin::Ty;
use pre_record::FnInfo;
use resolve::Resolver;


// This function is used to build a AST tree for the input program
fn parse<'a, T: ?Sized + AsRef<Path>>(path: &T,
                                      parse_session: &'a ParseSess)
                                      -> Result<ast::Crate, Option<DiagnosticBuilder<'a>>> {
    let path = path.as_ref();

    match parse::parse_crate_from_file(path, parse_session) {
        // There may be   parse errors that the parser recovered from, which
        // would be treat as error here
        Ok(_) if parse_session.span_diagnostic.has_errors() => Err(None),
        Ok(krate) => Ok(krate),
        Err(e) => Err(Some(e)),
    }
}

// Entry piont of this program.
fn main() {
    // Take filename from command line
    let args: Vec<String> = env::args().collect();
    // Build a new parse session for parser
    let parse_session = ParseSess::new();
    // unwrap ast from result
    let krate = parse(args[1].as_str(), &parse_session).unwrap();
    // Get function and enum records from input program
    let pre_records =  pre_record::get_records(&krate);
    let fun_records = pre_records.fun_record;
    let enum_list = pre_records.enum_record;
    let structure_list = pre_records.struct_record;

    // Start to analyze the whole program
    analyze_prog(&krate,parse_session.codemap(), 
                            &fun_records,&enum_list,&structure_list,args[1].clone());

}


// Start to traverse the whole AST in order to record all the information 
// that is needed into the symbol table, then call function in analyze module
// to analyze the symbolt table
fn analyze_prog(krate:&ast::Crate,
                            codemap: &CodeMap, 
                            fun_records :& HashMap<String, FnInfo>,
                            enum_list: & HashMap<String,Vec<String>>, 
                            structure_list:&HashMap<String,HashMap<String,Ty>>,
                            file_name:String
                            ) 
{
    // Build a new SymbolTable
    let mut visitor = SymbolTable{
        var_table : HashMap::new(),
        enclose_scope : vec![],
        outer_scope : None,
        codemap : codemap,
        scope_start : codemap.lookup_char_pos(krate.module.inner.lo).line,
        scope_end : codemap.lookup_char_pos(krate.module.inner.hi).line,
        fun_records:  fun_records,
        call_records: HashMap::new(),
        enum_list: enum_list,
        structure_list:structure_list,
    };

    let node_id = NodeId::new(0);

    // Start to vistit AST from 'mod' field
    visitor.visit_mod(&krate.module,krate.span, node_id);

    // Print out all the variable information in the symbol table
    println!("\nVariable Information:\n");
    visitor.debug();

    //  Call start_analyze to analyze the symbol table
    //  Print out the analyze result (advice)
    println!("\nStart analyze...\n");
    visitor.start_analyze(file_name);
}

// Override drop for SymbolTable in order to avoid segfault
impl<'a> Drop for SymbolTable<'a>{
    fn drop(&mut self){
        mem::replace(&mut self.enclose_scope, vec![]);
        let mut outer_scope = mem::replace(&mut self.outer_scope, None);
        loop{
            outer_scope = match outer_scope{
                Some(mut n )=>mem::replace(&mut n.outer_scope, None),
                None => break,
            }
        }
    }
}

// Used to record variables and their information for each scope
pub struct SymbolTable<'a>{
    var_table:HashMap<String,VarInfo>,
    enclose_scope:Vec<Box<SymbolTable<'a>>>,
    outer_scope:Option<Box<SymbolTable<'a>>>,
    codemap:& 'a CodeMap,
    scope_start: usize,
    scope_end: usize,
    fun_records: & 'a HashMap<String, FnInfo>,
    call_records: HashMap<String,Vec<CallInfo>>,
    enum_list:&'a HashMap<String,Vec<String>>,
    structure_list:&'a HashMap<String,HashMap<String,Ty>>
}

// Record all the information of call for defined method
#[derive(Debug)]
struct CallInfo{
    receiver: Option<Vec<String>>,
    arguments: Vec<Vec<String>>,
    call_location: usize,
}

// A data structure used to record the information for each variable
#[derive(Debug)]
struct VarInfo {
    decl_loc: usize ,
    last_used_loc: Option<usize>,
    var_type: Ty,
    ref_by: Vec<Vec<String>>,
    ref_to:Option<Vec<String>>,
    moved: bool,
    when_to_drop:usize,
    structure:Option<HashMap<String,VarInfo>>,
}

    
impl <'a> SymbolTable<'a> {

    // Print all the variable information in the symbol table
    fn debug(&self){
        println!("=======================");
        println!("scope: {}-{}", self.scope_start, self.scope_end );
        println!("=======================");
        println!("--------------------------------------------------------");
        
        for (var,info) in self.var_table.iter(){
            println!("variable: {}\ndeclared on line: {}, last_used_loc: {:?}\nvar_type: {:?}, moved: {}\nref_to: {:?},ref_by:{:?}, when_to_drop: {:?}",
             var, info.decl_loc,info.last_used_loc,info.var_type,info.moved,info.ref_to,info.ref_by,info.when_to_drop);
            println!("\nFields:");
            match info.structure{
                Some(ref structure) => {
                    for (field_name, field_info) in structure{
                        println!("\n-----\"{}\"-----",field_name);
                        println!("last_used_loc: {:?}, var_type: {:?}\nmoved: {}, ref_to: {:?}\nref_by:{:?}",field_info.last_used_loc,field_info.var_type,field_info.moved,field_info.ref_to,field_info.ref_by);
                    } 
                },
                None=> println!("None"),
            }
            println!("--------------------------------------------------------");
        }
        // After printing the variable information of a symbol table
        // start to print the variable in formation of its enclosing scope
        let en = & self.enclose_scope;
                for e in en  {
                      e.debug();
          }

    }

    // Used to check whether a specific variable's ownership is moved
    fn check_move(&self, var:&str)-> bool{
        let mut is_move = false;
        if let Some(info) = self.var_table.get(var){
            if !info.moved{
                if let Some(ref structure) = info.structure{
                    for(fields, _ ) in structure{
                        is_move = is_move|check_move_struct(&fields,structure);
                    }
                }
            }else{
                is_move = true;
            }
        }  
        is_move      
    }

    // Used to check wether a specific variable is referred by another variable 
    fn no_var_ref(&self, var:&str)->bool{
        let mut is_reffered = false;
        if let Some(info) = self.var_table.get(var){
            if !info.ref_by.is_empty(){
                if let Some(ref structure) = info.structure{
                    for(fields, _ ) in structure{
                        is_reffered = is_reffered|no_var_ref_struct(&fields,structure);
                    }
                }
            }else{
                is_reffered = true;
            }
        }  
        is_reffered
    }

    // Turn a span in to a line number to should the location
    fn location(& self, span:Span) -> usize {
        self.codemap.lookup_char_pos(span.lo).line
    }

    // Record the line a variable is last used into its variable information
    fn record_last_used(&mut self, vars: &Vec<String>, line: usize){
        if let Some(info) = self.var_table.get_mut(&vars[0]){
            if vars.len() == 1{
                if info.moved {
                    // Cannot use a moved variable
                    panic!("Try to move moved value !");
                } else{
                    info.last_used_loc = Some(line);
                }
            }else{
                match &info.var_type{
                    &Ty::NonPrimitive => {
                        // Remove the first element in vector
                        let mut new_var = (*vars).clone();
                        new_var.remove(0);
                        if let Some(ref mut structure) = info.structure{
                            record_last_used_for_struct(structure,&new_var,line);
                        }
                    }
                    &Ty::Ref => info.last_used_loc = Some(line),
                    _=>(),
                }
            }
        }else{
            // If the variable can not be find in current scope, search it 
            //from outer scope
            match self.outer_scope{
                Some(ref mut table)=>{
                    let sym_table = (*table).deref_mut();
                    sym_table.record_last_used(vars,line)
               },
                 None =>(),
            }
        }
    }

      
    // When the walker get into a new scope, build a new symbol table 
    // and bund it to current scope's enclosing scope.
    fn get_in_scope(&mut self,b: &Block){
        let new =SymbolTable{
            var_table:HashMap::new(),
            enclose_scope:vec![],
            outer_scope: Some(unsafe{Box::from_raw((self as *mut SymbolTable))}),
            codemap : self.codemap,
            scope_start : self.codemap.lookup_char_pos(b.span.lo).line,
            scope_end : self.codemap.lookup_char_pos(b.span.hi).line,     
            fun_records: self.fun_records,
            call_records: HashMap::new(),
            enum_list: self.enum_list,
            structure_list:self.structure_list
        };
        self.enclose_scope.push(Box::new(new));
    }

    // When a variable's ownnership is moved, change its 'moved' state 
    fn move_ownership(&mut self, vars : &Vec<String>){
        // If var already been used, print out a error massage 
        // and terminate
        if let Some(ref mut info) = self.var_table.get_mut(&vars[0]){
            // Only NonPrimitive type can be moved
            match &info.var_type{
                &Ty::NonPrimitive=>{
                    if vars.len()==1{
                        if info.moved {
                            panic!("Try to move moved value !");
                        }else {
                            info.moved = true;
                        }
                    }else{
                        let mut new_var = (*vars).clone();
                        new_var.remove(0);
                        if let Some(ref mut structure) = info.structure{
                            move_ownership_for_struct(structure,&new_var);
                        }
                    }
                },
                _=>(),
            }
        }else{
            // If the variable can not be find in current scope, search it 
            //from outer scope
             match self.outer_scope {
                 Some(ref mut out) => {
                     let out = (*out).deref_mut() ;
                    out.move_ownership(vars);
                },
                None => (),
            }
        }
    }

    // Record the declared variable into symbol table
    fn local_lhs (&mut self , 
                        pattern:& PatKind,
                        var_ty: Ty , 
                        ref_to : Option<Vec<String>>,
                        structure:HashMap<String,Resolver>)
    {
           match pattern{
                &PatKind::Ident( _ , span_ident, _ ) => {
                    let location = self.location(span_ident.span);
                    let var_name = vec![span_ident.node.name.as_str().to_string()];

                    let mut info = VarInfo{decl_loc:location , last_used_loc : None, var_type: var_ty ,ref_by: vec![], ref_to: ref_to.clone() , moved: false , when_to_drop: self.scope_end,structure:None};
                    if !structure.is_empty(){
                        let mut var_structure = HashMap::new();
                        for (field_name,field_resolve) in structure{
                            if let Some(ty) = field_resolve.var_type{
                                let mut new_info = VarInfo{
                                    decl_loc: location ,
                                    last_used_loc: None,
                                    var_type: ty,
                                    ref_by: vec![],
                                    ref_to:field_resolve.ref_to,
                                    moved: false,
                                    when_to_drop:self.scope_end,
                                    structure:None,
                                };
                                let mut whole_field_name = var_name.clone();
                                whole_field_name.push(field_name.clone());
                                match new_info.ref_to{
                                    Some(ref ref_var) => {
                                        self.ref_to_var(ref_var,&whole_field_name);
                                    }
                                    None=>(),
                                } 

                                if !field_resolve.structure.is_empty(){
                                   new_info.structure = Some(self.build_struct(location,field_resolve.structure,&mut whole_field_name));
                                }
                                var_structure.insert(field_name,new_info);
                            }                             
                        } 
                        info.structure = Some(var_structure);
                    }
                        
                    self.var_table.insert(span_ident.node.name.as_str().to_string(),info);

                    match ref_to{
                        Some(ref_var) => {
                            self.ref_to_var(&ref_var,&var_name);
                        }
                        None=>(),
                    }                
                },
                _ => (),
            }
    }

    // Construct the structure for a specific variable
    fn build_struct(&mut self,
                            location:usize,
                            fields:HashMap<String,Resolver>,
                            whole_field_name:&mut Vec<String>
                            ) ->HashMap<String,VarInfo>{
        let mut new_hash = HashMap::new();
        
        for (field_name,field_resolve) in fields{
            if let Some(ty) = field_resolve.var_type{
                let mut new_info = VarInfo{
                    decl_loc: location ,
                    last_used_loc: None,
                    var_type: ty,
                    ref_by: vec![],
                    ref_to:field_resolve.ref_to,
                    moved: false,
                    when_to_drop:self.scope_end,
                    structure:None,
                };
                whole_field_name.push(field_name.clone());
              
                match new_info.ref_to{
                    Some(ref ref_var) => {
                        self.ref_to_var(ref_var,&whole_field_name);
                    }
                    None=>(),
                } 

                // Recursively build the sturcture until there is no more sturcture
                if !field_resolve.structure.is_empty(){
                    new_info.structure = Some(self.build_struct(location,field_resolve.structure,whole_field_name))
                }
                new_hash.insert(field_name,new_info);
            }                             
        } 
        new_hash
    }

    // Record a sepecific variable is referred by another variable 
    fn ref_to_var(&mut self, var: &Vec<String>, ref_by: &Vec<String>){
         if let Some(info) = self.var_table.get_mut(&var[0]){
                if var.len()==1{
                    info.ref_by.push((*ref_by).clone())
                }else{
                    let mut new_var = (*var).clone();
                    new_var.remove(0);
                    if let Some(ref mut structure) = info.structure{
                        ref_to_var_struct(structure,&new_var,ref_by);
                    }
                }
        } else {
            if let Some (ref mut outer) = self.outer_scope{
                outer.ref_to_var(var,ref_by)
            }else{
                panic!{"Variable cannot be found!"};
            }
        }
    }


    // Get type of a variable
    // If this variable cannot be found in current scope,
    // find it in outer scope
    fn get_var_type (& self , var: &Vec<String>) -> Ty {
         let mut var_type = Ty::Primitive;
        if let Some(info) = self.var_table.get(&var[0]){
           if var.len()==1{
                var_type = info.var_type.clone();
           }else{
                let mut new_var = (*var).clone();
                new_var.remove(0);
                if let Some(ref structure) = info.structure{
                     var_type=get_var_type_struct(structure,&new_var);
                }
           }
        } else {
            if let Some (ref outer) = self.outer_scope{
                var_type = outer.get_var_type(var)
            }else{
                panic!{"Variable cannot be found!"};
            }
        }
        var_type
    }

    fn get_last_used(&self,var: &Vec<String>) ->usize{
        let mut last_used:usize = 0;
        if let Some(info) = self.var_table.get(&var[0]){
            if var.len()==1{
                if let Some(last_used_loc) = info.last_used_loc{
                    last_used = last_used_loc;   
                }
            }else{
                let mut new_var = (*var).clone();
                new_var.remove(0);
                if let Some(ref structure) = info.structure{
                    last_used=get_last_used_struct(structure,&new_var);
                }
            }
        }else {
            if let Some (ref outer) = self.outer_scope{
                 last_used = outer.get_last_used(var);
            }else{
                panic!{"Variable cannot be found!"};
            }
        }

        last_used
    }

    // The the line on which a specific variable will get out of scope
    fn get_when_drop(&self, var: &[String], ref_to: &str ) -> usize{
        if let Some(info) = self.var_table.get(&var[0]){
           if let Some( _ ) = info.ref_to{
               return  info.when_to_drop
           }
        }
        let mut when_drop = 0;
       for scope in &self.enclose_scope{
             when_drop = scope.get_when_drop(var , ref_to)
        }
        when_drop      
    }

    // If there are variables that are used insid a if statement, record
    // there last used location as the last line of the if statement
    fn record_last_used_for_if(&mut self, start: usize, end: usize, scope_num:usize){
        let len = self.enclose_scope.len();
        let mut count = 1;
        while count<scope_num+1 {
            let ref mut cur_scope = self.enclose_scope[len-count];
            for info in cur_scope.var_table.values_mut(){
                match info.last_used_loc{
                    Some(loc)=> {
                        if (loc >= start) & (loc <= end) {
                            if info.decl_loc< start{
                                info.last_used_loc = Some(end)
                            }
                        }
                    },
                    None =>(),
                }
            }
            match self.outer_scope{
                    Some(ref mut table)=>{
                        let sym_table = (*table).deref_mut();
                        sym_table.record_last_used_for_if(start,end,scope_num)
                   },
                     None =>(),
             }
             count+=1;
        }
    }

    fn record_call_loc_for_if(&mut self, start: usize, end: usize,scope_num:usize){
        let len = self.enclose_scope.len();
        let mut count = 1;
        while count<scope_num+1 {
            let ref mut cur_scope = self.enclose_scope[len-count];        
            for infos in cur_scope.call_records.values_mut(){
                for info in infos{
                     if (info.call_location >= start) & (info.call_location <= end) {
                           info.call_location = end
                    }
                }
            } 
            count+=1;   
        }
    }

    // Check whether a specific variable is exist.
    fn check_var(&self , var:&Vec<String>) -> bool{
        let mut re = false;
        
        if let Some(info) = self.var_table.get(&var[0]){
            if var.len()==1{
                re = true
            }else{                
                let mut new_var = (*var).clone();
                new_var.remove(0);
                if let Some(ref structure) = info.structure{
                    re=check_var_struct(structure,&new_var);
                }
            }
        }else{
            if let Some(ref outer) = self.outer_scope{
                re = outer.check_var(var);
            }
        }
        re
    }

    // Change the whole variable information of a variable when
    // the visitor meet a asignment of the variable.
    fn change_var_info(&mut self ,
                                    var:&Vec<String>,
                                    info:VarInfo)
    {
        if let Some(old_info) = self.var_table.get_mut(&var[0]){
            if var.len()==1{
                   *old_info = info;
            }else{                
                let mut new_var = (*var).clone();
                new_var.remove(0);
                if let Some(ref mut structure) = old_info.structure{
                    change_var_info_struct(&new_var,structure,info);
                }
            }
        }else{
            if let Some(ref mut outer) = self.outer_scope{
                outer.change_var_info(var,info);
            }
        }
    }


    //When visitor meet assignment expression, put the variable into var_table
    // or change the exist variable's information
    fn assign_var(&mut self,
                            var: &Vec<String>,
                            var_ty: Ty , 
                            ref_to : Option<Vec<String>>,
                            structure:HashMap<String,Resolver>,
                            span:Span
                            )
    {
        let location = self.location(span);
        let mut info = VarInfo{decl_loc:location , last_used_loc : None, var_type: var_ty ,ref_by: vec![], ref_to: ref_to.clone() , moved: false , when_to_drop: self.scope_end,structure:None};
        if !structure.is_empty(){
            let mut var_structure = HashMap::new();
            for (field_name,field_resolve) in structure{
                if let Some(ty) = field_resolve.var_type{
                    let mut new_info = VarInfo{
                        decl_loc: location ,
                        last_used_loc: None,
                        var_type: ty,
                        ref_by: vec![],
                        ref_to:field_resolve.ref_to,
                        moved: false,
                        when_to_drop:self.scope_end,
                        structure:None,
                    };
                    let mut whole_field_name = vec![var[0].clone()];
                    whole_field_name.push(field_name.clone());
                    match new_info.ref_to{
                        Some(ref ref_var) => {
                            self.ref_to_var(ref_var,&whole_field_name);
                        }
                        None=>(),                
                    } 

                    if !field_resolve.structure.is_empty(){
                        new_info.structure = Some(self.build_struct(location,field_resolve.structure,&mut whole_field_name));
                    }  
            
                    var_structure.insert(field_name,new_info);
                }                             
            } 
            info.structure = Some(var_structure);
        }
        if self.check_var(var){
            self.change_var_info(var,info);
        }else{
            if var.len() ==1{
                self.var_table.insert(var[0].clone(),info); 
            }else{
                panic!{"Unresolved field!"};
            }
        }
        match ref_to{
            Some(ref_var) => {
                self.ref_to_var(&ref_var,&vec![var[0].clone()]);            
            }
            None=>(),
        }
    }   

    // when a specific virable is used in a statement, change its last used
    // location and chang its state of moved if needed
    fn change_and_used(&mut self,
                                    resolver: &resolve::Resolver,
                                    line: usize)
    {
        if let Some(ref var_name) = resolver.var_name{
            self.record_last_used(var_name,line);
            match self.get_var_type (var_name) {
                Ty::NonPrimitive =>self.move_ownership(var_name),
                 _=>() ,                          
            }
        }else{                    
            if let Some(ref var_type) = resolver.var_type{
                match var_type{
                    &Ty::Ref=> {
                        if let Some(ref ref_to) = resolver.ref_to{
                            self.record_last_used(ref_to,line)
                        }
                    },
                    _=>(),
                }
            }
        }             
    }
}

// fn get_var_ref_struct(
//                                     structure:&HashMap<String,VarInfo>,
//                                     var:&Vec<String>)-> Option<Vec<String>>{
//     let mut ref_to_var = Some(vec![]);
//     if let Some(info) = structure.get(&var[0]){
//         if var.len()==1{
//             ref_to_var = info.ref_to.clone();
//         }else{
//             let mut new_var = (*var).clone();
//             new_var.remove(0);
//             if let Some(ref structure) = info.structure{
//                 ref_to_var=get_var_ref_struct(structure,&new_var);
//             }
//         }
//     }
//     ref_to_var  
// }


// Get type of a filed of structure
// If this variable cannot be found in current scope,
// find it in outer scope
fn get_var_type_struct(
                                    structure:&HashMap<String,VarInfo>,
                                    var:&Vec<String>)->Ty
{
    let mut var_type = Ty::Primitive;
    if let Some(info) = structure.get(&var[0]){
        if var.len()==1{
            var_type = info.var_type.clone();
        }else{
            let mut new_var = (*var).clone();
            new_var.remove(0);
            if let Some(ref structure) = info.structure{
                var_type=get_var_type_struct(structure,&new_var);
            }
        }
    }
    var_type
}

fn get_last_used_struct(structure:&HashMap<String,VarInfo>,
                                        var: &Vec<String>) ->usize
{
    let mut last_used:usize = 0;
    if let Some(info) = structure.get(&var[0]){
        if var.len()==1{
            if let Some(last_used_loc) = info.last_used_loc{
                last_used = last_used_loc;   
            }
        }else{
            let mut new_var = (*var).clone();
            new_var.remove(0);
            if let Some(ref structure) = info.structure{
                last_used=get_last_used_struct(structure,&new_var);
            }
        }
    }
    last_used
}

// check whether a field is exist
fn check_var_struct(
                                structure:&HashMap<String,VarInfo>,
                                var:&Vec<String>) -> bool
{
    let mut re = false;

    if let Some(info) = structure.get(&var[0]){
        if var.len()==1{
            re = true
        }else{
            let mut new_var = (*var).clone();
            new_var.remove(0);
            if let Some(ref structure) = info.structure{
                re = check_var_struct(structure,&new_var);
            }
        }
    }
    re
}

// Record last used location for a field
fn record_last_used_for_struct(
                                                    structure:&mut HashMap<String, VarInfo>,
                                                    vars:&Vec<String>,
                                                    line:usize
                                                    )
{
    if let Some(info) = structure.get_mut(&vars[0]){
        if vars.len() == 1{
            if info.moved {
                panic!("Try to move moved value !");
            } else{
                info.last_used_loc = Some(line);
            }
        }else{
            match &info.var_type{
               & Ty::NonPrimitive => {
                    let mut new_var = (*vars).clone();
                    new_var.remove(0);
                    if let Some(ref mut structure) = info.structure{
                        record_last_used_for_struct(structure,&new_var,line);
                    }
                },
                &Ty::Ref => info.last_used_loc = Some(line),
                _=>(),
            }
        }
    }
}

// Change the moved state for a field
fn move_ownership_for_struct(
                                                    structure:&mut HashMap<String, VarInfo>,
                                                    vars:&Vec<String>,
                                                    )
{
    if let Some(info) = structure.get_mut(&vars[0]){
        match  info.var_type {
            Ty::NonPrimitive=>{
                if vars.len()==1{
                    if info.moved {
                        panic!("Try to move moved value !");
                    }else {
                         info.moved = true;
                    }
                }else{
                    let mut new_var = (*vars).clone();
                    new_var.remove(0);
                    if let Some(ref mut structure) = info.structure{
                        move_ownership_for_struct(structure,&new_var);
                    }
                }
            },
            _=>(),
        }
    }
}

// Record a field is referred by another variable
fn ref_to_var_struct(
                                structure:&mut HashMap<String,VarInfo>,
                                var:&Vec<String>,
                                ref_by:&Vec<String>)
{
    if let Some(info) = structure.get_mut(&var[0]){
        if var.len()==1{
            info.ref_by.push((*ref_by).clone());
        }else{        
            let mut new_var = (*var).clone();
            new_var.remove(0);
            if let Some(ref mut structure) = info.structure{
                ref_to_var_struct(structure,&new_var,ref_by);
            }
        }
    }
}

// Check whether a field is moved
fn check_move_struct(
                                    var:&str,
                                    structure:& HashMap<String,VarInfo>
                                    )->bool
{
    let mut is_move = false;
    if let Some(info) = structure.get(var){
        if !info.moved{
            if let Some(ref structure) = info.structure{
                for(fields, _ ) in structure{
                    is_move=is_move|check_move_struct(&fields,structure);
                }
            }
        }else{
            is_move = true;
        }
    }
    is_move
}

// Check whether a field is referred by another variables
fn no_var_ref_struct(var:&str,
                                structure:& HashMap<String,VarInfo>
                                )->bool
{
    let mut is_reffered = false;
    if let Some(info) = structure.get(var){
        if !info.ref_by.is_empty(){
            if let Some(ref structure) = info.structure{
                for(fields, _ ) in structure{
                    is_reffered = is_reffered|no_var_ref_struct(&fields,structure);
                }
            }
        }else{
            is_reffered = true;
        }
    }  
    is_reffered
}

// Change a field's variable information when there is a assignement for
// the field
fn change_var_info_struct(var:&Vec<String>,
                                            structure:&mut HashMap<String,VarInfo>,
                                            info:VarInfo)
    {
        if let Some(old_info) = structure.get_mut(&var[0]){
            if var.len()==1{
                   *old_info = info;
            }else{                
                let mut new_var = (*var).clone();
                new_var.remove(0);
                if let Some(ref mut structure) = old_info.structure{
                    change_var_info_struct(&new_var,structure,info);
                }
            }
        }else{
            panic!{"Unresolved field!"};
        }
    }


impl <'v,'a> Visitor <'v> for SymbolTable <'a> {

    // Once get into a new block, construct a new scope and change
    // the caller into the new symbol table
    fn visit_block( &mut self, b: &'v Block){
        self.get_in_scope(b);
        let len = self.enclose_scope.len();
       {
            let ref mut cur_scope = self.enclose_scope[len-1];  
            visit::walk_block((*cur_scope).deref_mut() ,b);
        }
    }
    
    // Visit each statment in block
    fn visit_stmt ( &mut self, s : &'v Stmt){
      
        let stmtkind = & s.node;
        match stmtkind {
            // When  a let binding occurs, record the declared variable and 
            // the virable whose value is moved, if any. 
            &StmtKind::Local (ref l) => {	
                let init = &l.init;
                // LHS of Local           
                let pattern =  &l.pat.node;    
                // Deal with the right hand side of the let binding
                match init.as_ref() {
                    Some(ref expr) => { 
                        let resolver = resolve::resolve_expr(expr, self.enum_list,self.fun_records,self,self.structure_list);
                        let line = self.location(expr.span);
                        self.change_and_used(&resolver,line);
                        if let Some(var_type) = resolver.var_type{
                            if let Some(ref_to) = resolver.ref_to{
                                 self.local_lhs (pattern, var_type, Some(ref_to),resolver.structure);
                            }else{
                                 self.local_lhs (pattern, var_type, None,resolver.structure);
                            }
                        }
                    },
                    // If there is nothing on the right hand side 
                    None =>(),
                  }
                visit::walk_stmt(self,s)
            },
            _ => visit::walk_stmt(self,s),
        }
    }

    // Visit the expression of the AST
    fn visit_expr(&mut self, ex: &'v Expr){
        let line = self.codemap.lookup_char_pos(ex.span.hi).line;
        match &ex.node{
            // When visit a match expression, check whether the ownership of variable 
           // used match is passed 
            &ExprKind::Match( ref expr, _)=>{
                let resolve = resolve::resolve_expr(expr,self.enum_list,self.fun_records,self,self.structure_list);
                self.change_and_used(&resolve,line);
                visit::walk_expr(self,ex);
            },
            // When the visited expr is a Methodcall, record when are the variables
            // used and moved.
            &ExprKind::MethodCall(ref func_name, _ , ref args) =>{
                let function = func_name.node.name.as_str().to_string();
                let mut record_call = false;
                
               let caller_resolve = resolve::resolve_expr(&args[0],self.enum_list,self.fun_records,self,self.structure_list); 
                
                let mut call_info = CallInfo{
                    receiver: caller_resolve.var_name,                   
                    arguments: vec![],
                    call_location: line,
                };
                
                if self.fun_records.contains_key(&function){
                    record_call =true;
                }

                // When a method is called, check the type of each argument
                let mut count = 1;
                while count < args.len(){
                    let arg = &args[count]; 
                    let resolve = resolve::resolve_expr(arg,self.enum_list,self.fun_records,self,self.structure_list);
                    self.change_and_used(&resolve,line);
                    match resolve.var_name{
                        Some(arg_name)=> call_info.arguments.push(arg_name),
                        None =>{
                            match resolve.ref_to{
                                Some(arg_name)=> call_info.arguments.push(arg_name),
                                None=>()
                            }
                        }
                    }

                    count +=1;
                } 

                if record_call{
                    if self.call_records.contains_key(&function){
                        if let Some(info) = self.call_records.get_mut(&function){
                            info.push(call_info);
                        }
                    }else{
                        self.call_records.insert(function,vec![call_info]);
                    }
                }

                visit::walk_expr(self,ex);   
            },
            // When the visited expr is a Call, record when are the variables
            // used and moved.
            &ExprKind::Call(ref fun_name, ref args)=>{
                let mut function = "".to_string();
                match &fun_name.node{
                    &ExprKind::Path(_, ref p) if p.segments.len() == 1 =>{
                        function = p.segments[0].identifier.name.as_str().to_string();
                    },
                    _=>(), 
                }
                let mut record_call = false;

                let mut call_info = CallInfo{
                    receiver: None,
                    arguments: vec![],
                    call_location: line,
                };
                
                if self.fun_records.contains_key(&function){
                    record_call =true;
                }

                for arg in args {
                    let resolve = resolve::resolve_expr(arg,self.enum_list,self.fun_records,self,self.structure_list);
                    self.change_and_used(&resolve,line);

                    match resolve.var_name{
                        Some(arg_name)=> call_info.arguments.push(arg_name),
                        None =>{
                            match resolve.ref_to{
                                Some(arg_name)=> call_info.arguments.push(arg_name),
                                None=>()
                            }
                        }
                    }
                }
              
               if record_call{
                    if self.call_records.contains_key(&function){
                        if let Some(info) = self.call_records.get_mut(&function){
                            info.push(call_info);
                        }
                    }else{
                        self.call_records.insert(function,vec![call_info]);
                    }
                }

                visit::walk_expr(self,ex);                            
            },
            // When visitor meet If or while expression
            // Set the last used location of all the variables that are used 
            // inside this expression to the last line of this expression
            &ExprKind::If (_, _, _) | &ExprKind::While(_,_,_)=>{
                let start =  self.codemap.lookup_char_pos(ex.span.lo).line;
                let end = self.codemap.lookup_char_pos(ex.span.hi).line;
                visit::walk_expr(self,ex);
                self.record_last_used_for_if(start,end,2);
                self.record_call_loc_for_if(start,end,2);

            },
            &ExprKind::Loop(_,_)=>{
                let start =  self.codemap.lookup_char_pos(ex.span.lo).line;
                let end = self.codemap.lookup_char_pos(ex.span.hi).line;
                visit::walk_expr(self,ex);
                self.record_call_loc_for_if(start,end,1);
                self.record_last_used_for_if(start,end,1);
            }
            // When visitor meet assign, check whether the name is exist.
            &ExprKind::Assign(ref lvalue,ref rvalue)=>{
                let lvalue_resolve = resolve::resolve_expr(lvalue,self.enum_list, self.fun_records, self,self.structure_list);
                let rvalue_resolve = resolve::resolve_expr(rvalue,self.enum_list,self.fun_records,self,self.structure_list);
               
                self.change_and_used(&rvalue_resolve,line);

                if let Some(ref var) = lvalue_resolve.var_name{
                    if let Some(var_type) = rvalue_resolve.var_type{
                        if let Some(ref_to) = rvalue_resolve.ref_to{
                             self.assign_var(var, var_type, Some(ref_to),
                                                        rvalue_resolve.structure, ex.span);
                        }else{
                             self.assign_var(var, var_type, None,
                                                    rvalue_resolve.structure, ex.span);
                        }
                    }
                }
                visit::walk_expr(self,ex);
            }
            &ExprKind::Binary(ref binop,ref first, ref second)=>{
                if !binop.node.is_comparison(){
                    let first_resolve = resolve::resolve_expr(first,
                                                                                        self.enum_list,
                                                                                        self.fun_records,
                                                                                        self,
                                                                                        self.structure_list);
                    let second_resolve = resolve::resolve_expr(second,
                                                                                            self.enum_list,
                                                                                            self.fun_records,
                                                                                            self,
                                                                                            self.structure_list);

                    self.change_and_used(&first_resolve,line);
                    self.change_and_used(&second_resolve,line);
                
                }
            }
            _=>visit::walk_expr(self,ex),
        }     
    }


    // When a macro is meet, check which variable is used in this macro
    fn visit_mac (&mut self, _mac: &'v Mac) {
        let mut idents = vec![];
        let tts = _mac.node.stream().into_trees();
        for tokentree in tts{
            match tokentree {
                TokenTree::Token(_,token) =>{
                    match token {
                        Token::Ident(ident) => idents.push(ident),
                        _=>(),
                    }
                }
                _=>(),
            }
        }
        for ident in idents{
            //??????
            let var = vec![ident.name.as_str().to_string()];
            let line = self.location(_mac.span);
            self.record_last_used(&var, line)
        }
        visit::walk_mac (self, _mac)
    }
}



