use crate::build_function;
use crate::cache::{FunctionID, ViewID};
use binaryninja::architecture::Architecture;
use binaryninja::binary_view::BinaryView;
use binaryninja::function::Function as BNFunction;
use binaryninja::low_level_il::RegularLowLevelILFunction;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use std::sync::OnceLock;
use warp::signature::function::Function;

// TODO: We also need a function cache to hold the matches that we did not get for a function
// TODO: this is so the UI can show all the possible matches for a function. That or we run that
// TODO: every time that we navigate to another function.
// TODO: Keeping a copy of all possible matches is a _lot_
pub static MATCHED_FUNCTION_CACHE: OnceLock<DashMap<ViewID, MatchedFunctionCache>> =
    OnceLock::new();

// TODO: Do we even need this? This is i think needed for generation.
pub static FUNCTION_CACHE: OnceLock<DashMap<ViewID, FunctionCache>> = OnceLock::new();

pub fn clear_function_cache(view: &BinaryView) {
    let view_id = ViewID::from(view);
    if let Some(cache) = MATCHED_FUNCTION_CACHE.get() {
        cache.remove(&view_id);
    }
    if let Some(cache) = FUNCTION_CACHE.get() {
        cache.remove(&view_id);
    }
}

pub fn cached_function_match<F>(function: &BNFunction, f: F) -> Option<Function>
where
    F: Fn() -> Option<Function>,
{
    let view = function.view();
    let view_id = ViewID::from(view.as_ref());
    let function_id = FunctionID::from(function);
    let function_cache = MATCHED_FUNCTION_CACHE.get_or_init(Default::default);
    match function_cache.get(&view_id) {
        Some(cache) => cache.get_or_insert(&function_id, f).to_owned(),
        None => {
            let cache = MatchedFunctionCache::default();
            let matched = cache.get_or_insert(&function_id, f).to_owned();
            function_cache.insert(view_id, cache);
            matched
        }
    }
}

pub fn insert_cached_function_match(
    function: &BNFunction,
    matched_function: Option<Function>,
) -> Option<Function> {
    let view = function.view();
    let view_id = ViewID::from(view);
    let function_id = FunctionID::from(function);
    let function_cache = MATCHED_FUNCTION_CACHE.get_or_init(Default::default);
    function_cache
        .get(&view_id)?
        .insert(function_id, matched_function)
}

pub fn try_cached_function_match(function: &BNFunction) -> Option<Function> {
    let view = function.view();
    let view_id = ViewID::from(view);
    let function_id = FunctionID::from(function);
    let function_cache = MATCHED_FUNCTION_CACHE.get_or_init(Default::default);
    function_cache
        .get(&view_id)?
        .get(&function_id)?
        .value()
        .to_owned()
}

pub fn cached_function<A: Architecture>(
    function: &BNFunction,
    llil: &RegularLowLevelILFunction<A>,
) -> Function {
    let view = function.view();
    let view_id = ViewID::from(view.as_ref());
    let function_cache = FUNCTION_CACHE.get_or_init(Default::default);
    match function_cache.get(&view_id) {
        Some(cache) => cache.function(function, llil),
        None => {
            let cache = FunctionCache::default();
            let function = cache.function(function, llil);
            function_cache.insert(view_id, cache);
            function
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MatchedFunctionCache {
    pub cache: DashMap<FunctionID, Option<Function>>,
}

impl MatchedFunctionCache {
    pub fn insert(&self, function_id: FunctionID, function: Option<Function>) -> Option<Function> {
        self.cache.insert(function_id, function)?
    }

    pub fn get_or_insert<F>(
        &self,
        function_id: &FunctionID,
        f: F,
    ) -> Ref<'_, FunctionID, Option<Function>>
    where
        F: FnOnce() -> Option<Function>,
    {
        self.cache.get(function_id).unwrap_or_else(|| {
            self.cache.insert(*function_id, f());
            self.cache.get(function_id).unwrap()
        })
    }

    pub fn get(&self, function_id: &FunctionID) -> Option<Ref<'_, FunctionID, Option<Function>>> {
        self.cache.get(function_id)
    }
}

#[derive(Clone, Debug, Default)]
pub struct FunctionCache {
    pub cache: DashMap<FunctionID, Function>,
}

impl FunctionCache {
    pub fn function<A: Architecture>(
        &self,
        function: &BNFunction,
        llil: &RegularLowLevelILFunction<A>,
    ) -> Function {
        let function_id = FunctionID::from(function);
        match self.cache.get(&function_id) {
            Some(function) => function.value().to_owned(),
            None => {
                let function = build_function(function, llil);
                self.cache.insert(function_id, function.clone());
                function
            }
        }
    }
}
