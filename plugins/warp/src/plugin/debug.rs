use crate::cache::{ViewID, FUNCTION_CACHE, GUID_CACHE, MATCHED_FUNCTION_CACHE};
use crate::matcher::{invalidate_function_matcher_cache, Matcher, PlatformID, PLAT_MATCHER_CACHE};
use crate::{build_function, cache};
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use binaryninja::command::{Command, FunctionCommand};
use binaryninja::function::Function;
use binaryninja::ObjectDestructor;
use warp::signature::function::constraints::FunctionConstraint;

pub struct DebugFunction;

impl FunctionCommand for DebugFunction {
    fn action(&self, _view: &BinaryView, func: &Function) {
        if let Ok(llil) = func.low_level_il() {
            log::info!("{:#?}", build_function(func, &llil));
        }
    }

    fn valid(&self, _view: &BinaryView, _func: &Function) -> bool {
        true
    }
}

pub struct DebugMatcher;

impl FunctionCommand for DebugMatcher {
    fn action(&self, _view: &BinaryView, function: &Function) {
        let Ok(llil) = function.low_level_il() else {
            log::error!("No LLIL for function 0x{:x}", function.start());
            return;
        };
        let platform = function.platform();
        // Build the matcher every time this is called to make sure we aren't in a bad state.
        let matcher = Matcher::from_platform(platform);
        let func = build_function(function, &llil);
        // TODO: Clean this up.
        if let Some(possible_matches) = matcher.functions.get(&func.guid) {
            let print_constraint = |prefix: &str, constraint: &FunctionConstraint| {
                log::info!(
                    "    {} {} ({})",
                    prefix,
                    constraint
                        .to_owned()
                        .symbol
                        .map(|s| s.name)
                        .unwrap_or("*".to_string()),
                    constraint
                        .guid
                        .map(|g| g.to_string())
                        .unwrap_or("*".to_string())
                );
            };
            log::info!("POSSIBLE MATCHES FOR 0x{:x}", function.start());
            for possible_match in possible_matches.value() {
                log::info!("{} ({})", possible_match.symbol.name, possible_match.guid);
                for constraint in &possible_match.constraints.call_sites {
                    print_constraint("CS ", constraint);
                }
                for constraint in &possible_match.constraints.adjacent {
                    print_constraint("ADJ", constraint);
                }
            }
            let matched_function =
                matcher.match_function_from_constraints(&function, possible_matches.value());
            if let Some(matched_function) = matched_function {
                log::info!(
                    "MATCHED FUNCTION '{}' FOR 0x{:x}",
                    matched_function.symbol.name,
                    function.start(),
                );
            } else {
                log::error!("NO MATCHED FUNCTION FOR 0x{:x}", function.start());
            }
        } else {
            log::error!(
                "No possible matches found for the function 0x{:x}",
                function.start()
            );
        };
    }

    fn valid(&self, _view: &BinaryView, _function: &Function) -> bool {
        true
    }
}

pub struct DebugCache;

impl Command for DebugCache {
    fn action(&self, view: &BinaryView) {
        let view_id = ViewID::from(view);
        let function_cache = FUNCTION_CACHE.get_or_init(Default::default);
        if let Some(cache) = function_cache.get(&view_id) {
            log::info!("View functions: {}", cache.cache.len());
        }

        let matched_function_cache = MATCHED_FUNCTION_CACHE.get_or_init(Default::default);
        if let Some(cache) = matched_function_cache.get(&view_id) {
            log::info!("View matched functions: {}", cache.cache.len());
        }

        let function_guid_cache = GUID_CACHE.get_or_init(Default::default);
        if let Some(cache) = function_guid_cache.get(&view_id) {
            log::info!("View function guids: {}", cache.cache.len());
        }

        let plat_cache = PLAT_MATCHER_CACHE.get_or_init(Default::default);
        if let Some(plat) = view.default_platform() {
            let platform_id = PlatformID::from(plat);
            if let Some(cache) = plat_cache.get(&platform_id) {
                log::info!("Platform functions: {}", cache.functions.len());
                log::info!("Platform types: {}", cache.types.len());
                log::info!("Platform settings: {:?}", cache.settings);
            }
        }
    }

    fn valid(&self, _view: &BinaryView) -> bool {
        true
    }
}

pub struct DebugInvalidateCache;

impl Command for DebugInvalidateCache {
    fn action(&self, view: &BinaryView) {
        invalidate_function_matcher_cache();
        let destructor = cache::CacheDestructor {};
        destructor.destruct_view(view);
        log::info!("Invalidated all WARP caches...");
    }

    fn valid(&self, _view: &BinaryView) -> bool {
        true
    }
}
