 // File: The resolverer of the tool
 // Purpose: Functions defined in this file are mainly used for resolving
 //                 the input expression and returning a resolver that contain
 //                 the needed information
 // Author : Ziling Zhou (802414)

use builtin::{self,Ty};
use pre_record::{self,FnInfo};
use SymbolTable;
use syntax::ast::Path;
use std::collections::HashMap;
use syntax::ast::{UnOp,Expr,ExprKind,Block,StmtKind};
#[derive(Debug)]
pub struct Resolver{
    pub var_name: Option<Vec<String>>,
    pub var_type: Option<Ty>,
    pub ref_to:Option<Vec<String>>,
    pub structure:HashMap<String,Resolver>,
}

impl Resolver{
    fn resolve_path_for_var(&mut self, 
                                            p: &Path, 
                                            enum_list:& HashMap<String,Vec<String>>,
                                            symbol_table:&SymbolTable,
                    
                                            )
    {
        match p.segments.len() {
            // If Path only has one segment, and it represent a variable declared 
            // before, then record its name and type
            1 => {
                let ident = p.segments[0].identifier.name.as_str().to_string();
                if String::eq(&ident,"None"){
                    self.var_type=Some(Ty::Primitive);
                    return
                }
                // Get name of variable 
                let var = vec![p.segments[0].identifier.name.as_str().to_string()];
                //Check whether this var is in symbol_table
                if symbol_table.check_var(&var){
                    self.var_type = Some(symbol_table.get_var_type(&var));
                    self.var_name= Some(var.clone());
                }
            }
            // If Path has 2 segments, check whrther this path represent a enum type 
            // declared before. If it represent a enum type, then record this expression
            // as NonPrimitive
            2 => {
                if let Some(coms) = enum_list.get(&p.segments[0].identifier.name.as_str().to_string()){
                    let expect = p.segments[1].identifier.name.as_str().to_string(); 
                    for com in coms{
                        if String::eq(&com,&expect){
                                self.var_type=Some(Ty::NonPrimitive);
                                break
                        }
                    }
                }
            }
           // If Path has more than 3 segments, then it might point to something in 
           // other file, which is not considered in this program.
            _ => (),
        }
    }
    // Only consider the situation that the last statement of block is 
    // an expression. The type of block depends on the type of last expression.
    fn resolve_block_for_var(&mut self,
                                             block:&Block, 
                                             enum_list:& HashMap<String,Vec<String>>,
                                             fun_records:&HashMap<String, FnInfo>,
                                             symbol_table:&SymbolTable,
                                             structure_list: &HashMap<String,HashMap<String,Ty>>)
    {
        let stmts = &block.stmts;
        let length = stmts.len();
        let stmt = &stmts[length-1];
        match &stmt.node {
            &StmtKind::Expr(ref ex) => self.resolve_expr_for_var(ex,enum_list,fun_records,symbol_table,structure_list),
            _=>() 
        }

    }

