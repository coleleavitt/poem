use std::sync::Arc;

use poem_openapi::{
    Object, Union,
    registry::{
        MetaDiscriminatorObject, MetaExternalDocument, MetaSchema, MetaSchemaRef, Registry,
    },
    types::{Example, ParseFromJSON, ToJSON, Type},
};
use serde_json::json;

fn get_meta<T: Type>() -> MetaSchema {
    let mut registry = Registry::new();
    T::register(&mut registry);
    registry.schemas.remove(&*T::name()).unwrap()
}

fn get_meta_by_name<T: Type>(name: &str) -> MetaSchema {
    let mut registry = Registry::new();
    T::register(&mut registry);
    registry.schemas.remove(name).unwrap()
}

#[test]
fn with_discriminator() {
    #[derive(Object, Debug, PartialEq)]
    struct AContents {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v3: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type")]
    enum MyObj {
        A(AContents),
        B(B),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_discriminator::MyObj"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    ("A".to_string(), "#/components/schemas/MyObj_A".to_string()),
                    ("B".to_string(), "#/components/schemas/MyObj_B".to_string()),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            ty: "string",
                            example: Some("A".into()),
                            enum_items: vec!["A".into()],
                            ..MetaSchema::ANY
                        }))
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("AContents".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            ty: "string",
                            example: Some("B".into()),
                            enum_items: vec!["B".into()],
                            ..MetaSchema::ANY
                        }))
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "A",
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap(),
        MyObj::A(AContents {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(AContents {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        Some(json!({
            "type": "A",
            "v1": 100,
            "v2": "hello",
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "B",
            "v3": true,
        })))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        Some(json!({
            "type": "B",
            "v3": true,
        }))
    );
}

#[test]
fn with_discriminator_mapping() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v3: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type")]
    enum MyObj {
        #[oai(mapping = "c")]
        A(A),
        #[oai(mapping = "d")]
        B(B),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_discriminator_mapping::MyObj"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    ("c".to_string(), "#/components/schemas/MyObj_A".to_string()),
                    ("d".to_string(), "#/components/schemas/MyObj_B".to_string()),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            ty: "string",
                            example: Some("c".into()),
                            enum_items: vec!["c".into()],
                            ..MetaSchema::ANY
                        }))
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("A".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![
                MetaSchemaRef::Inline(Box::new(MetaSchema {
                    required: vec!["type"],
                    properties: vec![(
                        "type",
                        MetaSchemaRef::Inline(Box::new(MetaSchema {
                            ty: "string",
                            example: Some("d".into()),
                            enum_items: vec!["d".into()],
                            ..MetaSchema::ANY
                        }))
                    )],
                    ..MetaSchema::new("object")
                })),
                MetaSchemaRef::Reference("B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "c",
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        Some(json!({
            "type": "c",
            "v1": 100,
            "v2": "hello",
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "d",
            "v3": true,
        })))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        Some(json!({
            "type": "d",
            "v3": true,
        }))
    );
}

#[test]
fn without_discriminator() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Union, Debug, PartialEq)]
    enum MyObj {
        A(A),
        B(bool),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::without_discriminator::MyObj"),
            ty: "object",
            discriminator: None,
            any_of: vec![
                MetaSchemaRef::Reference("A".to_string()),
                bool::schema_ref()
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        Some(json!({
            "v1": 100,
            "v2": "hello",
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!(true))).unwrap(),
        MyObj::B(true)
    );
    assert_eq!(MyObj::B(true).to_json(), Some(json!(true)));
}

#[test]
fn anyof() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v1: i32,
    }

    #[derive(Union, Debug, PartialEq)]
    enum MyObj {
        A(A),
        B(B),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::anyof::MyObj"),
            ty: "object",
            discriminator: None,
            any_of: vec![
                MetaSchemaRef::Reference("A".to_string()),
                MetaSchemaRef::Reference("B".to_string())
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
        })))
        .unwrap(),
        MyObj::B(B { v1: 100 })
    );
}

