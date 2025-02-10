use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use uuid::Uuid;
use warp::r#type::guid::TypeGUID;
use warp::r#type::Type;
use warp::signature::function::{Function, FunctionGUID};

pub mod disk;
pub mod memory;
pub mod network;

/// Represents the ID for a single container source.
///
/// A source is used to relate types and functions separate from the container. This allows
/// type name lookups and for containers which are bandwidth sensitive to exist.
///
/// An example of a bandwidth sensitive container would be a container which pulls functions over
/// the network instead of from memory or disk.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy)]
pub struct SourceId(Uuid);

impl SourceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Storage for WARP functions.
///
/// Containers are made up of sources, see [`SourceId`] for more details.
pub trait Container: Send + Sync + Display + Debug {
    /// Get the sources that contain a type with the given [`TypeGUID`].
    fn sources_with_type_guid(&self, guid: &TypeGUID) -> Vec<&SourceId>;

    /// Plural version of [`Container::sources_with_type_guid`].
    ///
    /// Each source will have a list of the containing GUID's so that when looking up a source you give
    /// it only the GUID's that it knows about, for networking this means cutting down traffic significantly.
    fn sources_with_type_guids<'a>(
        &'a self,
        guids: &'a [TypeGUID],
    ) -> HashMap<&'a TypeGUID, Vec<&'a SourceId>>;

    /// Retrieve all [`TypeGUID`]'s with the given name.
    fn type_guids_with_name(&self, source: &SourceId, name: &str) -> Vec<TypeGUID>;

    fn type_with_guid(&self, source: &SourceId, guid: &TypeGUID) -> Option<Type>;

    fn has_type_with_guid(&self, source: &SourceId, guid: &TypeGUID) -> bool {
        self.type_with_guid(source, guid).is_some()
    }

    /// Get the sources that contain functions with the given [`FunctionGUID`].
    fn sources_with_function_guid(&self, guid: &FunctionGUID) -> Vec<&SourceId>;

    // TODO: Allocating with Vec is not good.
    /// Plural version of [`Container::sources_with_function_guid`].
    ///
    /// Each source will have a list of the containing GUID's so that when looking up a source you give
    /// it only the GUID's that it knows about, for networking this means cutting down traffic significantly.
    fn sources_with_function_guids<'a>(
        &'a self,
        guids: &'a [FunctionGUID],
    ) -> HashMap<&'a FunctionGUID, Vec<&'a SourceId>>;

    fn functions_with_guid(&self, source: &SourceId, guid: &FunctionGUID) -> Vec<Function>;

    fn has_function_with_guid(&self, source: &SourceId, guid: &FunctionGUID) -> bool {
        !self.functions_with_guid(source, guid).is_empty()
    }
}
