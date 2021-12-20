Simple Oracle Pair Token

1. pick a deposit token
2. pick the decider's pubkey
3. pick the mint term end slot
4. pick the decide term end slot, must be after 3

Each deposit token can mint one `Pass` and one `Fail` token up to
the mint term end slot.  After the decide term end slot the `Pass`
token converts 1:1 with the deposit token if and only if the decider
had set `pass` before the end of the decide term, otherwise the `Fail`
token converts 1:1 with the deposit token.
