<p align="center">
  <a href="https://solana.com">
    <img alt="Solana" src="https://i.imgur.com/IKyzQ6T.png" width="250" />
  </a>
</p>

# SPL Noop Rust SDK

This is crate provides a wrapper for invoking `spl-noop`, which does nothing. 
It's primary use is circumventing log truncation when emitting application data by `invoke`-ing `spl-noop` with event data.

<p align="center">
  <a href="https://solana.com">
    <img alt="Solana" src="https://i.imgur.com/IKyzQ6T.png" width="250" />
  </a>
</p>

# SPL Account Compression Rust SDK (Beta)

More information about account compression can be found in [the solana-program-library repo](https://github.com/solana-labs/solana-program-library/tree/master/account-compression).

The [Solana Program Examples repo](https://github.com/solana-developers/program-examples) will eventually include examples of how to use this program.

`spl-noop` and this crate's implementation are targeted towards supporting [account-compression](https://github.com/solana-labs/solana-program-library/tree/master/account-compression) and may be subject to change.