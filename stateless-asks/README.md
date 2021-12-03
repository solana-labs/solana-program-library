# Stateless Offer

 Simple program to make token offers to any bidder that can satisify
 the constraints.

 This program is stateless.  It is up to the maker to advertize.  It
 uses the PDA as a one way hash of the offer that the maker wants
 to create.  The maker then only needs to approve their token to the
 PDA address for the taker to receive the items.  The maker doesn't
 need to be online to complete the transaction, but needs to advertize
 the offer off-chain.

 ## Maker
 1. compute the offer PDA
 2. approve the token delegation for the amount to the PDA
 3. publish the offer off-chain

 ## Taker
 1. Create the offer TX
 2. Submit the TX to the stateless-offer program

 To cancel, the maker simply needs to cancel the delegation.