    // Resolve several kinds of expressions
    fn resolve_expr_for_var(&mut self,
                                             ex:&Expr, 
                                             enum_list:& HashMap<String,Vec<String>>,
                                             fun_records:&HashMap<String, FnInfo>,
                                             symbol_table:&SymbolTable,
                                             structure_list: &HashMap<String,HashMap<String,Ty>>
                                             )
    {
        match &ex.node{
            //Path 
            &ExprKind::Path(_, ref p) => self.resolve_path_for_var(p,enum_list,symbol_table),
            //Reference
            &ExprKind::AddrOf(_ ,ref e) => {
                self.resolve_expr_for_var(e,enum_list,fun_records,symbol_table,structure_list);
                // If the path in a reference point to a declared variable,then
                // set var_type to Ref and var_name to none, and set ref_to to the 
                // variable name that this expression refer to                
                if let Some(ref var) = self.var_name{
                    self.ref_to = Some(var.clone());
                    self.var_type = Some(Ty::Ref);
                } else{
                    // If the path does no point to a specified variable, then if 
                    // the path point to a NonPrimitive type then set var_type to
                    //ref.
                    if let Some(ref mut ty) = self.var_type{
                        match ty{
                            &mut Ty::NonPrimitive => {
                                *ty = Ty::Ref;
                            },
                            _ => (), 
                        }
                    }
                }
                self.var_name = None;
            },
            // Literal,for example : 1 or "foo"
            &ExprKind::Lit(_) => self.var_type = Some(Ty::Primitive),
            // Call, for example: foo(..) or A::B::foo(..)
            &ExprKind::Call(ref function,ref args) =>{

                //Check function's return_type
                match &function.node {
                    &ExprKind::Path(_,ref p) =>{
                        // Check whether is option.
                        if String::eq(&p.segments[0].identifier.name.as_str().to_string().to_string(),"Some"){
                            let mut arg_resolver = Resolver{
                                        var_name:None,
                                        var_type:None,
                                        ref_to:None,
                                        structure:HashMap::new(),
                                        };
                            arg_resolver.resolve_expr_for_var(&args[0],enum_list,fun_records,symbol_table,structure_list);
                            self.var_type = arg_resolver.var_type;
                            return
                        }
                        let funs = p.segments.clone();
                        let mut function_name = " ".to_string(); 
                        for fun in funs {
                            function_name = function_name+ &(fun.identifier.name.as_str().to_string());
                        }
 
                        let function_name = function_name.trim();
                     
                        // Builtin need to change         
                        let mut return_type = None;
                        if let Some(ref info) = fun_records.get(function_name) {
                            // Check whether  the method is a constructor of a structure
                            let mut is_constructor = false;
                            let mut structure_name ="";
                            for(struct_name, _ ) in structure_list{
                                let new_method = struct_name.to_string() +"new";
                                if String::eq(&new_method,&function_name){
                                    is_constructor= true;
                                    structure_name = struct_name;
                                    break;
                                }
                            } 
 
                            if is_constructor{
                                self.var_type = Some(Ty::NonPrimitive);
                                if let Some(fields) = structure_list.get(structure_name){
                                    for (field_name, field_type) in fields{
                                        let field_resolver = Resolver{
                                            var_name:None,
                                            var_type:Some(field_type.clone()),
                                            ref_to:None,
                                            structure:HashMap::new(),
                                        };
                                        self.structure.insert(field_name.clone(),field_resolver);                
                                    }

                                }
                            }else if let Some(ref re_ty) = info.output{
                                return_type = Some(re_ty.clone());
                            }
                        }else{
                            return_type =Some(builtin::get_func_rety(function_name, None));
                        }
                        match &self.var_type{
                            &None => self.var_type = return_type,
                            _=>(),
                        }
                        
                    },
                     _ =>(),
                }
            },
            // MethodCall, for example: x.foo(..)
            &ExprKind::MethodCall(ref func_name, _ , ref args) =>{
                // Get function name 
                let function = func_name.node.name.as_str().to_string();
            
                let ref receiver = args[0] ;
                // Use a new resolver to resolve receiver
                let mut receiver_resolver = Resolver{
                    var_name:None,
                    var_type:None,
                    ref_to:None,
                    structure:HashMap::new(),
                };

                let mut return_type = None;    
               
                receiver_resolver.resolve_expr_for_var(receiver,enum_list,fun_records,symbol_table,structure_list);
                
                if let Some(info) = fun_records.get(&function) {
                    if let Some(ref re_ty) = info.output{
                        return_type = Some(re_ty.clone());
                    }
                }else{
                    let receiver_type = receiver_resolver.var_type.clone();
                    return_type =Some(builtin::get_func_rety(&function, receiver_type));
                }

                self.var_type = return_type;
            },
            //Array. Type of array depends on its components type.
            &ExprKind::Array(ref exprs)=>{
                let comp = &exprs[0];
                self.resolve_expr_for_var(comp,enum_list,fun_records,symbol_table,structure_list);
                self.var_name = None;
            },
            &ExprKind::Repeat(ref expr,_)=>{
                self.resolve_expr_for_var(expr,enum_list,fun_records,symbol_table,structure_list);
                self.var_name = None;
            },
            &ExprKind::If(_,ref block,_)|&ExprKind::Block(ref block)=>{
                self.resolve_block_for_var(block,enum_list,fun_records,symbol_table,structure_list);
                self.var_name = None;
            },
            // Binary expression's type depends on its first expression
            &ExprKind::Binary(ref bo,ref first_ex, _)=>{
               if bo.node.is_comparison(){
                    self.var_type = Some(Ty::Primitive);
               }else{
                    self.resolve_expr_for_var(first_ex,enum_list,fun_records,symbol_table,structure_list);
                    self.var_name = None;
               }
            },
            //??????????
            &ExprKind::Unary(ref uo, ref ex)=>{
                match uo{
                    &UnOp::Deref=>{
                        self.resolve_expr_for_var(ex,enum_list,fun_records,symbol_table,structure_list);
                        if let Some(ref var) = self.ref_to{
                            self.var_type = Some(symbol_table.get_var_type(var));
                        } else {
                            panic!{"{:?} can not be dereferenced!",self.var_name}
                        }
                        self.var_name =None;
                    },
                    _=>self.var_type = Some(Ty::Primitive),
                }
            }

            &ExprKind::Cast(_,ref ty) =>{
                let return_type = pre_record::classify(ty);
                self.var_type = Some(return_type);
            },
             &ExprKind::Index(ref expr,_)=>{
                self.resolve_expr_for_var(expr,enum_list,fun_records,symbol_table,structure_list);
             },
             &ExprKind::Struct(_, ref fields, _ )=>{
                self.var_type = Some(Ty::NonPrimitive);
                for field in fields{
                    let mut field_resolver=Resolver{
                        var_name:None,
                        var_type:None,
                        ref_to:None,
                        structure:HashMap::new(),
                    };
                    field_resolver.resolve_expr_for_var(&field.expr,enum_list,fun_records,symbol_table,structure_list);
                    field_resolver.var_name = None;
                    self.structure.insert(field.ident.node.name.as_str().to_string(),field_resolver);
                }
             },
             &ExprKind::Field(ref ex, ref span_ident)=>{
                    let mut var = vec![span_ident.node.name.as_str().to_string()];
                    let mut tem_resolver=Resolver{
                        var_name:None,
                        var_type:None,
                        ref_to:None,
                        structure:HashMap::new(),
                    };
                    tem_resolver.resolve_expr_for_var(ex,enum_list,fun_records,symbol_table,structure_list);
                    if let Some(mut rest) = tem_resolver.var_name{
                        rest.append(&mut var);
                        self.var_type=Some(symbol_table.get_var_type(&rest));
                        self.var_name = Some(rest);

                    }
             },
             // &ExprKind::Mac(ref mac)=>{
             // }
            _=>(),
        }
    }
}

// Entry point of this file.
// Takes the input expression, resolves it and return needed information 
pub fn resolve_expr(ex:& Expr, 
                                    enum_list:& HashMap<String,Vec<String>>,
                                    fun_records:&HashMap<String, FnInfo>,
                                    symbol_table:&SymbolTable,
                                    structure_list: &HashMap<String,HashMap<String,Ty>>)
                                    -> Resolver
{
    let mut resolve = Resolver{
        var_name:None,
        var_type:None,
        ref_to:None,
        structure:HashMap::new(),
    };

    resolve.resolve_expr_for_var(ex,enum_list,fun_records,symbol_table,structure_list);
    resolve
}