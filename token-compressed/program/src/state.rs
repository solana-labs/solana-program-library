type SHA256 = [u8;32];

#[repr(C)]
enum State {
    AccountSet{root: SHA256},
}
