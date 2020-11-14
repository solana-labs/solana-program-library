use ouroboros::self_referencing;

#[self_referencing]
struct BoxAndRef {
    data: Box<i32>,
    #[borrows(data)]
    data_ref: &'this i32,
}

fn main() {
    let instance = BoxAndRefBuilder {
        data: Box::new(12),
        data_ref_builder: |dref| dref,
    }.build();
    let data_ref = instance.with_data_ref(|dref| *dref);
    drop(instance);
    println!("{:?}", data_ref);
}
