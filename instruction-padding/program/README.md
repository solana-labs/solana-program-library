# Instruction Pad Program

A program for padding instructions with additional data or accounts, to be used
for testing larger transactions, either more instruction data, or more accounts.

The main use-case is with solana-bench-tps, where we can see the impact of larger
transactions through TPS numbers. With that data, we can develop a fair fee model
for large transactions.

It operates with two instructions: no-op and wrap.

* No-op: simply an instruction with as much data and as many accounts as desired,
of which none will be used for processing.
* Wrap: before the padding data and accounts, accepts a real instruction and
required accounts, and performs a CPI into the program specified by the instruction

Both of these modes add the general overhead of calling a BPF program, and
the wrap mode adds the CPI overhead.

Because of the overhead, it's best to use the instruction padding program with
all large transaction tests, and comparing TPS numbers between:

* using the program with no padding
* using the program with data and account padding

## Audit

The repository [README](https://github.com/solana-labs/solana-program-library#audits)
contains information about program audits.
