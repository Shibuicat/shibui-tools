fn main() {}

trait B {}

struct A {
    field_1: Box<dyn B>,
}
