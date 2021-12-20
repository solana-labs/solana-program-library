# Serum DEX Dynamic market making curve for token-swap

Implement a curve for token-swap that accepts serum CLOB dex for the same pair. Implementation should 'split'/'route' portion of order using predefined curve for the pool and the rest to the dex as IOC order. Dynamic curve should find ideal size of the order based on the curve parameters and market depth for a given DEX.

Ideas: 
* CLOB fee discounts 
    - If pool contains SRM use tressure account to receive `taker` discounts from dex
    - Curve can provide additional stake pool for LPs to store SRM to receive `taker` discounts
* Fees
    - AMM should charge same fee structure with 30bps difference from fees in CLOB

## Links
1. CLOB implementation: https://github.com/project-serum/serum-dex/tree/master/dex