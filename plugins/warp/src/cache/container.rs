use crate::cache::ViewID;
use crate::container::Container;
use binaryninja::binary_view::BinaryView;
use dashmap::DashMap;
use std::sync::OnceLock;

pub static CONTAINER_CACHE: OnceLock<DashMap<ViewID, ContainerCache>> = OnceLock::new();

pub fn cached_containers(view: &BinaryView, f: impl Fn(&dyn Container)) {
    let view_id = ViewID::from(view.as_ref());
    let containers_cache = CONTAINER_CACHE.get_or_init(Default::default);
    if let Some(cache) = containers_cache.get(&view_id) {
        for container in cache.containers() {
            f(container);
        }
    }
}

// TODO: The static lifetime here is a little wierd... (we need it to Box)
pub fn add_cached_container(view: &BinaryView, container: impl Container + 'static) {
    let view_id = ViewID::from(view.as_ref());
    let containers_cache = CONTAINER_CACHE.get_or_init(Default::default);
    let mut cache = containers_cache
        .entry(view_id)
        .or_insert_with(Default::default);
    // TODO: What happens if we have already added the container?
    // TODO: We need to replace container.
    cache.add_container(Box::new(container));
}

#[derive(Default)]
pub struct ContainerCache {
    pub cache: Vec<Box<dyn Container>>,
}

impl ContainerCache {
    pub fn add_container(&mut self, container: Box<dyn Container>) {
        self.cache.push(container);
    }

    pub fn containers(&self) -> impl Iterator<Item = &dyn Container> {
        self.cache.iter().map(|c| c.as_ref())
    }
}
