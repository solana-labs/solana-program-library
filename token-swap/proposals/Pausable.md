# Pausable & Ownable 

Implement shared library for SPL that can be used to extend contracts with ability to pause, resume and check for the owner before instructions are executed.

PR should include:
* Implementation of Ownable library with:
    - exposed current owner of the program
    - ability to transfer/renounce ownership
    - helper function to verify if call is issued by the owner
* Implementation of Pausable library with operations
    - pause
    - resume
* Only owner can pause/resume normal operations
* Example usage how other programs can interact with library
* Unit tests 

Links: 
* [Pausable solidity contract](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/24a0bc23cfe3fbc76f8f2510b78af1e948ae6651/contracts/security/Pausable.sol)
* [Ownable solidity contract](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/24a0bc23cfe3fbc76f8f2510b78af1e948ae6651/contracts/access/Ownable.sol)
