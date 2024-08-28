fn main() {
    println!("Hello");
    let a = 3;
    let something = TestStruct::new();
}

struct TestStruct {
    pub a: String,
}

impl TestStruct {
    pub fn new() -> Self {
        Self {
            a: "he".to_string(),
        }
    }
}
