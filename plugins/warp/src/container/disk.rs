use crate::container::memory::MemorySource;
use crate::container::{Container, SourceId};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::path::PathBuf;
use walkdir::WalkDir;
use warp::r#type::guid::TypeGUID;
use warp::r#type::Type;
use warp::signature::function::{Function, FunctionGUID};
use warp::signature::Data;

// TODO: How to support remote projects? I.e. collaboration?
pub struct DiskContainer {
    name: String,
    sources: HashMap<SourceId, DiskContainerSource>,
}

impl DiskContainer {
    pub fn new(name: String, sources: HashMap<SourceId, DiskContainerSource>) -> Self {
        Self { name, sources }
    }

    pub fn new_from_dir(dir_path: PathBuf) -> Self {
        let name = dir_path.to_string_lossy().to_string();
        let sources = WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| DiskContainerSource::new(e.into_path()))
            .map(|source| (SourceId::new(), source))
            .collect();

        Self::new(name, sources)
    }
}

impl Container for DiskContainer {
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
        self.sources.get(source).unwrap().type_guids_with_name(name)
    }

    fn type_with_guid(&self, source: &SourceId, guid: &TypeGUID) -> Option<Type> {
        self.sources.get(source).unwrap().type_with_guid(guid)
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
        self.sources.get(source).unwrap().functions_with_guid(guid)
    }
}

impl Display for DiskContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Debug for DiskContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskContainer")
            .field("name", &self.name)
            .field("sources", &self.sources)
            .finish()
    }
}

#[derive(Eq, PartialEq)]
pub struct DiskContainerSource {
    path: PathBuf,
    // TODO: For now we read the entire file into memory.
    cache: MemorySource,
}

impl DiskContainerSource {
    // TODO: Where do we uuid from?
    // TODO: Returning option, really?
    pub fn new(path: PathBuf) -> Option<Self> {
        let contents = std::fs::read(&path).ok()?;
        Some(Self {
            path,
            // TODO: Right now we dont ever read the file, only initially.
            cache: Data::from_bytes(&contents)?.into(),
        })
    }

    // TODO: Create from file.

    /// Reads a file to data.
    pub fn read_as_data(&self) -> Option<Data> {
        let contents = std::fs::read(&self.path).ok()?;
        Data::from_bytes(&contents)
    }

    fn type_guids_with_name(&self, name: &str) -> Vec<TypeGUID> {
        self.cache.type_guids_with_name(name)
    }

    fn type_with_guid(&self, guid: &TypeGUID) -> Option<Type> {
        self.cache.type_with_guid(guid)
    }

    // TODO: When we support reading lazily instead of all in memory.
    fn has_type_with_guid(&self, guid: &TypeGUID) -> bool {
        self.cache.has_type_with_guid(guid)
    }

    fn functions_with_guid(&self, guid: &FunctionGUID) -> Vec<Function> {
        self.cache.functions_with_guid(guid)
    }

    // TODO: When we support reading lazily instead of all in memory.
    fn has_function_with_guid(&self, guid: &FunctionGUID) -> bool {
        self.cache.has_function_with_guid(guid)
    }
}

impl Hash for DiskContainerSource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Display for DiskContainerSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.to_string_lossy())
    }
}

impl Debug for DiskContainerSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskContainerSource")
            .field("path", &self.path)
            // .field("cache", &self.cache)
            .finish()
    }
}
