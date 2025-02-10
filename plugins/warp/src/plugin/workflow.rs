use crate::cache::container::{add_cached_container, cached_containers};
use crate::cache::{cached_function_guid, try_cached_function_guid, try_cached_function_match};
use crate::container::disk::DiskContainer;
use crate::container::SourceId;
use crate::convert::{to_bn_symbol_at_address, to_bn_type};
use crate::matcher::{Matcher, MatcherSettings};
use crate::plugin::get_warp_tag_type;
use crate::{core_signature_dir, user_signature_dir};
use binaryninja::background_task::BackgroundTask;
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use binaryninja::command::Command;
use binaryninja::function::{Function as BNFunction, FunctionUpdateType};
use binaryninja::low_level_il::function::RegularNonSSA;
use binaryninja::rc::Ref;
use binaryninja::settings::Settings;
use binaryninja::workflow::{Activity, AnalysisContext, Workflow};
use std::collections::HashMap;
use std::time::Instant;
use warp::signature::function::Function;

pub const APPLY_ACTIVITY_NAME: &str = "analysis.warp.apply";
const APPLY_ACTIVITY_CONFIG: &str = r#"{
    "name": "analysis.warp.apply",
    "title" : "WARP Applier",
    "description": "This analysis step applies WARP info to matched functions...",
    "eligibility": {
        "auto": {},
        "runOnce": false
    }
}"#;

pub const MATCHER_ACTIVITY_NAME: &str = "analysis.warp.matcher";
const MATCHER_ACTIVITY_CONFIG: &str = r#"{
    "name": "analysis.warp.matcher",
    "title" : "WARP Matcher",
    "description": "This analysis step attempts to find matching WARP functions..",
    "eligibility": {
        "auto": {},
        "runOnce": true
    }
}"#;

// TODO: This should run every time a function is changed no?
// TODO: Ah whatever this is fine for now...
pub const GUID_ACTIVITY_NAME: &str = "analysis.warp.guid";
const GUID_ACTIVITY_CONFIG: &str = r#"{
    "name": "analysis.warp.guid",
    "title" : "WARP GUID Generator",
    "description": "This analysis step generates the GUID for all analyzed functions...",
    "eligibility": {
        "auto": {},
        "runOnce": true
    }
}"#;

pub struct RunMatcher;

impl Command for RunMatcher {
    fn action(&self, view: &BinaryView) {
        let view = view.to_owned();
        // TODO: Check to see if the GUID cache is empty and ask the user if they want to regenerate the guids.
        std::thread::spawn(move || {
            let undo_id = view.file().begin_undo_actions(true);
            let background_task = BackgroundTask::new("Matching on functions...", false);
            let start = Instant::now();
            // view.functions()
            //     .iter()
            //     .for_each(|function| cached_function_matcher(&function));
            log::info!("Function matching took {:?}", start.elapsed());
            background_task.finish();
            view.file().commit_undo_actions(undo_id);
            // Now we want to trigger re-analysis.
            view.update_analysis();
        });
    }

    fn valid(&self, _view: &BinaryView) -> bool {
        true
    }
}

