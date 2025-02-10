use crate::container::{Container, SourceId};
use std::collections::HashMap;
use std::fmt::Display;
use warp::r#type::guid::TypeGUID;
use warp::r#type::Type;
use warp::signature::function::{Function, FunctionGUID};
use warp::signature::Data;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryContainer {
    sources: HashMap<SourceId, MemorySource>,
}

impl MemoryContainer {
    pub fn new() -> Self {
        MemoryContainer::default()
    }

    pub fn with_source(mut self, id: SourceId, source: MemorySource) -> Self {
        self.sources.insert(id, source);
        self
    }

    pub fn with_source_function(
        mut self,
        id: SourceId,
        guid: FunctionGUID,
        func: Function,
    ) -> Self {
        self.sources
            .entry(id)
            .or_default()
            .functions
            .entry(guid)
            .or_default()
            .push(func);
        self
    }

    pub fn with_source_type(mut self, id: SourceId, guid: TypeGUID, ty: Type) -> Self {
        self.sources.entry(id).or_default().types.insert(guid, ty);
        self
    }
}

impl Container for MemoryContainer {
    fn sources_with_type_guid(&self, guid: &TypeGUID) -> Vec<&SourceId> {
        self.sources
            .iter()
            .filter(|(_, source)| source.has_type_with_guid(guid))
            .map(|(id, _)| id)
            .collect()
    }

    fn sources_with_type_guids<'a>(
        &'a self,
        guids: &'a [TypeGUID],
    ) -> HashMap<&'a TypeGUID, Vec<&'a SourceId>> {
        let mut result: HashMap<&'a TypeGUID, Vec<&'a SourceId>> = HashMap::new();
        for (source_id, source) in &self.sources {
            guids
                .iter()
                .filter(|guid| source.has_type_with_guid(guid))
                .for_each(|guid| result.entry(guid).or_default().push(source_id));
        }
        result
    }

    fn type_guids_with_name(&self, source: &SourceId, name: &str) -> Vec<TypeGUID> {
        self.sources
            .get(source)
            .map(|source| source.type_guids_with_name(name))
            .unwrap_or_default()
    }

    fn type_with_guid(&self, source: &SourceId, guid: &TypeGUID) -> Option<Type> {
        self.sources.get(source)?.type_with_guid(guid)
    }

    fn sources_with_function_guid(&self, guid: &FunctionGUID) -> Vec<&SourceId> {
        self.sources
            .iter()
            .filter(|(_, source)| source.has_function_with_guid(guid))
            .map(|(id, _)| id)
            .collect()
    }

    fn sources_with_function_guids<'a>(
        &'a self,
        guids: &'a [FunctionGUID],
    ) -> HashMap<&'a FunctionGUID, Vec<&'a SourceId>> {
        let mut result: HashMap<&'a FunctionGUID, Vec<&'a SourceId>> = HashMap::new();
        for (source_id, source) in &self.sources {
            guids
                .iter()
                .filter(|guid| source.has_function_with_guid(guid))
                .for_each(|guid| result.entry(guid).or_default().push(source_id));
        }
        result
    }

    fn functions_with_guid(&self, source: &SourceId, guid: &FunctionGUID) -> Vec<Function> {
        self.sources
            .get(source)
            .map(|source| source.functions_with_guid(guid))
            .unwrap_or_default()
    }
}

impl Display for MemoryContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemoryContainer")
    }
}

/// An in-memory store of functions.
///
/// While not technically a container source, this is typically an overlay on top of a container source.
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub struct MemorySource {
    pub functions: HashMap<FunctionGUID, Vec<Function>>,
    pub types: HashMap<TypeGUID, Type>,
    pub named_types: HashMap<String, Vec<TypeGUID>>,
}

impl MemorySource {
    pub fn type_guids_with_name(&self, name: &str) -> Vec<TypeGUID> {
        // TODO: The function here is a little goofy.
        // TODO: This is cloned.
        self.named_types.get(name).cloned().unwrap_or_default()
    }

    pub fn type_with_guid(&self, guid: &TypeGUID) -> Option<Type> {
        // TODO: This is cloned.
        self.types.get(guid).cloned()
    }

    pub fn functions_with_guid(&self, guid: &FunctionGUID) -> Vec<Function> {
        // TODO: The function here is a little goofy.
        // TODO: This is cloned.
        self.functions.get(guid).cloned().unwrap_or_default()
    }

    pub fn has_type_with_guid(&self, guid: &TypeGUID) -> bool {
        self.type_with_guid(guid).is_some()
    }

    pub fn has_function_with_guid(&self, guid: &FunctionGUID) -> bool {
        !self.functions_with_guid(guid).is_empty()
    }
}

// TODO: I really dislike this...
impl From<Data> for MemorySource {
    fn from(data: Data) -> Self {
        let functions = data.functions.into_iter().fold(
            HashMap::new(),
            |mut map: HashMap<FunctionGUID, Vec<_>>, func| {
                map.entry(func.guid).or_default().push(func);
                map
            },
        );
        let types = data
            .types
            .iter()
            .map(|ty| (ty.guid, ty.ty.clone()))
            .collect();
        let named_types = data
            .types
            .into_iter()
            .filter_map(|t| Some((t.ty.name?, t.guid)))
            .fold(
                HashMap::new(),
                |mut map: HashMap<String, Vec<_>>, (name, guid)| {
                    map.entry(name).or_default().push(guid);
                    map
                },
            );

        Self {
            functions,
            types,
            named_types,
        }
    }
}
