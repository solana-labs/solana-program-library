---
title: Introduction
---

The Transfer Hook Interface is one of several interfaces introduced within the
Solana Program Library that can be implemented by any Solana program.

Token-2022 implements this interface, as described in the [Transfer Hook
Extension Guide](../../token-2022/extensions#transfer-hook). Additionally, a
[reference
implementation](https://github.com/solana-labs/solana-program-library/tree/master/token/transfer-hook/example)
can be found in the SPL GitHub repository, detailing
how one might implement this interface in their own program.

Transfer Hook is designed to allow programs to "hook" additional functionality
into token transfers. The program performing the transfer can then CPI into the
transfer hook program using the interface-defined instructions to perform the
custom functionality.

In the case of Token-2022, the presence of a transfer hook program is assigned
to a mint using a mint extension, and this extension tells Token-2022 which
program to CPI to whenever a transfer is conducted. However, this particular
implementation is not required, and the interface can be implemented in a
variety of ways!

With this interface, programs can compose highly customizable transfer
functionality that can be compatible with many other programs - particularly
tokens who implement the SPL Token interface.
