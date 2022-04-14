# Security Policy

1. [Reporting security problems](#reporting)
1. [Security Bug Bounties](#bounty)
1. [Scope](#scope)
1. [Incident Response Process](#process)

<a name="reporting"></a>
## Reporting security problems to Solana

**DO NOT CREATE AN ISSUE** to report a security problem. Instead, please send an
email to security@solana.com and provide your github username so we can add you
to a new draft security advisory for further discussion.

Expect a response as fast as possible, typically within 72 hours.

<a name="bounty"></a>
## Security Bug Bounties
We offer bounties for critical security issues. Please see the
[Solana Security Bug Bounties](https://github.com/solana-labs/solana/security/policy#security-bug-bounties)
for details on classes of bugs and payment amounts.

<a name="scope"></a>
## Scope

Only a subset of programs within the Solana Program Library repo are deployed to
mainnet-beta and maintained by the team. Currently, this includes:

* [associated-token-account](https://github.com/solana-labs/solana-program-library/tree/master/associated-token-account/program)
* [feature-proposal](https://github.com/solana-labs/solana-program-library/tree/master/feature-proposal/program)
* [governance](https://github.com/solana-labs/solana-program-library/tree/master/governance/program)
* [memo](https://github.com/solana-labs/solana-program-library/tree/master/memo/program)
* [name-service](https://github.com/solana-labs/solana-program-library/tree/master/name-service/program)
* [stake-pool](https://github.com/solana-labs/solana-program-library/tree/master/stake-pool/program)
* [token](https://github.com/solana-labs/solana-program-library/tree/master/token/program)

If you discover a critical security issue in an out-of-scope program, your finding
may still be valuable.

Many programs, including
[token-swap](https://github.com/solana-labs/solana-program-library/tree/master/token-swap/program)
and [token-lending](https://github.com/solana-labs/solana-program-library/tree/master/token-lending/program),
have been forked and deployed by prominent ecosystem projects, many of which
have their own bug bounty programs.

While we cannot guarantee a bounty from another entity, we can help determine who
may be affected and put you in touch the corresponding teams.

<a name="process"></a>
## Incident Response Process

In case an incident is discovered or reported, the
[Solana Security Incident Response Process](https://github.com/solana-labs/solana/security/policy#incident-response-process)
will be followed to contain, respond and remediate. 