#[test]
fn oneof() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v1: i32,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(one_of)]
    enum MyObj {
        A(A),
        B(B),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::oneof::MyObj"),
            ty: "object",
            discriminator: None,
            one_of: vec![
                MetaSchemaRef::Reference("A".to_string()),
                MetaSchemaRef::Reference("B".to_string())
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
        })))
        .unwrap(),
        MyObj::B(B { v1: 100 })
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "v1": 100,
            "v2": "hello",
        })))
        .unwrap_err()
        .into_message(),
        "Expected input type \"MyObj\", found {\"v1\":100,\"v2\":\"hello\"}."
    );
}

#[test]
fn title_and_description() {
    /// A
    ///
    /// B
    /// C
    #[derive(Union, Debug, PartialEq)]
    enum MyObj {
        A(i32),
        B(f32),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(schema.description, Some("A\n\nB\nC"));
}

#[tokio::test]
async fn external_docs() {
    #[derive(Union, Debug, PartialEq)]
    #[oai(
        external_docs = "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
    )]
    enum MyObj {
        A(i32),
        B(f32),
    }

    let schema = get_meta::<MyObj>();
    assert_eq!(
        schema.external_docs,
        Some(MetaExternalDocument {
            url: "https://github.com/OAI/OpenAPI-Specification/blob/main/versions/3.1.0.md"
                .to_string(),
            description: None
        })
    );
}

#[tokio::test]
async fn generics() {
    #[derive(Union, Debug, PartialEq)]
    enum MyObj<A: ParseFromJSON + ToJSON, B: ParseFromJSON + ToJSON> {
        A(A),
        B(B),
    }

    let schema_i32_i64 = get_meta::<MyObj<i32, i64>>();
    let schema_f32_f64 = get_meta::<MyObj<f32, f64>>();

    assert_eq!(
        <MyObj<i32, i64>>::schema_ref(),
        MetaSchemaRef::Reference("MyObj_integer_int32_integer_int64".to_string())
    );
    assert_eq!(
        <MyObj<f32, f64>>::schema_ref(),
        MetaSchemaRef::Reference("MyObj_number_float_number_double".to_string())
    );

    assert_eq!(schema_i32_i64.any_of[0], i32::schema_ref());
    assert_eq!(schema_i32_i64.any_of[1], i64::schema_ref());

    assert_eq!(schema_f32_f64.any_of[0], f32::schema_ref());
    assert_eq!(schema_f32_f64.any_of[1], f64::schema_ref());
}

#[test]
fn rename_all() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        value: i32,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        value: String,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type", rename_all = "camelCase")]
    enum MyObj {
        PutInt(A),
        PutString(B),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::rename_all::MyObj"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    (
                        "putInt".to_string(),
                        "#/components/schemas/MyObj_PutInt".to_string()
                    ),
                    (
                        "putString".to_string(),
                        "#/components/schemas/MyObj_PutString".to_string()
                    ),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_PutInt".to_string()),
                MetaSchemaRef::Reference("MyObj_PutString".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "putInt",
            "value": 100,
        })))
        .unwrap(),
        MyObj::PutInt(A { value: 100 })
    );

    assert_eq!(
        MyObj::PutInt(A { value: 100 }).to_json(),
        Some(json!({
            "type": "putInt",
            "value": 100,
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "type": "putString",
            "value": "abc",
        })))
        .unwrap(),
        MyObj::PutString(B {
            value: "abc".to_string()
        })
    );

    assert_eq!(
        MyObj::PutString(B {
            value: "abc".to_string()
        })
        .to_json(),
        Some(json!({
            "type": "putString",
            "value": "abc",
        }))
    );
}

#[test]
fn with_externally_tagged() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v3: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(externally_tagged)]
    enum MyObj {
        A(A),
        B(B),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_externally_tagged::MyObj"),
            ty: "object",
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![MetaSchemaRef::Inline(Box::new(MetaSchema {
                required: vec!["A"],
                properties: vec![("A", MetaSchemaRef::Reference("A".to_string()),)],
                ..MetaSchema::new("object")
            })),],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![MetaSchemaRef::Inline(Box::new(MetaSchema {
                required: vec!["B"],
                properties: vec![("B", MetaSchemaRef::Reference("B".to_string()),)],
                ..MetaSchema::new("object")
            })),],
            ..MetaSchema::ANY
        }
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "A":{
                "v1": 100,
                "v2": "hello",
            }
        })))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        Some(json!({
            "A":{
                "v1": 100,
                "v2": "hello",
            }
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "B": {
                "v3": true,
            }
        })))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        Some(json!({
            "B": {
                "v3": true,
            }
        }))
    );
}

