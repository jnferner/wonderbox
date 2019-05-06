use wonderbox::{resolve_dependencies, Container};

#[derive(Debug, Default)]
struct Foo;

#[resolve_dependencies]
impl Foo {
    fn new() -> Self {
        Foo::default()
    }
}

#[test]
fn test() {
    let mut container = Container::new();
    container.register_autoresolved(|foo: Option<Foo>| foo.unwrap());
}
