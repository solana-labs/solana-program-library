Simple Oracle Pair Token

1. pick a deposit token
2. pick the decider signature
3. pick the mint term end slot
4. pick the decide term end slot, must be after 3


Each deposit token can mint one `Yes` and one `Not Yes` token up to
the mint term end slot.  After the decide term end slot the `Yes`
token converts 1:1 with the deposit token if and only if the decider
had set `yes` before the end of the decide term, otherwise the `Not
Yes` token converts 1:1 with the deposit token.