#[test]
fn with_externally_tagged_mapping() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
        v2: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v3: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(externally_tagged)]
    enum MyObj {
        #[oai(mapping = "c")]
        A(A),
        #[oai(mapping = "d")]
        B(B),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_externally_tagged_mapping::MyObj"),
            ty: "object",
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![MetaSchemaRef::Inline(Box::new(MetaSchema {
                required: vec!["c"],
                properties: vec![("c", MetaSchemaRef::Reference("A".to_string()),)],
                ..MetaSchema::new("object")
            })),],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![MetaSchemaRef::Inline(Box::new(MetaSchema {
                required: vec!["d"],
                properties: vec![("d", MetaSchemaRef::Reference("B".to_string()),)],
                ..MetaSchema::new("object")
            })),],
            ..MetaSchema::ANY
        }
    );

    let mut registry = Registry::new();
    MyObj::register(&mut registry);
    assert!(registry.schemas.contains_key("A"));
    assert!(registry.schemas.contains_key("B"));

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "c":{
                "v1": 100,
                "v2": "hello",
            }
        })))
        .unwrap(),
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
    );

    assert_eq!(
        MyObj::A(A {
            v1: 100,
            v2: "hello".to_string()
        })
        .to_json(),
        Some(json!({
            "c":{
                "v1": 100,
                "v2": "hello",
            }
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "d": {
                "v3": true,
            }
        })))
        .unwrap(),
        MyObj::B(B { v3: true })
    );

    assert_eq!(
        MyObj::B(B { v3: true }).to_json(),
        Some(json!({
            "d": {
                "v3": true,
            }
        }))
    );
}

#[test]
fn with_externally_tagged_primitives() {
    #[derive(Union, Debug, PartialEq)]
    #[oai(externally_tagged)]
    enum MyObj {
        A(String),
        B(bool),
    }

    let schema = get_meta::<MyObj>();

    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::with_externally_tagged_primitives::MyObj"),
            ty: "object",
            any_of: vec![
                MetaSchemaRef::Reference("MyObj_A".to_string()),
                MetaSchemaRef::Reference("MyObj_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(
        schema_myobj_a,
        MetaSchema {
            all_of: vec![MetaSchemaRef::Inline(Box::new(MetaSchema {
                required: vec!["A"],
                properties: vec![(
                    "A",
                    MetaSchemaRef::Inline(Box::new(MetaSchema::new("string")))
                )],
                ..MetaSchema::new("object")
            })),],
            ..MetaSchema::ANY
        }
    );

    let schema_myobj_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(
        schema_myobj_b,
        MetaSchema {
            all_of: vec![MetaSchemaRef::Inline(Box::new(MetaSchema {
                required: vec!["B"],
                properties: vec![(
                    "B",
                    MetaSchemaRef::Inline(Box::new(MetaSchema::new("boolean")))
                )],
                ..MetaSchema::new("object")
            })),],
            ..MetaSchema::ANY
        }
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "A":"hello",
        })))
        .unwrap(),
        MyObj::A("hello".to_string())
    );

    assert_eq!(
        MyObj::B(true).to_json(),
        Some(json!({
            "B": true,
        }))
    );

    assert_eq!(
        MyObj::parse_from_json(Some(json!({
            "A": "hello",
        })))
        .unwrap(),
        MyObj::A("hello".to_string())
    );

    assert_eq!(
        MyObj::B(true).to_json(),
        Some(json!({
            "B": true,
        }))
    );
}

