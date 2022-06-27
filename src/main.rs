#[link(name = "foo")]
extern {
    fn triple(x: i32) -> i32;
}

fn main() {
    let x = unsafe { triple(3) }; 
    println!("Hello, world! {}", x);
}
