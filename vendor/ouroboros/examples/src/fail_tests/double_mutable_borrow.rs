use ouroboros::self_referencing;

#[self_referencing]
struct S {
    a: Box<i32>,
    #[borrows(mut a, mut a)]
    b: &'this i32,
}

fn main() { }