pub fn insert_workflow() {
    // "Hey look, it's a plier" ~ Josh 2025
    let apply_activity = |ctx: &AnalysisContext| {
        // There needs to be a way to "hold" running this activity for a function
        let view = ctx.view();
        let function = ctx.function();
        if let Some(matched_function) = try_cached_function_match(&function) {
            view.define_auto_symbol(&to_bn_symbol_at_address(
                &view,
                &matched_function.symbol,
                function.symbol().address(),
            ));
            function.set_auto_type(&to_bn_type(&function.arch(), &matched_function.ty));
            // TODO: Add metadata. (both binja metadata and warp metadata)
            function.add_tag(
                &get_warp_tag_type(&view),
                matched_function.guid.to_string(),
                None,
                false,
                None,
            );
        }
    };

    let matcher_activity = |ctx: &AnalysisContext| {
        let view = ctx.view();
        let platform_name = view
            .default_platform()
            .map(|p| p.name().to_string())
            .unwrap_or_default();

        // TODO: Once the spec has functions with a platform, we will load all on-disk files in the core directory
        // First we want to load all the directories into the container cache.
        let background_task = BackgroundTask::new("Loading WARP files...", false);
        let start = Instant::now();
        let core_plat_dir = core_signature_dir().join(&platform_name);
        let core_disk_container = DiskContainer::new_from_dir(core_plat_dir);
        log::debug!("{:#?}", core_disk_container);
        add_cached_container(&view, core_disk_container);
        let user_plat_dir = user_signature_dir().join(platform_name);
        let user_disk_container = DiskContainer::new_from_dir(user_plat_dir);
        log::debug!("{:#?}", user_disk_container);
        add_cached_container(&view, user_disk_container);
        log::info!("Loading files took {:?}", start.elapsed());
        background_task.finish();

        // Then we want to actually find matching functions.
        let background_task = BackgroundTask::new("Matching on WARP functions...", false);
        let start = Instant::now();
        let functions_guids: HashMap<_, Vec<_>> = view
            .functions()
            .iter()
            .filter_map(|f| try_cached_function_guid(&f).map(|guid| (guid, f.to_owned())))
            .fold(HashMap::new(), |mut acc, (guid, function)| {
                acc.entry(guid).or_default().push(function);
                acc
            });
        let guids = functions_guids.keys().copied().collect::<Vec<_>>();

        // Build matcher
        let view_settings = Settings::new();
        let matcher_settings = MatcherSettings::from_settings(&view_settings);
        let matcher = Matcher::new(matcher_settings);

        // TODO: Containers might both match on the same function. What should we do?
        // TODO: Methinks we should prioritize certain containers.
        // TODO: If we can make containers store matched functions than we can have two-level matching.
        cached_containers(&view, |container| {
            // We need to call sources_with_function_guids
            // all sources with a guid need to be collected.
            for (guid, sources) in container.sources_with_function_guids(&guids) {
                let matched_functions: HashMap<&SourceId, Vec<Function>> = sources
                    .iter()
                    .map(|&s| (s, container.functions_with_guid(s, guid)))
                    .collect();
                // TODO: Cloning the list here is wasteful.
                let matched_function_list: Vec<_> =
                    matched_functions.values().cloned().flatten().collect();

                let functions = functions_guids.get(&guid).expect("Function guid not found");
                for function in functions {
                    // Match on all the possible functions
                    match matcher.match_function_from_constraints(function, &matched_function_list)
                    {
                        Some(matched_function) => {
                            // We were able to find a match, add it to the match cache and then mark the function
                            // as requiring updates, this is so that we know about it in the applier activity.
                            // TODO: Ok so the applier needs to know the function and the source id it came from.

                            // NOTE: If we expect to run match_function multiple times on a function we should move this elsewhere.
                            function
                                .mark_updates_required(FunctionUpdateType::FullAutoFunctionUpdate);
                        }
                        None => {}
                    }
                }
            }
        });
        log::info!("Function matching took {:?}", start.elapsed());
        background_task.finish();

        // Now we want to trigger re-analysis.
        view.update_analysis();
    };

    let guid_activity = |ctx: &AnalysisContext| {
        let function = ctx.function();
        if let Some(llil) = unsafe { ctx.llil_function::<RegularNonSSA>() } {
            cached_function_guid(&function, &llil);
        }
    };

    let old_function_meta_workflow = Workflow::instance("core.function.metaAnalysis");
    let function_meta_workflow = old_function_meta_workflow.clone("core.function.metaAnalysis");
    let guid_activity = Activity::new_with_action(GUID_ACTIVITY_CONFIG, guid_activity);
    let apply_activity = Activity::new_with_action(APPLY_ACTIVITY_CONFIG, apply_activity);
    function_meta_workflow
        .register_activity(&guid_activity)
        .unwrap();
    function_meta_workflow
        .register_activity(&apply_activity)
        .unwrap();
    function_meta_workflow.insert("core.function.runFunctionRecognizers", [GUID_ACTIVITY_NAME]);
    function_meta_workflow.insert("core.function.generateMediumLevelIL", [APPLY_ACTIVITY_NAME]);
    function_meta_workflow.register().unwrap();

    let old_module_meta_workflow = Workflow::instance("core.module.metaAnalysis");
    let module_meta_workflow = old_module_meta_workflow.clone("core.module.metaAnalysis");
    let matcher_activity = Activity::new_with_action(MATCHER_ACTIVITY_CONFIG, matcher_activity);
    module_meta_workflow
        .register_activity(&matcher_activity)
        .unwrap();
    module_meta_workflow.insert(
        "core.module.deleteUnusedAutoFunctions",
        [MATCHER_ACTIVITY_NAME],
    );
    module_meta_workflow.register().unwrap();
}
