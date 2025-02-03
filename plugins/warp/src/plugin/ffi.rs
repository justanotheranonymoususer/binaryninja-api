use crate::cache::{cached_function_guid, insert_cached_function_match, try_cached_function_match};
use crate::convert::{to_bn_symbol_at_address, to_bn_type};
use crate::matcher::cached_possible_function_matches;
use crate::{basic_block_guid, relocatable_regions};
use binaryninja::basic_block::BasicBlock;
use binaryninja::function::{Function, NativeBlock};
use binaryninja::platform::Platform;
use binaryninja::rc::Ref;
use binaryninja::string::BnString;
use binaryninjacore_sys::{BNBasicBlock, BNFunction, BNPlatform, BNSymbol, BNType};
use std::ffi::{c_char, CStr};
use std::str::FromStr;
use warp::signature::function::FunctionGUID;

pub type BNWARPFunction = warp::signature::function::Function;

// TODO: Function Constraints
// TODO: Some sort of callback for loading functions
// TODO: Insert/remove matches
// TODO: Network callbacks?
// TODO: Network specific stuff?
// TODO: Add file to matcher cache
// TODO: Be able to run matcher for a specific file
// TODO: Generate signatures for a file, return what?
// TODO: Generate a basic block guid
// TODO: Is instruction maskable?

#[no_mangle]
pub extern "C" fn BNWARPGetBasicBlockGUID(basic_block: *mut BNBasicBlock) -> *const c_char {
    let basic_block = unsafe { BasicBlock::from_raw(basic_block, NativeBlock::new()) };
    let function = basic_block.function();
    let Ok(llil) = function.low_level_il() else {
        return std::ptr::null();
    };
    // TODO: This should be the callers responsibility IMO to get relocatable ranges.
    let relocatable_regions = relocatable_regions(&function.view());
    let basic_block_guid = basic_block_guid(&relocatable_regions, &basic_block, &llil);
    let basic_block_guid_str = BnString::new(basic_block_guid.to_string());
    // NOTE: Leak the guid string to be freed by BNFreeString
    BnString::into_raw(basic_block_guid_str)
}

#[no_mangle]
pub extern "C" fn BNWARPGetFunctionGUID(analysis_function: *mut BNFunction) -> *const c_char {
    let function = unsafe { Function::from_raw(analysis_function) };
    let Ok(llil) = function.low_level_il() else {
        return std::ptr::null();
    };
    let function_guid = cached_function_guid(&function, &llil);
    let function_guid_str = BnString::new(function_guid.to_string());
    // NOTE: Leak the guid string to be freed by BNFreeString
    BnString::into_raw(function_guid_str)
}

