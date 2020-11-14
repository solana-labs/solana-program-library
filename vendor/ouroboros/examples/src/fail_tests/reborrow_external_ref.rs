use ouroboros::self_referencing;

#[self_referencing]
struct ReborrowExternalRef<'a> {
    external: &'a str,
    #[borrows(external)]
    reborrowed: &'this str,
}

fn main() {

}
