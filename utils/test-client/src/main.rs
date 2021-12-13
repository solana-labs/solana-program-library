fn main() {
    let sdk_dep = solana_sdk::signature::Signature::default();
    println!("Yes have some sdk_dep {:?}", sdk_dep);
    let memo_dep = spl_memo::id();
    println!("Yes have some memo_dep {:?}", memo_dep);
    let token_dep = spl_token::id();
    println!("Yes have some token_dep {:?}", token_dep);
    let token_swap_dep = spl_token_swap::id();
    println!("Yes have some token_swap_dep {:?}", token_swap_dep);
}
