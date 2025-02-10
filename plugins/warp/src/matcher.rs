use crate::cache::{
    cached_adjacency_constraints, cached_call_site_constraints, cached_function_match,
    try_cached_function_guid,
};
use crate::container::{Container, SourceId};
use crate::convert::to_bn_type;
use binaryninja::architecture::Architecture as BNArchitecture;
use binaryninja::binary_view::{BinaryView, BinaryViewExt};
use binaryninja::function::Function as BNFunction;
use binaryninja::function::FunctionUpdateType;
use binaryninja::settings::Settings as BNSettings;
use serde_json::json;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::hash::Hash;
use warp::r#type::class::TypeClass;
use warp::r#type::Type;
use warp::signature::function::Function;

/// A matcher represents a specific configuration for identify functions using WARP. A matcher
/// does not store/own any WARP information directly, instead the matcher is given a [Container]
/// that holds all of that information.
///
/// The separation of the WARP information from the [Matcher] allows a greater degree of control and
/// provides a clean interface for further logic to be built on top of. A matcher instance, unlike
/// a typical [Container] implementation, is cheap to create.
#[derive(Debug, Clone, Copy)]
pub struct Matcher {
    pub settings: MatcherSettings,
}

impl Matcher {
    pub fn new(settings: MatcherSettings) -> Self {
        Matcher { settings }
    }

