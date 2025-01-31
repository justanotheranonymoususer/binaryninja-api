use crate::cache::{FunctionID, ViewID};
use crate::convert::from_bn_symbol;
use crate::function_guid;
use binaryninja::architecture::Architecture;
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use binaryninja::function::Function as BNFunction;
use binaryninja::low_level_il::function::{
    FunctionMutability, LowLevelILFunction, NonSSA, RegularNonSSA,
};
use binaryninja::symbol::Symbol as BNSymbol;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::OnceLock;
use warp::signature::function::constraints::FunctionConstraint;
use warp::signature::function::FunctionGUID;

pub static GUID_CACHE: OnceLock<DashMap<ViewID, GUIDCache>> = OnceLock::new();

pub fn clear_guid_cache(view: &BinaryView) {
    let view_id = ViewID::from(view);
    if let Some(cache) = GUID_CACHE.get() {
        cache.remove(&view_id);
    }
}

pub fn cached_function_guid<A: Architecture, M: FunctionMutability>(
    function: &BNFunction,
    llil: &LowLevelILFunction<A, M, NonSSA<RegularNonSSA>>,
) -> FunctionGUID {
    let view = function.view();
    let view_id = ViewID::from(view);
    let guid_cache = GUID_CACHE.get_or_init(Default::default);
    match guid_cache.get(&view_id) {
        Some(cache) => cache.function_guid(function, llil),
        None => {
            let cache = GUIDCache::default();
            let guid = cache.function_guid(function, llil);
            guid_cache.insert(view_id, cache);
            guid
        }
    }
}

pub fn try_cached_function_guid(function: &BNFunction) -> Option<FunctionGUID> {
    let view = function.view();
    let view_id = ViewID::from(view);
    let guid_cache = GUID_CACHE.get_or_init(Default::default);
    guid_cache.get(&view_id)?.try_function_guid(function)
}

pub fn cached_call_site_constraints(function: &BNFunction) -> HashSet<FunctionConstraint> {
    let view = function.view();
    let view_id = ViewID::from(view);
    let guid_cache = GUID_CACHE.get_or_init(Default::default);
    match guid_cache.get(&view_id) {
        Some(cache) => cache.call_site_constraints(function),
        None => {
            let cache = GUIDCache::default();
            let constraints = cache.call_site_constraints(function);
            guid_cache.insert(view_id, cache);
            constraints
        }
    }
}

pub fn cached_adjacency_constraints<F>(
    function: &BNFunction,
    filter: F,
) -> HashSet<FunctionConstraint>
where
    F: Fn(&BNFunction) -> bool,
{
    let view = function.view();
    let view_id = ViewID::from(view);
    let guid_cache = GUID_CACHE.get_or_init(Default::default);
    match guid_cache.get(&view_id) {
        Some(cache) => cache.adjacency_constraints(function, filter),
        None => {
            let cache = GUIDCache::default();
            let constraints = cache.adjacency_constraints(function, filter);
            guid_cache.insert(view_id, cache);
            constraints
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GUIDCache {
    pub cache: DashMap<FunctionID, FunctionGUID>,
}

impl GUIDCache {
    pub fn call_site_constraints(&self, function: &BNFunction) -> HashSet<FunctionConstraint> {
        let view = function.view();
        let func_id = FunctionID::from(function);
        let func_start = function.start();
        let func_platform = function.platform();
        let mut constraints = HashSet::new();
        for call_site in &function.call_sites() {
            for cs_ref_addr in view.code_refs_from_addr(call_site.address, Some(function)) {
                match view.function_at(&func_platform, cs_ref_addr) {
                    Some(cs_ref_func) => {
                        // Call site is a function, constrain on it.
                        let cs_ref_func_id = FunctionID::from(cs_ref_func.as_ref());
                        if cs_ref_func_id != func_id {
                            let call_site_offset: i64 =
                                call_site.address.wrapping_sub(func_start) as i64;
                            // TODO: If the function is thunk we should also insert the called function.
                            constraints
                                .insert(self.function_constraint(&cs_ref_func, call_site_offset));
                        }
                    }
                    None => {
                        // We could be dealing with an extern symbol, get the symbol as a constraint.
                        let call_site_offset: i64 =
                            call_site.address.wrapping_sub(func_start) as i64;
                        if let Some(call_site_sym) = view.symbol_by_address(cs_ref_addr) {
                            constraints.insert(
                                self.function_constraint_from_symbol(
                                    &call_site_sym,
                                    call_site_offset,
                                ),
                            );
                        }
                    }
                }
            }
        }
        constraints
    }

    pub fn adjacency_constraints<F>(
        &self,
        function: &BNFunction,
        filter: F,
    ) -> HashSet<FunctionConstraint>
    where
        F: Fn(&BNFunction) -> bool,
    {
        let view = function.view();
        let func_id = FunctionID::from(function);
        let func_start = function.start();
        let mut constraints = HashSet::new();

        let mut func_addr_constraint = |func_start_addr| {
            // NOTE: We could potentially have dozens of functions all at the same start address.
            for curr_func in &view.functions_at(func_start_addr) {
                let curr_func_id = FunctionID::from(curr_func.as_ref());
                if curr_func_id != func_id && filter(curr_func.as_ref()) {
                    // NOTE: For this to work the GUID has to have already been cached. If not it will just be the symbol.
                    // Function adjacent to another function, constrain on the pattern.
                    let curr_addr_offset = (func_start_addr as i64) - func_start as i64;
                    constraints.insert(self.function_constraint(&curr_func, curr_addr_offset));
                }
            }
        };

        let mut before_func_start = func_start;
        for _ in 0..2 {
            before_func_start = view.function_start_before(before_func_start);
            func_addr_constraint(before_func_start);
        }

        let mut after_func_start = func_start;
        for _ in 0..2 {
            after_func_start = view.function_start_after(after_func_start);
            func_addr_constraint(after_func_start);
        }

        constraints
    }

    /// Construct a function constraint, must pass the offset at which it is located.
    pub fn function_constraint(&self, function: &BNFunction, offset: i64) -> FunctionConstraint {
        let guid = self.try_function_guid(function);
        let symbol = from_bn_symbol(&function.symbol());
        FunctionConstraint {
            guid,
            symbol: Some(symbol),
            offset,
        }
    }

    /// Construct a function constraint from a symbol, typically used for extern function call sites, must pass the offset at which it is located.
    pub fn function_constraint_from_symbol(
        &self,
        symbol: &BNSymbol,
        offset: i64,
    ) -> FunctionConstraint {
        let symbol = from_bn_symbol(symbol);
        FunctionConstraint {
            guid: None,
            symbol: Some(symbol),
            offset,
        }
    }

    pub fn function_guid<A: Architecture, M: FunctionMutability>(
        &self,
        function: &BNFunction,
        llil: &LowLevelILFunction<A, M, NonSSA<RegularNonSSA>>,
    ) -> FunctionGUID {
        let function_id = FunctionID::from(function);
        match self.cache.get(&function_id) {
            Some(function_guid) => function_guid.value().to_owned(),
            None => {
                let function_guid = function_guid(function, llil);
                self.cache.insert(function_id, function_guid);
                function_guid
            }
        }
    }

    pub fn try_function_guid(&self, function: &BNFunction) -> Option<FunctionGUID> {
        let function_id = FunctionID::from(function);
        self.cache
            .get(&function_id)
            .map(|function_guid| function_guid.value().to_owned())
    }
}
