from solders.pubkey import Pubkey


VOTE_PROGRAM_ID = Pubkey.from_string("Vote111111111111111111111111111111111111111")
"""Program id for the native vote program."""

VOTE_STATE_LEN: int = 3762
"""Size of vote account."""
