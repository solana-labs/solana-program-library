# Governance

Governance is a program the chief purpose of which is to control the upgrade of other programs through democratic means. It can also be used as an authority provider for mints and other forms of access control as well where we may want a voting population to vote on disbursement of access or funds collectively.

## Architecture

### Configs

The basic building block of governance is the Config object. It ties a Program ID to a Council mint (optional) and a Governance mint, as well as other configuration options. The tuple of (ProgramID, YourProgramID, CouncilMintID, GovernanceMintID) (where ProgramID is the ID of this program itself) forms the seeds of a Program Derived Address, and this program derived address is what is used as the address of the Config object for your Program ID for this Council mint and Governance mint.

What this means is that there can only ever be ONE Config object for any given Program, Council, Governance mint combination. Whoever creates it first makes the only one in existence.

### How does authority work?

Governance can handle arbitrary executions of code, but it's real power lies in the power to upgrade programs. It does this through executing commands to the bpf-upgrade-loader program. Bpf-upgrade-loader allows any signer who has Upgrade authority over a Buffer account and the Program account itself to upgrade it using it's Upgrade command. Normally, this is the developer who created and deployed the program, and this creation of the Buffer account containing the new program data and overwriting of the existing Program account's data with it is handled in the background for you by the solana program deploy cli command. However, in order for Governance to be useful, Governance now needs this authority.

The proposal-loader-cli, when used for the first time, also takes your program like a normal deployment and creates a Buffer account for you, but this time it delegates the authority of the Buffer account not to your wallet but to the PDA given by the Config you passed into it, and because you also have authority over your Program, you can delegate your authority to the PDA (notice once transferred, you cannot regain the authority back). From here on out, now Governance has the authority to upgrade, not you, and proposal-loader-cli will print out commands you can use in your proposals to upgrade your programs.

### Proposals/Sets

A 'Set' as it's called in the contract, or a Proposal as it's called on the front end, is a created instance of a Config. It is created by someone and tied to a given Config and has a set of executible commands to it, a name and a description. It goes through various states (draft, voting, executing) and users can vote on it if they have tokens. It's rules are determined by the Config that it is tied to, and when it executes, it is only eligible to use the PDA authority given by the Config. So a Proposal for Sushi cannot for instance upgrade the Program for Uniswap.

A Set is created by one user who has an Admin token, and with this power they can add other Signers to the
Set. These Signers can then add commands to the set and/or sign off on the Set. Once all Signers have signed off on the Set, the Set leaves Draft mode and enters Voting mode. Voting mode lasts as long as the
Config has it configured to last, and during this time people holding Governance tokens may vote on the
Set/Proposal. Once the Set is "tipped" it either enters the Defeated or Executing state. If Executed, it
enters Completed only once all commands have been run.

A command can be run by any one at any time after the Slot length has transpired on the given command struct.

### CustomSingleSignerTransaction

We only support one kind of executible command right now, and this is the CustomSingleSignerTransaction type. A Set can have a certain number of these, and they run independently of one another. These contain the actual data for a command, and how long after the voting phase a user must wait before they can be executed.

### Voting Dynamics

Each Proposal/Set that gets created creates three mints for itself: a Voting mint, a Yes mint, and a No mint. It also creates a holding account for all source tokens that people will deposit in order to vote.

To vote, one needs to convert the source token for a given Proposal/Set into a Voting token. Think of this as an undecided token - one may deposit 1 Uniswap and get 1 Voting token. If they then vote "Yes", they
use the Vote command to burn the Vote token, and in exchange receive 1 minted Yes token. Voting No does
likewise with the No mint. There is a conservation of energy going on here where the total tokens the person possesses is always equivalent to their starting token amount.

Once the vote completes, they may redeem their collected tokens (Yes, No, Undecided) for their starting tokens, whether they be Uniswap, Sushi, or what have you, from the holding account on the Proposal/Set.

This all happens behind the scenes.

### Councils and Governance

Each Config that gets created has the option to also have a Council mint. A council mint is simply a separate mint from the Governance mint. What this means is that users can submit Proposals/Sets that have a different voting population from a different mint that can affect the same program. A practical application of this policy may be to have a very large population control major version bumps of Solana via normal SOL, for instance, but hotfixes be controlled via Council tokens, of which there may be only 30, and which may be themselves minted and distributed via proposals by the governing population.
