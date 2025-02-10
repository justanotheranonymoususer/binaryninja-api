use uuid::Uuid;
use warp::r#type::class::TypeClass::Void;
use warp::r#type::Type;
use warp::signature::function::constraints::FunctionConstraint;
use warp::signature::function::{Function, FunctionGUID, NAMESPACE_FUNCTION};
use warp::symbol::class::SymbolClass;
use warp::symbol::Symbol;
use warp_ninja::container::memory::MemoryContainer;
use warp_ninja::container::SourceId;
use warp_ninja::matcher::{Matcher, MatcherSettings};

fn create_mock_function(name: &str) -> (FunctionGUID, Function) {
    let guid = FunctionGUID::from(Uuid::new_v5(&NAMESPACE_FUNCTION, name.as_bytes()));
    let function = Function {
        guid,
        symbol: Symbol {
            name: name.to_string(),
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
fn test_match_function() {
    let matcher_settings = MatcherSettings::default();
    let matcher = Matcher::new(matcher_settings);

    let (func_guid, func) = create_mock_function("test_match_function");

    let mut memory_container = MemoryContainer::new();
    let source = SourceId::new();
    memory_container.with_source_function(source, func_guid, func);

    // TODO: How to create a mock bv with the guid? i guess maybe we should just load
    // TODO: An object file instead.
    // // Create a mock BNFunction
    // let bn_function = BNFunction {
    //     // Add appropriate fields for BNFunction
    // };

    // // Act
    // matcher.match_function(&memory_container, source_id, &bn_function);

    // Assert
    // Add assertions here to verify the behavior of `match_function`
}

#[test]
fn test_match_function_from_constraints() {
    let matcher_settings = MatcherSettings::default();
    let matcher = Matcher::new(matcher_settings);
    let (func_guid, mut function) = create_mock_function("test_constraint_function");
    // Add constraint
    function.constraints.adjacent.insert(FunctionConstraint {
        guid: None,
        symbol: Some(Symbol {
            name: "my_function".to_string(),
            modifiers: Default::default(),
            class: SymbolClass::Function,
        }),
        offset: -10,
    });

    let matched_func_1 = function.clone();
    let mut matched_func_2 = function.clone();
    // Remove the constraint from 2, this means that the first matched_func should match.
    matched_func_2.constraints.adjacent.clear();
    let matched_functions = vec![matched_func_1, matched_func_2];

    // TODO: How to mock bv functions?
    // let matched = matcher.match_function_from_constraints(&function, &matched_functions);
    //
    // // Assert
    // assert!(matched.is_some());
    // assert_eq!(matched.unwrap(), &function);
}

#[test]
fn test_add_type_to_view() {
    let matcher_settings = MatcherSettings::default();
    let matcher = Matcher::new(matcher_settings);

    // let memory_container = create_mock_memory_container();
    //
    // // Mock components needed for the test
    // let bn_architecture = /* Mock BNArchitecture */;
    // let source_id = memory_container.sources.keys().next().unwrap();
    // let bn_view = /* Mock BinaryView */;
    // let bn_type = /* Mock Type */;
    //
    // // Act
    // matcher.add_type_to_view(
    //     &memory_container,
    //     source_id,
    //     &bn_view,
    //     &bn_architecture,
    //     &bn_type,
    // );

    // Assert
    // Add assertions here to verify the behavior of `add_type_to_view`
}