#[test]
fn boxed_variant() {
    /// A large struct that would benefit from boxing
    #[derive(Object, Debug, PartialEq)]
    struct LargeStruct {
        field1: String,
        field2: String,
        field3: i64,
        field4: i64,
    }

    #[derive(Object, Debug, PartialEq)]
    struct SmallStruct {
        value: i32,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(one_of)]
    enum MyUnion {
        Large(Box<LargeStruct>),
        Small(SmallStruct),
    }

    let schema = get_meta::<MyUnion>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::boxed_variant::MyUnion"),
            ty: "object",
            discriminator: None,
            one_of: vec![
                MetaSchemaRef::Reference("LargeStruct".to_string()),
                MetaSchemaRef::Reference("SmallStruct".to_string())
            ],
            ..MetaSchema::ANY
        }
    );

    // Test parsing into boxed variant
    let parsed = MyUnion::parse_from_json(Some(json!({
        "field1": "hello",
        "field2": "world",
        "field3": 100,
        "field4": 200,
    })))
    .unwrap();

    assert_eq!(
        parsed,
        MyUnion::Large(Box::new(LargeStruct {
            field1: "hello".to_string(),
            field2: "world".to_string(),
            field3: 100,
            field4: 200,
        }))
    );

    // Test serializing boxed variant
    assert_eq!(
        MyUnion::Large(Box::new(LargeStruct {
            field1: "hello".to_string(),
            field2: "world".to_string(),
            field3: 100,
            field4: 200,
        }))
        .to_json(),
        Some(json!({
            "field1": "hello",
            "field2": "world",
            "field3": 100,
            "field4": 200,
        }))
    );

    // Test non-boxed variant still works
    assert_eq!(
        MyUnion::parse_from_json(Some(json!({
            "value": 42,
        })))
        .unwrap(),
        MyUnion::Small(SmallStruct { value: 42 })
    );

    assert_eq!(
        MyUnion::Small(SmallStruct { value: 42 }).to_json(),
        Some(json!({
            "value": 42,
        }))
    );
}

#[test]
fn arc_variant() {
    #[derive(Object, Debug, PartialEq)]
    struct SharedData {
        id: i64,
        name: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct SimpleData {
        flag: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    enum MyUnion {
        Shared(Arc<SharedData>),
        Simple(SimpleData),
    }

    let schema = get_meta::<MyUnion>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::arc_variant::MyUnion"),
            ty: "object",
            discriminator: None,
            any_of: vec![
                MetaSchemaRef::Reference("SharedData".to_string()),
                MetaSchemaRef::Reference("SimpleData".to_string())
            ],
            ..MetaSchema::ANY
        }
    );

    // Test parsing into Arc variant
    let parsed = MyUnion::parse_from_json(Some(json!({
        "id": 123,
        "name": "test",
    })))
    .unwrap();

    assert_eq!(
        parsed,
        MyUnion::Shared(Arc::new(SharedData {
            id: 123,
            name: "test".to_string(),
        }))
    );

    // Test serializing Arc variant
    assert_eq!(
        MyUnion::Shared(Arc::new(SharedData {
            id: 123,
            name: "test".to_string(),
        }))
        .to_json(),
        Some(json!({
            "id": 123,
            "name": "test",
        }))
    );

    // Test non-Arc variant still works
    assert_eq!(
        MyUnion::parse_from_json(Some(json!({
            "flag": true,
        })))
        .unwrap(),
        MyUnion::Simple(SimpleData { flag: true })
    );
}

