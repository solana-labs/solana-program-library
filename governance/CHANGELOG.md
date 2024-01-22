# SPL Governance Changelog

## v4.0.0 - WIP

- Mandatory signatories

## v3.1.1 - 25 Apr 2022

- Weighted multi choice voting
- Revoking own membership

## v3.1.0 - 13 Dec 2022

- Council governance plugins
- Non transferable and revokable membership
- Veto vote
- Council wallet rules
  - approval quorum
  - vote tipping
  - veto threshold
- Explicitly disabled options
  - community/council vote
  - community/council proposals
- Absolute max supply
- Proposal cool off time
- Proposal deposit

## v2.2.4 - 24 Mar 2022

- Support Anchor native account discriminators for `MaxVoterWeightRecord` and `VoterWeightRecord`

## v2.2.3 - 09 Feb 2022

- Fix serialisation of multiple instructions within a single proposal transaction

## v2.2.2 - 07 Feb 2022

- Native SOL Treasuries
- Multi choice and survey style proposals
- `voter_weight` and `max_voter_weight` addins
- Multiple instructions per proposal transaction
- Configurable tipping point (`Strict`, `Early`, `Disabled`)
- Owner signed off proposals
- `realm_authority` can create governances
- Program metadata and version detection
- Custom deposit amount for governance tokens

## v1.1.1 - 23 Sep 2021

- Constrain number of outstanding proposals per token owner to 10 at a time

## v1.0.8 - 1 Aug 2021

- First release
