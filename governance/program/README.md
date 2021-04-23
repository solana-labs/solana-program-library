# Governance

Governance is a program the chief purpose of which is to control the upgrade of other programs through democratic means.
It can also be used as an authority provider for mints and other forms of access control as well where we may want
a voting population to vote on disbursement of access or funds collectively.

## Architecture

### Governance account

The basic building block of governance is the Governance account. It ties a governed Program ID to a Governance mint
and an optional Council mint and holds configuration options defining governance rules.
The governed Program ID is used as the seed for a [Program Derived Address](https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses),
and this program derived address is what is used as the address of the Governance account for your Program ID
and the corresponding Governance mint and Council mint (if provided).

What this means is that there can only ever be ONE Governance account for a given Program.
The governance program validates at creation time of the Governance account that the current upgrade authority of the program
taken under governance signed the transaction.

Note: In future versions, once allowed in solana runtime, the governance program will take over the upgrade authority
of the governed program when the Governance account is created.

### How does authority work?

Governance can handle arbitrary executions of code, but it's real power lies in the power to upgrade programs.
It does this through executing commands to the bpf-upgrade-loader program.
Bpf-upgrade-loader allows any signer who has Upgrade authority over a Buffer account and the Program account itself
to upgrade it using its Upgrade command.
Normally, this is the developer who created and deployed the program, and this creation of the Buffer account containing
the new program data and overwriting of the existing Program account's data with it is handled in the background for you
by the Solana program deploy cli command.
However, in order for Governance to be useful, Governance now needs this authority.

### Proposal accounts

A Proposal is an instance of a Governance created to vote on and execute given set of changes.
It is created by someone and tied to a given Governance account and has a set of executable commands to it,
a name and a description.
It goes through various states (draft, voting, executing) and users can vote on it
if they have relevant Governance or Council tokens.
It's rules are determined by the Governance account that it is tied to, and when it executes,
it is only eligible to use the [Program Derived Address](https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses)
authority given by the Governance account.
So a Proposal for Sushi cannot for instance upgrade the Program for Uniswap.

A Proposal is created by one user who has an Admin token, and with this power they can add other Signers to the Proposal.
These Signers can then add commands to the set and/or sign off on the Proposal. Once all Signers have signed off on the Proposal,
the Proposal leaves Draft mode and enters Voting mode.
Voting mode lasts as long as the Governance has it configured to last, and during this time
people holding Governance or Council tokens may vote on theProposal.
Once the Proposal is "tipped" it either enters the Defeated or Executing state.
If Executed, it enters Completed only once all commands have been run.

A command can be run by any one at any time after the Slot length has transpired on the given command struct.

### CustomSingleSignerTransaction

We only support one kind of executable command right now, and this is the CustomSingleSignerTransaction type.
A Proposal can have a certain number of these, and they run independently of one another.
These contain the actual data for a command, and how long after the voting phase a user must wait before they can be executed.

### Voting Dynamics

Each Proposal that gets created creates three mints for itself: a Voting mint, a Yes mint, and a No mint.
It also creates a holding account for all source tokens that people will deposit in order to vote.

To vote, one needs to convert the source token for a given Proposal into a Voting token.
Think of this as an undecided token - one may deposit 1 Uniswap and get 1 Voting token.
If they then vote "Yes", they use the Vote command to burn the Vote token, and in exchange receive 1 minted Yes token.
Voting No does likewise with the No mint.
There is a conservation of energy going on here where the total tokens the person possesses
is always equivalent to their starting token amount.

Once the vote completes, they may redeem their collected tokens (Yes, No, Undecided) for their starting tokens,
whether they be Uniswap, Sushi, or what have you, from the holding account on the Proposal.

This all happens behind the scenes.

### Councils and Governance

Each Governance that gets created has the option to also have a Council mint.
A council mint is simply a separate mint from the Governance mint.
What this means is that users can submit Proposals that have a different voting population from a different mint
that can affect the same program. A practical application of this policy may be to have a very large population control
major version bumps of Solana via normal SOL, for instance, but hot fixes be controlled via Council tokens,
of which there may be only 30, and which may be themselves minted and distributed via proposals by the governing population.
