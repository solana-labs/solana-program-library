# SPL Interface Base

This crate provides common utils for on-chain programs to implement interfaces.

Some of these utils include:

- Handling the `InitializeExtraAccountMetaList` instruction to manage
  validation data for additional required accounts for any interface's
  instruction
- Handling the `Emit` instruction to emit asset data as program return data
- The `OptionalNonZeroPubkey` state type

> Note: If many interfaces depend on these utils to allow for further
> customization when implementing, it makes sense to include these as a
> standalone library (or interface) in order to de-couple it from any
> particular interface.
