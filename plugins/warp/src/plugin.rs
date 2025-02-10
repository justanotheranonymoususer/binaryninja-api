use crate::cache::register_cache_destructor;

use crate::matcher::MatcherSettings;
use crate::plugin::render_layer::HighlightRenderLayer;
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use binaryninja::command::{register_command, register_command_for_function};
use binaryninja::logger::Logger;
use binaryninja::rc::Ref;
use binaryninja::settings::Settings;
use binaryninja::tags::TagType;
use log::LevelFilter;

mod add;
mod copy;
mod create;
mod debug;
mod ffi;
mod find;
mod load;
mod render_layer;
mod types;
mod workflow;

const TAG_ICON: &str = "🌐";
const TAG_NAME: &str = "WARP";

fn get_warp_tag_type(view: &BinaryView) -> Ref<TagType> {
    view.tag_type_by_name(TAG_NAME)
        .unwrap_or_else(|| view.create_tag_type(TAG_NAME, TAG_ICON))
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn CorePluginInit() -> bool {
    Logger::new("WARP").with_level(LevelFilter::Debug).init();

    // Register our matcher settings globally.
    let mut global_bn_settings = Settings::new();
    MatcherSettings::register(&mut global_bn_settings);

    // Make sure caches are flushed when the views get destructed.
    register_cache_destructor();

    // Register our highlight render layer.
    HighlightRenderLayer::register();

    workflow::insert_workflow();

    register_command(
        "WARP\\Run Matcher",
        "Run the matcher manually",
        workflow::RunMatcher {},
    );

    register_command(
        "WARP\\Debug\\Cache",
        "Debug cache sizes... because...",
        debug::DebugCache {},
    );

    register_command(
        "WARP\\Debug\\Invalidate Caches",
        "Invalidate all WARP caches",
        debug::DebugInvalidateCache {},
    );

    register_command_for_function(
        "WARP\\Debug\\Function Signature",
        "Print the entire signature for the function",
        debug::DebugFunction {},
    );

    register_command_for_function(
        "WARP\\Debug\\Function Matcher",
        "Print all possible matches for the function",
        debug::DebugMatcher {},
    );

    register_command(
        "WARP\\Debug\\Apply Signature File Types",
        "Load all types from a signature file and ignore functions",
        types::LoadTypes {},
    );

    register_command(
        "WARP\\Load Signature File",
        "Load file into the matcher, this does NOT kick off matcher analysis",
        load::LoadSignatureFile {},
    );

    register_command_for_function(
        "WARP\\Copy Function GUID",
        "Copy the computed GUID for the function",
        copy::CopyFunctionGUID {},
    );

    register_command(
        "WARP\\Find Function From GUID",
        "Locate the function in the view using a GUID",
        find::FindFunctionFromGUID {},
    );

    register_command(
        "WARP\\Generate Signature File",
        "Generates a signature file containing all binary view functions",
        create::CreateSignatureFile {},
    );

    register_command_for_function(
        "WARP\\Add Function Signature to File",
        "Stores the signature for the function in the signature file",
        add::AddFunctionSignature {},
    );

    true
}
