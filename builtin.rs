// File: Rust built-in function checker
// Purpose: Take the input function name to match with built-in 
//                 function of Rust that is recorded and return the cooresponding
//                 return type.
// Author : Ziling Zhou (802414)

#[derive(Debug,Clone)]
pub enum Ty{
    Ref,
    NonPrimitive,
    Primitive
}

// Entry point of this file
// Take the function name and receiver, call the relative
// function depends on type of receiver in order to get the
// return type of tunction  
pub fn get_func_rety( func_name : &str, var_ty : Option<Ty>) ->Ty{
    if let Some(var_type) = var_ty{
        match var_type {
            Ty::Primitive =>  get_prim_func_rety(func_name),
            Ty::NonPrimitive =>get_non_prim_func_rety(func_name),
            Ty::Ref=>get_ref_func_rety(func_name),
        }
    } else {
        get_call_rety(func_name)
    }
}

// Return the return type of functions that have 
// primitive receiver
fn get_prim_func_rety (func_name:&str) -> Ty {
    match func_name {
        "to_string" | "into_string" | "repeat" | "to_owned"
        => Ty::NonPrimitive,
         _=>Ty::Primitive
    }
}
// Return the return type of functions that have 
// ref receiver
fn get_ref_func_rety(func_name:&str) -> Ty {
    match func_name {
        "to_string"| "into_string" | "repeat"|"to_owned"
        => Ty::NonPrimitive,
         _=>Ty::Primitive
    }
}
// Return the return type of functions that have 
// non-primitive receiver
fn get_non_prim_func_rety(func_name:&str) -> Ty {
    match func_name{
        "split_off" => Ty::NonPrimitive,
        "as_ref"=>Ty::Ref,
        "is_empty"|"len" =>Ty::Primitive,
        _=>Ty::NonPrimitive,
    }
}

// Return the return type of functions that have 
// do not have receiver
fn get_call_rety (func_name:&str) -> Ty{
        match func_name {
        "Stringfrom"|"Stringnew"|
        "Stringwith_capacity"|"Stringfrom_utf16_lossy"|
        "Stringfrom_raw_parts"|"Stringfrom_utf8_unchecked"
         => Ty::NonPrimitive,
         "Stringeq"=>Ty::Primitive,
        _ => Ty::NonPrimitive,
         }
}