    pub fn match_function_from_constraints<'a>(
        &self,
        function: &BNFunction,
        matched_functions: &'a [Function],
    ) -> Option<&'a Function> {
        // Filter out adjacent functions which are trivial, this helps avoid false positives.
        // NOTE: If the user sets `trivial_function_adjacent_allowed` to true we will always match.
        // TODO: Expand on this more later. We might want to match on adjacent functions smaller than this.
        let adjacent_function_filter = |adj_func: &BNFunction| {
            let adj_func_len = adj_func.highest_address() - adj_func.lowest_address();
            adj_func_len > self.settings.trivial_function_len
                || self.settings.trivial_function_adjacent_allowed
        };

        let function_len = function.highest_address() - function.lowest_address();
        let is_function_trivial = { function_len < self.settings.trivial_function_len };
        let is_function_allowed = {
            function_len > self.settings.minimum_function_len
                && function_len < self.settings.maximum_function_len.unwrap_or(u64::MAX)
        };

        // Function not allowed, or no matches, stop early.
        if !is_function_allowed || matched_functions.is_empty() {
            return None;
        }

        // If we have a single possible match than that must be our function.
        // We must also not be a trivial function, as those will likely be artifacts of an incomplete dataset
        if matched_functions.len() == 1 && !is_function_trivial {
            return matched_functions.first();
        }

        let call_sites = cached_call_site_constraints(function);
        let adjacent = cached_adjacency_constraints(function, adjacent_function_filter);

        // "common" being the intersection between the observed and matched.
        fn find_highest_common_count<'a, F, T>(
            observed_items: &HashSet<T>,
            matched_functions: &'a [Function],
            extract_items: F,
        ) -> (usize, Option<&'a Function>)
        where
            F: Fn(&Function) -> HashSet<T>,
            T: Hash + Eq,
        {
            let mut highest_count = 0;
            let mut matched_func = None;
            for matched in matched_functions {
                let matched_items = extract_items(matched);
                let common_count = observed_items.intersection(&matched_items).count();
                match common_count.cmp(&highest_count) {
                    Ordering::Equal => matched_func = None,
                    Ordering::Greater => {
                        highest_count = common_count;
                        matched_func = Some(matched);
                    }
                    Ordering::Less => {}
                }
            }
            (highest_count, matched_func)
        }

        let call_site_guids: HashSet<_> = call_sites.iter().filter_map(|c| c.guid).collect();
        let call_site_symbol_names: HashSet<_> = call_sites
            .into_iter()
            .filter_map(|c| c.symbol.map(|s| s.name))
            .collect();
        let adjacent_guids: HashSet<_> = adjacent.iter().filter_map(|c| c.guid).collect();
        let adjacent_symbol_names: HashSet<_> = adjacent
            .into_iter()
            .filter_map(|c| c.symbol.map(|s| s.name))
            .collect();

        // Ordered from the lowest confidence to the highest confidence constraint.
        let checked_constraints = [
            find_highest_common_count(&adjacent_symbol_names, matched_functions, |matched| {
                matched
                    .constraints
                    .adjacent
                    .iter()
                    .filter_map(|c| c.symbol.to_owned().map(|s| s.name))
                    .collect()
            }),
            find_highest_common_count(&adjacent_guids, matched_functions, |matched| {
                matched
                    .constraints
                    .adjacent
                    .iter()
                    .filter_map(|c| c.guid)
                    .collect()
            }),
            find_highest_common_count(&call_site_symbol_names, matched_functions, |matched| {
                matched
                    .constraints
                    .call_sites
                    .iter()
                    .filter_map(|c| c.symbol.to_owned().map(|s| s.name))
                    .collect()
            }),
            find_highest_common_count(&call_site_guids, matched_functions, |matched| {
                matched
                    .constraints
                    .call_sites
                    .iter()
                    .filter_map(|c| c.guid)
                    .collect()
            }),
        ];

        // If there is a tie, the last one wins, which should be call_site guid.
        checked_constraints
            .into_iter()
            .max_by_key(|&(count, _)| count)
            .filter(|&(count, _)| count >= self.settings.minimum_matched_constraints)
            .and_then(|(_, func)| func)
    }

    pub fn add_type_to_view<A: BNArchitecture>(
        &self,
        container: &dyn Container,
        source: &SourceId,
        view: &BinaryView,
        arch: &A,
        ty: &Type,
    ) where
        Self: Sized,
    {
        fn inner_add_type_to_view<A: BNArchitecture>(
            container: &dyn Container,
            source: &SourceId,
            view: &BinaryView,
            arch: &A,
            visited_refs: &mut HashSet<String>,
            ty: &Type,
        ) {
            // Type not already added to the view.
            // Verify all nested types are added before adding type.
            match ty.class.as_ref() {
                TypeClass::Pointer(c) => inner_add_type_to_view(
                    container,
                    source,
                    view,
                    arch,
                    visited_refs,
                    &c.child_type,
                ),
                TypeClass::Array(c) => inner_add_type_to_view(
                    container,
                    source,
                    view,
                    arch,
                    visited_refs,
                    &c.member_type,
                ),
                TypeClass::Structure(c) => {
                    for member in &c.members {
                        inner_add_type_to_view(
                            container,
                            source,
                            view,
                            arch,
                            visited_refs,
                            &member.ty,
                        )
                    }
                }
                TypeClass::Enumeration(c) => inner_add_type_to_view(
                    container,
                    source,
                    view,
                    arch,
                    visited_refs,
                    &c.member_type,
                ),
                TypeClass::Union(c) => {
                    for member in &c.members {
                        inner_add_type_to_view(
                            container,
                            source,
                            view,
                            arch,
                            visited_refs,
                            &member.ty,
                        )
                    }
                }
                TypeClass::Function(c) => {
                    for out_member in &c.out_members {
                        inner_add_type_to_view(
                            container,
                            source,
                            view,
                            arch,
                            visited_refs,
                            &out_member.ty,
                        )
                    }
                    for in_member in &c.in_members {
                        inner_add_type_to_view(
                            container,
                            source,
                            view,
                            arch,
                            visited_refs,
                            &in_member.ty,
                        )
                    }
                }
                TypeClass::Referrer(c) => {
                    // Check to see if the referrer has been added to the view.
                    let mut resolved = false;
                    if let Some(ref_guid) = c.guid {
                        // NOTE: We do not need to check for cyclic reference here because
                        // NOTE: GUID references are unable to be referenced by themselves.
                        if view.type_by_id(ref_guid.to_string()).is_none() {
                            // Add the referrer to the view if it is in the Matcher types
                            if let Some(ref_ty) = container.type_with_guid(source, &ref_guid) {
                                inner_add_type_to_view(
                                    container,
                                    source,
                                    view,
                                    arch,
                                    visited_refs,
                                    &ref_ty,
                                );
                                resolved = true;
                            }
                        }
                    }

                    if let Some(ref_name) = &c.name {
                        // Only try and resolve by name if not already visiting.
                        if !resolved
                            && visited_refs.insert(ref_name.to_string())
                            && view.type_by_name(ref_name).is_none()
                        {
                            // Add the ref to the view if it is in the Matcher types
                            let type_guids = container.type_guids_with_name(source, ref_name);
                            // TODO: What happens if we have more than one?
                            if type_guids.len() == 1 {
                                // TODO: What happens if we cant get the guid?
                                if let Some(ref_ty) =
                                    container.type_with_guid(source, &type_guids[0])
                                {
                                    inner_add_type_to_view(
                                        container,
                                        source,
                                        view,
                                        arch,
                                        visited_refs,
                                        &ref_ty,
                                    );
                                }
                            }
                            // No longer visiting type.
                            visited_refs.remove(ref_name);
                        }
                    }

                    match (c.guid, &c.name) {
                        (Some(guid), Some(name)) => {
                            view.define_auto_type_with_id(
                                name,
                                guid.to_string(),
                                &to_bn_type(arch, ty),
                            );
                        }
                        (Some(_guid), None) => {
                            // TODO: How would we reference this type without a name???
                        }
                        (None, Some(_name)) => {
                            // TODO: Cyclic type reference if no guid, so... dont define?
                        }
                        (None, None) => {
                            // TODO: What?!?!?
                        }
                    }
                }
                TypeClass::Void
                | TypeClass::Boolean(_)
                | TypeClass::Integer(_)
                | TypeClass::Character(_)
                | TypeClass::Float(_) => {}
            }
        }
        inner_add_type_to_view(container, source, view, arch, &mut HashSet::new(), ty)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MatcherSettings {
    /// Any function under this length will be required to constrain.
    ///
    /// This is set to [MatcherSettings::DEFAULT_TRIVIAL_FUNCTION_LEN] by default.
    pub trivial_function_len: u64,
    /// Any function under this length will not match.
    ///
    /// This is set to [MatcherSettings::MINIMUM_FUNCTION_LEN_DEFAULT] by default.
    pub minimum_function_len: u64,
    /// Any function above this length will not match.
    ///
    /// This is set to [MatcherSettings::MAXIMUM_FUNCTION_LEN_DEFAULT] by default.
    pub maximum_function_len: Option<u64>,
    /// For a successful constrained function match the number of matches must be above this.
    ///
    /// This is set to [MatcherSettings::DEFAULT_TRIVIAL_FUNCTION_LEN] by default.
    pub minimum_matched_constraints: usize,
    /// For a successful constrained function match the number of matches must be above this.
    ///
    /// This is set to [MatcherSettings::DEFAULT_TRIVIAL_FUNCTION_LEN] by default.
    pub trivial_function_adjacent_allowed: bool,
}

impl MatcherSettings {
    pub const TRIVIAL_FUNCTION_LEN_DEFAULT: u64 = 20;
    pub const TRIVIAL_FUNCTION_LEN_SETTING: &'static str = "analysis.warp.trivialFunctionLength";
    pub const MINIMUM_FUNCTION_LEN_DEFAULT: u64 = 0;
    pub const MINIMUM_FUNCTION_LEN_SETTING: &'static str = "analysis.warp.minimumFunctionLength";
    pub const MAXIMUM_FUNCTION_LEN_DEFAULT: u64 = 0;
    pub const MAXIMUM_FUNCTION_LEN_SETTING: &'static str = "analysis.warp.maximumFunctionLength";
    pub const MINIMUM_MATCHED_CONSTRAINTS_DEFAULT: usize = 1;
    pub const MINIMUM_MATCHED_CONSTRAINTS_SETTING: &'static str =
        "analysis.warp.minimumMatchedConstraints";
    pub const TRIVIAL_FUNCTION_ADJACENT_ALLOWED_DEFAULT: bool = false;
    pub const TRIVIAL_FUNCTION_ADJACENT_ALLOWED_SETTING: &'static str =
        "analysis.warp.trivialFunctionAdjacentAllowed";

    /// Populates the [MatcherSettings] to the current Binary Ninja settings instance.
    ///
    /// Call this once when you initialize so that the settings exist.
    ///
    /// NOTE: If you are using this as a library then just modify the MatcherSettings directly
    /// in the matcher instance, that way you don't need to round-trip through Binary Ninja.
    pub fn register(bn_settings: &mut BNSettings) {
        let trivial_function_len_props = json!({
            "title" : "Trivial Function Length",
            "type" : "number",
            "default" : Self::TRIVIAL_FUNCTION_LEN_DEFAULT,
            "description" : "Functions below this length in bytes will be required to match on constraints.",
            "ignore" : ["SettingsProjectScope", "SettingsResourceScope"]
        });
        bn_settings.register_setting_json(
            Self::TRIVIAL_FUNCTION_LEN_SETTING,
            trivial_function_len_props.to_string(),
        );

        let minimum_function_len_props = json!({
            "title" : "Minimum Function Length",
            "type" : "number",
            "default" : Self::MINIMUM_FUNCTION_LEN_DEFAULT,
            "description" : "Functions below this length will not be matched.",
            "ignore" : ["SettingsProjectScope", "SettingsResourceScope"]
        });
        bn_settings.register_setting_json(
            Self::MINIMUM_FUNCTION_LEN_SETTING,
            minimum_function_len_props.to_string(),
        );

        let maximum_function_len_props = json!({
            "title" : "Maximum Function Length",
            "type" : "number",
            "default" : Self::MAXIMUM_FUNCTION_LEN_DEFAULT,
            "description" : "Functions above this length will not be matched. A value of 0 will disable this check.",
            "ignore" : ["SettingsProjectScope", "SettingsResourceScope"]
        });
        bn_settings.register_setting_json(
            Self::MAXIMUM_FUNCTION_LEN_SETTING,
            maximum_function_len_props.to_string(),
        );

        let minimum_matched_constraints_props = json!({
            "title" : "Minimum Matched Constraints",
            "type" : "number",
            "default" : Self::MINIMUM_MATCHED_CONSTRAINTS_DEFAULT,
            "description" : "When function constraints are checked the amount of constraints matched must be at-least this.",
            "ignore" : ["SettingsProjectScope", "SettingsResourceScope"]
        });
        bn_settings.register_setting_json(
            Self::MINIMUM_MATCHED_CONSTRAINTS_SETTING,
            minimum_matched_constraints_props.to_string(),
        );

        let trivial_function_adjacent_allowed_props = json!({
            "title" : "Trivial Function Adjacent Constraints Allowed",
            "type" : "boolean",
            "default" : Self::TRIVIAL_FUNCTION_ADJACENT_ALLOWED_DEFAULT,
            "description" : "When function constraints are checked if this is enabled functions can match based off trivial adjacent functions.",
            "ignore" : ["SettingsProjectScope", "SettingsResourceScope"]
        });
        bn_settings.register_setting_json(
            Self::TRIVIAL_FUNCTION_ADJACENT_ALLOWED_SETTING,
            trivial_function_adjacent_allowed_props.to_string(),
        );
    }

    /// Retrieve matcher settings from [`BNSettings`].
    pub fn from_settings(bn_settings: &BNSettings) -> Self {
        let mut settings = MatcherSettings::default();
        if bn_settings.contains(Self::TRIVIAL_FUNCTION_LEN_SETTING) {
            settings.trivial_function_len =
                bn_settings.get_integer(Self::TRIVIAL_FUNCTION_LEN_SETTING);
        }
        if bn_settings.contains(Self::MINIMUM_FUNCTION_LEN_SETTING) {
            settings.minimum_function_len =
                bn_settings.get_integer(Self::MINIMUM_FUNCTION_LEN_SETTING);
        }
        if bn_settings.contains(Self::MAXIMUM_FUNCTION_LEN_SETTING) {
            match bn_settings.get_integer(Self::MAXIMUM_FUNCTION_LEN_SETTING) {
                0 => settings.maximum_function_len = None,
                len => settings.maximum_function_len = Some(len),
            }
        }
        if bn_settings.contains(Self::MINIMUM_MATCHED_CONSTRAINTS_SETTING) {
            settings.minimum_matched_constraints =
                bn_settings.get_integer(Self::MINIMUM_MATCHED_CONSTRAINTS_SETTING) as usize;
        }
        if bn_settings.contains(Self::TRIVIAL_FUNCTION_ADJACENT_ALLOWED_SETTING) {
            settings.trivial_function_adjacent_allowed =
                bn_settings.get_bool(Self::TRIVIAL_FUNCTION_ADJACENT_ALLOWED_SETTING);
        }
        settings
    }
}

impl Default for MatcherSettings {
    fn default() -> Self {
        Self {
            trivial_function_len: MatcherSettings::TRIVIAL_FUNCTION_LEN_DEFAULT,
            minimum_function_len: MatcherSettings::MINIMUM_FUNCTION_LEN_DEFAULT,
            maximum_function_len: None,
            minimum_matched_constraints: MatcherSettings::MINIMUM_MATCHED_CONSTRAINTS_DEFAULT,
            trivial_function_adjacent_allowed:
                MatcherSettings::TRIVIAL_FUNCTION_ADJACENT_ALLOWED_DEFAULT,
        }
    }
}
