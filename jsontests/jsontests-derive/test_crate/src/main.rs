#[macro_use]
extern crate jsontests_derive;
extern crate serde_json;

#[derive(JsonTests)]
#[directory = "/home/mike/dev/etcdev/sputnikvm/jsontests/res/files/vmSystemOperations"]
#[test_with = "test::dummy_tester"]
struct Tests;

fn main() {

}

pub mod test {
    use std::any::Any;
    use std::fmt::Display;
    pub fn dummy_tester<T: Any + Display + 'static>(test: &T) -> Result<(), ()> {
        println!("{}", test);
        Err(())
    }
}
