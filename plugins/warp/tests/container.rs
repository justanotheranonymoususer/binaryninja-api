use std::collections::HashSet;
use std::str::FromStr;
use uuid::Uuid;
use warp::r#type::class::BooleanClass;
use warp::r#type::class::TypeClass::Void;
use warp::r#type::guid::TypeGUID;
use warp::r#type::Type;
use warp::signature::function::{Function, FunctionGUID};
use warp::symbol::class::SymbolClass;
use warp::symbol::Symbol;
use warp_ninja::container::disk::DiskContainer;
use warp_ninja::container::memory::MemoryContainer;
use warp_ninja::container::{Container, SourceId};

fn type_0() -> (TypeGUID, Type) {
    let ty = Type::builder()
        .name("type_0")
        .class(BooleanClass::builder().width(1).build())
        .build();
    (TypeGUID::from(&ty), ty)
}

fn type_1() -> (TypeGUID, Type) {
    let ty = Type::builder()
        .name("type_1")
        .class(BooleanClass::builder().width(4).build())
        .build();
    (TypeGUID::from(&ty), ty)
}

fn type_2() -> (TypeGUID, Type) {
    let ty = Type::builder()
        .name("type_2")
        .class(BooleanClass::builder().width(8).build())
        .build();
    (TypeGUID::from(&ty), ty)
}

fn func_0() -> (FunctionGUID, Function) {
    let guid = FunctionGUID::from(Uuid::from_str("d4e56ec8-1f2b-4a87-9f2c-bc3f10c4d8e9").unwrap());
    let function = Function {
        guid,
        symbol: Symbol {
            name: "func_0".to_string(),
            modifiers: Default::default(),
            class: SymbolClass::Function,
        },
        // TODO: We might want to give this an actual function type.
        ty: Type::builder::<String, _>().class(Void).build(),
        constraints: Default::default(),
    };
    (guid, function)
}

fn func_1() -> (FunctionGUID, Function) {
    let guid = FunctionGUID::from(Uuid::from_str("b713c293-463a-5baa-b31b-ed010510d5c0").unwrap());
    let function = Function {
        guid,
        symbol: Symbol {
            name: "func_1".to_string(),
            modifiers: Default::default(),
            class: SymbolClass::Function,
        },
        // TODO: We might want to give this an actual function type.
        ty: Type::builder::<String, _>().class(Void).build(),
        constraints: Default::default(),
    };
    (guid, function)
}

#[test]
fn test_sources_with_type_guid() {
    let source1 = SourceId::new();
    let source2 = SourceId::new();
    let (guid_1, ty_1) = type_0();
    let (guid_2, ty_2) = type_1();

    let container = MemoryContainer::new()
        .with_source_type(source1, guid_1, ty_1)
        .with_source_type(source1, guid_2, ty_2.clone())
        .with_source_type(source2, guid_2, ty_2);

    let result = container.sources_with_type_guid(&guid_2);
    // HashSet used for unordered comparison
    let result_set: HashSet<_> = result.into_iter().collect();
    let expected_set: HashSet<_> = vec![&source1, &source2].into_iter().collect();
    assert_eq!(result_set, expected_set);

    let result = container.sources_with_type_guid(&guid_1);
    // HashSet used for unordered comparison
    let result_set: HashSet<_> = result.into_iter().collect();
    let expected_set: HashSet<_> = vec![&source1].into_iter().collect();
    assert_eq!(result_set, expected_set);
}

#[test]
fn test_sources_with_function_guid() {
    let source1 = SourceId::new();
    let source2 = SourceId::new();
    let (guid_1, fn_1) = func_0();
    let (guid_2, fn_2) = func_1();

    let container = MemoryContainer::new()
        .with_source_function(source1, guid_1, fn_1)
        .with_source_function(source1, guid_2, fn_2.clone())
        .with_source_function(source2, guid_2, fn_2);

    let result = container.sources_with_function_guid(&guid_2);
    // HashSet used for unordered comparison
    let result_set: HashSet<_> = result.into_iter().collect();
    let expected_set: HashSet<_> = vec![&source1, &source2].into_iter().collect();
    assert_eq!(result_set, expected_set);

    let result = container.sources_with_function_guid(&guid_1);
    assert_eq!(result, vec![&source1]);
}

#[test]
fn test_sources_with_type_guids() {
    let source1 = SourceId::new();
    let source2 = SourceId::new();
    let (guid_1, ty_1) = type_0();
    let (guid_2, ty_2) = type_1();
    let (guid_3, ty_3) = type_2();

    let container = MemoryContainer::new()
        .with_source_type(source1, guid_1, ty_1)
        .with_source_type(source1, guid_2, ty_2.clone())
        .with_source_type(source2, guid_2, ty_2)
        .with_source_type(source2, guid_3, ty_3);

    let result = container.sources_with_type_guids(&[guid_2.clone(), guid_3.clone()]);
    assert_eq!(result.len(), 2);
    assert!(result.get(&source1).unwrap().contains(&guid_2));
    assert!(result.get(&source2).unwrap().contains(&guid_2));
    assert!(result.get(&source2).unwrap().contains(&guid_3));
}

#[test]
fn test_disk_container() {
    // We are going to use the OUT_DIR as the disk container path, this might have other artifacts in it
    // so it's a good test to make sure that we handle bad files gracefully.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR environment variable is not set");
    let out_dir_path = out_dir.parse().expect("Failed to parse OUT_DIR as path");
    // Write a simple test to open a file with DiskContainer
    let container = DiskContainer::new_from_dir(out_dir_path);

    // Test type retrieval.
    // Type -> f7IEWmRJLlEg7rzU : 7ae3f213-03c6-55f1-9575-b395e388e59a
    let type_0 = TypeGUID::from(Uuid::from_str("7ae3f213-03c6-55f1-9575-b395e388e59a").unwrap());
    let sources = container.sources_with_type_guid(&type_0);
    assert_eq!(sources.len(), 1);
    let result_type_0 = container
        .type_with_guid(&sources[0], &type_0)
        .expect("Failed to get type");
    assert_eq!(result_type_0.name, Some("f7IEWmRJLlEg7rzU".to_string()));
    let result_type_guids_0 = container.type_guids_with_name(sources[0], "f7IEWmRJLlEg7rzU");
    assert_eq!(result_type_guids_0.len(), 1);
    assert_eq!(result_type_guids_0[0], type_0);

    // Test function retrieval.
    // Function -> func_12 : b713c293-463a-5baa-b31b-ed010510d5c0
    let func_0 =
        FunctionGUID::from(Uuid::from_str("b713c293-463a-5baa-b31b-ed010510d5c0").unwrap());
    let func_sources = container.sources_with_function_guid(&func_0);
    assert_eq!(func_sources.len(), 1);
    let result_func_0 = container.functions_with_guid(&func_sources[0], &func_0);
    assert_eq!(result_func_0.len(), 1);
    assert_eq!(result_func_0[0].symbol.name, "func_12".to_string());
}
