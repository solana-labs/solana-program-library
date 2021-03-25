# Pausable & Ownable 

Implement two programs for SPL that can be used to extend contracts with ability to pause, resume and check for the owner before instructions are executed.

An Owner program with the instructions you've listed:

    - set owner
    - renounce ownership
    - check owner

Additionally:
* an Owner struct should contain Option<Pubkey>
* library code should generate a pda, probably given (struct_key, program_id)

Given an Owner program, compose it with the Pause program.

    - pause
    - resume

Note: only owner can pause/resume normal operations

For both programs provide example usage from other programs via CPI and unit tests.

Links: 
* [Pausable solidity contract](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/24a0bc23cfe3fbc76f8f2510b78af1e948ae6651/contracts/security/Pausable.sol)
* [Ownable solidity contract](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/24a0bc23cfe3fbc76f8f2510b78af1e948ae6651/contracts/access/Ownable.sol)
