use crate::cache::{cached_function_guid, try_cached_function_match};
use crate::convert::{to_bn_symbol_at_address, to_bn_type};
use crate::matcher::cached_function_matcher;
use crate::plugin::get_warp_tag_type;
use binaryninja::background_task::BackgroundTask;
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use binaryninja::command::Command;
use binaryninja::low_level_il::function::RegularNonSSA;
use binaryninja::workflow::{Activity, AnalysisContext, Workflow};
use std::time::Instant;

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
            view.functions()
                .iter()
                .for_each(|function| cached_function_matcher(&function));
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
        let background_task = BackgroundTask::new("Matching on functions...", false);
        let start = Instant::now();
        view.functions()
            .iter()
            .for_each(|function| cached_function_matcher(&function));
        log::info!("Function matching took {:?}", start.elapsed());
        background_task.finish();
        // Now we want to trigger re-analysis.
        view.update_analysis();
    };

    let guid_activity = |ctx: &AnalysisContext| {
        let function = ctx.function();
        // TODO: Returning RegularNonSSA means we cant modify the il (the lifting code was written just for lifted il, that needs to be fixed)
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