#[no_mangle]
pub extern "C" fn BNWARPGetMatchedFunction(
    analysis_function: *mut BNFunction,
) -> *mut BNWARPFunction {
    let function = unsafe { Function::from_raw(analysis_function) };
    match try_cached_function_match(&function) {
        Some(matched_function) => {
            let boxed_matched_function = Box::new(matched_function);
            // NOTE: Leak the matched function to be freed by BNWARPFreeFunction
            Box::into_raw(boxed_matched_function)
        }
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn BNWARPSetMatchedFunction(
    analysis_function: *mut BNFunction,
    function: *mut BNWARPFunction,
) -> *mut BNWARPFunction {
    let analysis_function = unsafe { Function::from_raw(analysis_function) };
    let previous_match = match function.is_null() {
        false => {
            // Set the matched function to `function` and return previous.
            let matched_function = unsafe { Box::from_raw(function) };
            let previous = insert_cached_function_match(
                &analysis_function,
                Some(matched_function.as_ref().clone()),
            );
            // We do not own matched_function so we should not drop.
            std::mem::forget(matched_function);
            previous
        }
        true => {
            // We are removing the previous match and returning it.
            insert_cached_function_match(&analysis_function, None)
        }
    };

    // Return the previous match, if any.
    match previous_match {
        Some(previous) => {
            let boxed_prev_matched_function = Box::new(previous);
            // NOTE: Leak the previous matched function to be freed by BNWARPFreeFunction
            Box::into_raw(boxed_prev_matched_function)
        }
        None => std::ptr::null_mut(),
    }
}

// TODO: This is not what we want.
// TODO: We need a way to register a warp provider.
// TODO: It needs platform and guid only right?
#[no_mangle]
pub extern "C" fn BNWARPGetPossibleFunctions(
    platform: *mut BNPlatform,
    guid: *mut c_char,
    count: *mut usize,
) -> *mut *mut BNWARPFunction {
    let platform = unsafe { Platform::from_raw(platform) };
    let guid_cstr = unsafe { CStr::from_ptr(guid) };
    let guid_str = guid_cstr.to_str().unwrap();
    let Ok(guid) = FunctionGUID::from_str(guid_str) else {
        return std::ptr::null_mut();
    };
    let possible_matches = cached_possible_function_matches(&platform, &guid);

    // SAFETY: This is safe, count is an out param expected to be written to.
    unsafe { *count = possible_matches.len() };
    let boxed_possible_matches = possible_matches
        .into_iter()
        .map(|f| {
            let boxed_function = Box::new(f);
            // NOTE: Leak the function to be freed by BNWARPFreeFunctionList
            Box::into_raw(boxed_function)
        })
        .collect();
    // NOTE: Leak the list to be freed by BNWARPFreeFunctionList
    let possible_matches_ptr = Box::into_raw(boxed_possible_matches);
    // SAFETY: This is safe as *mut Box<Function> is equiv to *mut *mut BNWARPFunction
    possible_matches_ptr as *mut *mut BNWARPFunction
}

#[no_mangle]
pub extern "C" fn BNWARPGetFunctionSymbol(
    analysis_function: *mut BNFunction,
    function: *mut BNWARPFunction,
) -> *mut BNSymbol {
    let analysis_function = unsafe { Function::from_raw(analysis_function) };
    let function = unsafe { Box::from_raw(function) };
    let view = analysis_function.view();
    let address = analysis_function.symbol().address();
    let function_symbol = to_bn_symbol_at_address(&view, &function.symbol, address);
    // We do not own function so we should not drop.
    std::mem::forget(function);
    // NOTE: The symbol ref has been pre-incremented for the caller.
    unsafe { Ref::into_raw(function_symbol) }.handle
}

#[no_mangle]
pub extern "C" fn BNWARPGetFunctionType(
    analysis_function: *mut BNFunction,
    function: *mut BNWARPFunction,
) -> *mut BNType {
    let analysis_function = unsafe { Function::from_raw(analysis_function) };
    let function = unsafe { Box::from_raw(function) };
    let arch = analysis_function.arch();
    let function_type = to_bn_type(&arch, &function.ty);
    // We do not own function so we should not drop.
    std::mem::forget(function);
    // NOTE: The type ref has been pre-incremented for the caller.
    unsafe { Ref::into_raw(function_type) }.handle
}

#[no_mangle]
pub extern "C" fn BNWARPFreeFunction(function: *mut BNWARPFunction) {
    // NOTE: Free the function leaked by BNWARPGetPossibleFunctions
    let _ = unsafe { Box::from_raw(function) };
}

#[no_mangle]
pub extern "C" fn BNWARPFreeFunctionList(functions: *mut *mut BNWARPFunction, count: usize) {
    // NOTE: Free the function leaked by BNWARPGetMatchedFunction
    let functions_ptr = std::ptr::slice_from_raw_parts_mut(functions, count);
    let functions = unsafe { Box::from_raw(functions_ptr) };
    for function in functions {
        // NOTE: The functions themselves should also be boxed.
        let _ = unsafe { Box::from_raw(function) };
    }
}
