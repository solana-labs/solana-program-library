---
title: Transfer Hook Interface
---

The Transfer Hook Interface is one of several interfaces introduced within the
Solana Program Library that can be implemented by any Solana program.

During transfers, Token-2022 calls a mint's configured transfer hook program
using this interface, as described in the
[Transfer Hook Extension Guide](../../token-2022/extensions#transfer-hook).
Additionally, a
[reference implementation](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook/example)
can be found in the SPL GitHub repository, detailing
how one might implement this interface in their own program.

The Transfer Hook Interface is designed to allow token creators to "hook"
additional functionality into token transfers. The token program CPIs into the
transfer hook program using the interface-defined instruction. The transfer
hook program can then perform any custom functionality.

In the case of Token-2022, a token creator configures a transfer hook program
using a mint extension, and this extension tells Token-2022 which program to
invoke whenever a transfer is conducted.

With this interface, programs can compose highly customizable transfer
functionality that can be compatible with many other programs - particularly
tokens who implement the SPL Token interface.