#[test]
fn boxed_with_discriminator() {
    #[derive(Object, Debug, PartialEq)]
    struct TypeA {
        value_a: String,
    }

    #[derive(Object, Debug, PartialEq)]
    struct TypeB {
        value_b: i32,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type")]
    enum MyUnion {
        A(Box<TypeA>),
        B(TypeB),
    }

    let schema = get_meta::<MyUnion>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::boxed_with_discriminator::MyUnion"),
            ty: "object",
            discriminator: Some(MetaDiscriminatorObject {
                property_name: "type",
                mapping: vec![
                    (
                        "A".to_string(),
                        "#/components/schemas/MyUnion_A".to_string()
                    ),
                    (
                        "B".to_string(),
                        "#/components/schemas/MyUnion_B".to_string()
                    ),
                ]
            }),
            any_of: vec![
                MetaSchemaRef::Reference("MyUnion_A".to_string()),
                MetaSchemaRef::Reference("MyUnion_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    // Test parsing boxed variant with discriminator
    assert_eq!(
        MyUnion::parse_from_json(Some(json!({
            "type": "A",
            "value_a": "hello",
        })))
        .unwrap(),
        MyUnion::A(Box::new(TypeA {
            value_a: "hello".to_string()
        }))
    );

    // Test serializing boxed variant with discriminator
    assert_eq!(
        MyUnion::A(Box::new(TypeA {
            value_a: "hello".to_string()
        }))
        .to_json(),
        Some(json!({
            "type": "A",
            "value_a": "hello",
        }))
    );

    // Test non-boxed variant
    assert_eq!(
        MyUnion::parse_from_json(Some(json!({
            "type": "B",
            "value_b": 42,
        })))
        .unwrap(),
        MyUnion::B(TypeB { value_b: 42 })
    );

    assert_eq!(
        MyUnion::B(TypeB { value_b: 42 }).to_json(),
        Some(json!({
            "type": "B",
            "value_b": 42,
        }))
    );
}

#[test]
fn boxed_externally_tagged() {
    #[derive(Object, Debug, PartialEq)]
    struct DataA {
        x: i32,
    }

    #[derive(Object, Debug, PartialEq)]
    struct DataB {
        y: String,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(externally_tagged)]
    enum MyUnion {
        A(Box<DataA>),
        B(DataB),
    }

    let schema = get_meta::<MyUnion>();
    assert_eq!(
        schema,
        MetaSchema {
            rust_typename: Some("union::boxed_externally_tagged::MyUnion"),
            ty: "object",
            any_of: vec![
                MetaSchemaRef::Reference("MyUnion_A".to_string()),
                MetaSchemaRef::Reference("MyUnion_B".to_string()),
            ],
            ..MetaSchema::ANY
        }
    );

    // Test parsing boxed externally tagged variant
    assert_eq!(
        MyUnion::parse_from_json(Some(json!({
            "A": {
                "x": 100,
            }
        })))
        .unwrap(),
        MyUnion::A(Box::new(DataA { x: 100 }))
    );

    // Test serializing boxed externally tagged variant
    assert_eq!(
        MyUnion::A(Box::new(DataA { x: 100 })).to_json(),
        Some(json!({
            "A": {
                "x": 100,
            }
        }))
    );

    // Test non-boxed variant
    assert_eq!(
        MyUnion::parse_from_json(Some(json!({
            "B": {
                "y": "test",
            }
        })))
        .unwrap(),
        MyUnion::B(DataB {
            y: "test".to_string()
        })
    );

    assert_eq!(
        MyUnion::B(DataB {
            y: "test".to_string()
        })
        .to_json(),
        Some(json!({
            "B": {
                "y": "test",
            }
        }))
    );
}

#[test]
fn variant_doc_comments() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v2: String,
    }

    /// The union type description
    #[derive(Union, Debug, PartialEq)]
    #[oai(discriminator_name = "type")]
    enum MyObj {
        /// Variant A documentation
        A(A),
        /// Variant B documentation
        B(B),
    }

    // Check the union-level schema has the union description
    let schema = get_meta::<MyObj>();
    assert_eq!(schema.description, Some("The union type description"));

    // Check variant schemas have their individual descriptions
    let schema_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(schema_a.description, Some("Variant A documentation"));

    let schema_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(schema_b.description, Some("Variant B documentation"));
}

#[test]
fn variant_doc_comments_externally_tagged() {
    #[derive(Object, Debug, PartialEq)]
    struct A {
        v1: i32,
    }

    #[derive(Object, Debug, PartialEq)]
    struct B {
        v2: String,
    }

    /// The union type description
    #[derive(Union, Debug, PartialEq)]
    #[oai(externally_tagged)]
    enum MyObj {
        /// Variant A documentation
        A(A),
        /// Variant B documentation
        B(B),
    }

    // Check the union-level schema has the union description
    let schema = get_meta::<MyObj>();
    assert_eq!(schema.description, Some("The union type description"));

    // Check variant schemas have their individual descriptions
    let schema_a = get_meta_by_name::<MyObj>("MyObj_A");
    assert_eq!(schema_a.description, Some("Variant A documentation"));

    let schema_b = get_meta_by_name::<MyObj>("MyObj_B");
    assert_eq!(schema_b.description, Some("Variant B documentation"));
}

#[test]
fn flatten_union_in_object() {
    // Test for issue #945: Union flattened in Object should use allOf composition
    #[derive(Object, Debug, PartialEq)]
    struct ValueA {
        value: bool,
    }

    #[derive(Object, Debug, PartialEq)]
    struct ValueB {
        value: bool,
    }

    #[derive(Union, Debug, PartialEq)]
    #[oai(one_of = true, discriminator_name = "type")]
    enum UnionEnum {
        ValueA(ValueA),
        ValueB(ValueB),
    }

    #[derive(Object, Debug, PartialEq)]
    struct Flatten {
        #[oai(flatten)]
        inner: UnionEnum,
        other: String,
    }

    let schema = get_meta::<Flatten>();

    // The schema should use allOf to combine the Union with the additional
    // properties
    assert!(
        !schema.all_of.is_empty(),
        "Expected allOf to be used for flattened Union, got: {:?}",
        schema
    );

    // Should have 2 items in allOf: the Union reference and the inline object with
    // 'other' property
    assert_eq!(
        schema.all_of.len(),
        2,
        "Expected 2 items in allOf, got: {:?}",
        schema.all_of
    );

    // Verify JSON serialization works correctly
    let obj = Flatten {
        inner: UnionEnum::ValueB(ValueB { value: true }),
        other: "test".to_string(),
    };

    let json = obj.to_json().unwrap();
    assert_eq!(json["type"], "ValueB");
    assert_eq!(json["value"], true);
    assert_eq!(json["other"], "test");

    // Verify JSON deserialization works correctly
    let parsed = Flatten::parse_from_json(Some(json!({
        "type": "ValueA",
        "value": false,
        "other": "hello"
    })))
    .unwrap();

    assert_eq!(parsed.other, "hello");
    assert_eq!(parsed.inner, UnionEnum::ValueA(ValueA { value: false }));
}

/// Test that Union types can use the `example` attribute
/// This is the fix for issue #960
#[test]
fn with_example() {
    #[derive(Object, Debug, PartialEq, Clone)]
    struct Dog {
        name: String,
        breed: String,
    }

    #[derive(Object, Debug, PartialEq, Clone)]
    struct Cat {
        name: String,
        color: String,
    }

    #[derive(Union, Debug, PartialEq, Clone)]
    #[oai(discriminator_name = "type", one_of = true, example)]
    enum Pet {
        Dog(Dog),
        Cat(Cat),
    }

    impl Example for Pet {
        fn example() -> Self {
            Pet::Dog(Dog {
                name: "Max".to_string(),
                breed: "Labrador".to_string(),
            })
        }
    }

    let schema = get_meta::<Pet>();

    // Verify the example is present in the schema
    assert!(schema.example.is_some(), "Schema should have an example");

    // Verify the example contains the expected data
    let example = schema.example.unwrap();
    assert_eq!(example.get("type").and_then(|v| v.as_str()), Some("Dog"));
    assert_eq!(example.get("name").and_then(|v| v.as_str()), Some("Max"));
    assert_eq!(
        example.get("breed").and_then(|v| v.as_str()),
        Some("Labrador")
    );
}

/// Test Union with example but without discriminator
#[test]
fn with_example_no_discriminator() {
    #[derive(Object, Debug, PartialEq, Clone)]
    struct StringValue {
        value: String,
    }

    #[derive(Object, Debug, PartialEq, Clone)]
    struct IntValue {
        value: i32,
    }

    #[derive(Union, Debug, PartialEq, Clone)]
    #[oai(example)]
    enum Value {
        String(StringValue),
        Int(IntValue),
    }

    impl Example for Value {
        fn example() -> Self {
            Value::String(StringValue {
                value: "hello".to_string(),
            })
        }
    }

    let schema = get_meta::<Value>();
    assert!(schema.example.is_some(), "Schema should have an example");
}
