use wonderbox::{register, Container};
use wonderbox_codegen::resolve_dependencies;

#[derive(Debug, Default)]
struct Foo {
    stored_string: String,
}

#[resolve_dependencies]
impl Foo {
    fn new(stored_string: String) -> Self {
        Self { stored_string }
    }
}

#[test]
fn test() {
    let mut container = Container::new();
    container.register_clone("foo".to_string());
    register!(container, Foo);

    let foo = container.resolve::<Foo>();
    assert!(foo.is_some())
}