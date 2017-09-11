 // File: The analyzer of the tool
 // Purpose: Functions defined in this file are mainly used for analyze
 //                 the symbol table, generate proper advice and print out
 //                 the advice 
 // Author : Ziling Zhou (802414)

use SymbolTable;
use builtin::Ty;
use VarInfo;
use std::collections::{HashMap};
use std::io::{BufReader,BufRead};
use std::fs::File;

impl <'a> SymbolTable<'a>{

    // Entry point of this file, start to analyze the symbol table
    // print out the generated advice
    pub fn start_analyze(&self,
                                      file_name:String
                                      ) 
    {
        println!("========================================================");
        println!("Adivice one (drop): \n");
        let mut print_list = HashMap::new();
        for (var,info) in &self.var_table{
            if !self.check_move(var){
                match info.var_type{
                   Ty::NonPrimitive=>
                    {  
                        self.choice_one_drop(&var,&info,&mut print_list);
                    },
                    _=>(),
                }
            }
        }

        for scope in & self.enclose_scope{
            scope.drop_analyze_innerscope(&mut print_list);
        }

        if print_list.is_empty(){
            println!("No adivice!");
        }else{
            let file = File::open(file_name).unwrap();
            let file_reader = BufReader::new(&file);
            let mut line_num:usize = 1;
            for line in file_reader.lines(){
                let l = line.unwrap();
                if let Some(print_lines) = print_list.get(&line_num){
                    for print_line in print_lines{
                        println!("{}", print_line);
                    }
                }

                println!("{}", l );

                line_num+=1;
            }
        }

        println!("========================================================");
        println!("Adivice two (fucntion): \n");
        let mut print_list_func:HashMap<String,Vec<bool>> = HashMap::new();
        self.choice_two_function(&mut print_list_func);
        let mut printed = false;
        for (print_fun, prints) in print_list_func {
            if (!String::eq(&print_fun,"main")) & (!String::eq(&print_fun,"new")) {
                let mut can_print = true;
                for(struct_name, _ ) in self.structure_list{
                    let new_method = struct_name.to_string() +"new";
                    if String::eq(&new_method,&print_fun){
                        can_print = false;
                        break;
                    }
                }  
                if can_print{
                    printed = true;
                    println!("{}:",print_fun);
                    let mut printed = false;
                    let mut index = 0;
                    while index< prints.len(){
                        if prints[index] == true{
                            println!("\targument {} can take ownership ", index+1 );
                            printed = true;
                        }
                        index+=1;
                    }
                    if !printed {
                        println!("\tNo advice for this function");
                    }  
                }
            }  
        }
        if !printed{
            println!("\n No advice for funciton");
        }
         println!("========================================================");
        
     }


     // Help analyze drop for inner scope
     fn drop_analyze_innerscope(&self, 
                                                    mut print_list:&mut HashMap<usize,Vec<String>>)
     {
          for (var,info) in &self.var_table{

              if !self.check_move(var){
                  match info.var_type{
                      Ty::NonPrimitive=>
                      {  
                          self.choice_one_drop(&var,&info,&mut print_list);
                      },
                      _=>(),
                  }
              }
          }
          for scope in & self.enclose_scope{
              scope.drop_analyze_innerscope(&mut print_list);
          }
     }

     // Generate advice for function
     // call  check function call for each function in function list
     fn choice_two_function(&self,
                                            print_list_func:&mut HashMap<String,Vec<bool>> )
     {
      
        for (func_name, info) in self.fun_records{
            let length = info.input;
            let mut print = vec![];
            let mut count:usize = 0;
            while count < length{
              print.push(true);
              count+=1;
            }
            self.check_function_call(func_name,length,&mut print);
            print_list_func.insert(func_name.to_string(), print);
        }
     }

     // Check each function call of a specific function.
     // If onw of a perameter's coorespond argument is used after the
     // call, then its place in print will be recorded as false.
     fn check_function_call(&self, 
                                            func_name:&str,
                                            length:usize,
                                            print:&mut Vec<bool>)
    {
        if let Some(call_infos) = self.call_records.get(func_name){
            //Each call for a specific function 
            for info in call_infos{
                let call_loc = info.call_location; 
                let mut index = 0;
                while index < length{
                    let arg_last_used = self.get_last_used(&info.arguments[index]);
                    if arg_last_used != call_loc{
                        print[index] = false;
                    }
                    index+=1;
                }
            }
        }
        for scope in & self.enclose_scope{
            scope.check_function_call(func_name,length,print);
        }
    }

     // Generate advice for function drop.
     // Check whether the given varibale can be dropped earlier, if can,
     // put it into the print list
     fn choice_one_drop(&self,
                                      var: &str,
                                      info: &VarInfo,
                                      mut print_list:&mut HashMap<usize,Vec<String>>) 
     {
          if  self.no_var_ref(var){
              match info.last_used_loc{
                  Some(line) => {
                      let mut can_print = true;
                      if (line+1) == info.when_to_drop{
                          can_print = false
                      } 
                
                      if can_print{
                          let mut found = false;
                          if let Some(print_line) = print_list.get_mut(&(line+1)){
                              found = true;
                              print_line.push("drop( ".to_string()+ var + 
                                                          " ); // Adivice: a drop function can add here");
                          }
                            
                          if ! found {
                              print_list.insert(line+1,vec!["drop( ".to_string()+ var + " ); // Adivice: a drop function can add here"]);
                          }
                      }      
                  }
                  None => (),
              }  
          } else{
              // If the variable is reffed by another variable, check when will the reference
              // be dropped
              let mut when_drop = 0;
              for  refer in &info.ref_by{
                  let tmp = self.get_when_drop(&refer,var);
                  if when_drop< tmp { when_drop = tmp } 
              }
              // If the references will be drop earlier than the variable
              if  (when_drop != 0) & (when_drop < info.when_to_drop){
                  let mut found = false;
                  if let Some(print_line) = print_list.get_mut(&(when_drop+1)){
                      found = true;
                      print_line.push("drop( ".to_string()+ var + 
                                                  " ); // Adivice: a drop function can add here");
                  }
                  if !found{
                      print_list.insert(when_drop+1,
                                                    vec!["drop( ".to_string()+ var + 
                                                    " ); // Adivice: a drop function can add here"]);
                  }
              } 
          }
      }
}    
