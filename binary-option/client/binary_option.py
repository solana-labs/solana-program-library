import json
from http import HTTPStatus
from cryptography.fernet import Fernet
import base64
import base58
import struct

from solana.publickey import PublicKey 
from solana.transaction import Transaction, AccountMeta, TransactionInstruction
from solana.account import Account 
from solana.rpc.api import Client
import solana.rpc.types as types
from solana.system_program import transfer, TransferParams
from spl.token._layouts import MINT_LAYOUT, ACCOUNT_LAYOUT
from spl.token.instructions import (
    get_associated_token_address, create_associated_token_account,
    mint_to, MintToParams,
)

SYSTEM_PROGRAM_ID = '11111111111111111111111111111111'
SYSVAR_RENT_ID = 'SysvarRent111111111111111111111111111111111'
ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID = 'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'
TOKEN_PROGRAM_ID = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
BINARY_OPTION_PROGRAM_ID = 'betw959P4WToez4DkuXwNsJszqbpe3HuY56AcG5yevx'


def initialize_binary_option_instruction(
    pool_account,
    escrow_mint_account,
    escrow_account,
    long_token_mint_account,
    short_token_mint_account,
    mint_authority_account,
    update_authority_account,
    token_account,
    system_account,
    rent_account,
    decimals
):
    keys = [
        AccountMeta(pubkey=pool_account, is_signer=True, is_writable=True),
        AccountMeta(pubkey=escrow_mint_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=escrow_account, is_signer=True, is_writable=True),
        AccountMeta(pubkey=long_token_mint_account, is_signer=True, is_writable=False),
        AccountMeta(pubkey=short_token_mint_account, is_signer=True, is_writable=False),
        AccountMeta(pubkey=mint_authority_account, is_signer=True, is_writable=False),
        AccountMeta(pubkey=update_authority_account, is_signer=True, is_writable=False),
        AccountMeta(pubkey=token_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=system_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=rent_account, is_signer=False, is_writable=False),
    ]
    data = struct.pack("<BB", 0, decimals)
    return TransactionInstruction(keys=keys, program_id=PublicKey(BINARY_OPTION_PROGRAM_ID), data=data)

def trade_instruction(
    pool_account,
    escrow_account,
    long_token_mint_account,
    short_token_mint_account,
    buyer,
    seller,
    buyer_account,
    seller_account,
    buyer_long_token_account,
    buyer_short_token_account,
    seller_long_token_account,
    seller_short_token_account,
    escrow_authority_account,
    token_account,
    size,
    buyer_price,
    seller_price,
):
    keys = [
        AccountMeta(pubkey=pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=escrow_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=long_token_mint_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=short_token_mint_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=buyer, is_signer=True, is_writable=False),
        AccountMeta(pubkey=seller, is_signer=True, is_writable=False),
        AccountMeta(pubkey=buyer_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=seller_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=buyer_long_token_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=buyer_short_token_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=seller_long_token_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=seller_short_token_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=escrow_authority_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=token_account, is_signer=False, is_writable=False),
    ]
    data = struct.pack("<BQQQ", 1, size, buyer_price, seller_price)
    return TransactionInstruction(keys=keys, program_id=PublicKey(BINARY_OPTION_PROGRAM_ID), data=data)

def settle_instruction(
    pool_account,
    winning_mint_account,
    pool_owner_account,
):
    keys = [
        AccountMeta(pubkey=pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=winning_mint_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=pool_owner_account, is_signer=True, is_writable=False),
    ]
    data = struct.pack("<B", 2)
    return TransactionInstruction(keys=keys, program_id=PublicKey(BINARY_OPTION_PROGRAM_ID), data=data)

def collect_instruction(
    pool_account,
    collector_account,
    collector_long_token_account,
    collector_short_token_account,
    collector_collateral_account,
    long_token_mint_account,
    short_token_mint_account,
    escrow_account,
    escrow_authority_account,
    token_account,
):
    keys = [
        AccountMeta(pubkey=pool_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=collector_account, is_signer=False, is_writable=False),
        AccountMeta(pubkey=collector_long_token_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=collector_short_token_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=collector_collateral_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=long_token_mint_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=short_token_mint_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=escrow_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=escrow_authority_account, is_signer=False, is_writable=True),
        AccountMeta(pubkey=token_account, is_signer=False, is_writable=False),
    ]
    data = struct.pack("<B", 3)
    return TransactionInstruction(keys=keys, program_id=PublicKey(BINARY_OPTION_PROGRAM_ID), data=data)

class BinaryOption():

    def __init__(self, cfg):
        self.private_key = list(base58.b58decode(cfg["PRIVATE_KEY"]))[:32]
        self.public_key = cfg["PUBLIC_KEY"]
        self.cipher = Fernet(cfg["DECRYPTION_KEY"])


    def initialize(self, api_endpoint, escrow_mint, decimals=2, skip_confirmation=True):
        msg = ""
        # Initialize Clinet
        client = Client(api_endpoint)
        msg += "Initialized client"
        # Create account objects
        source_account = Account(self.private_key)
        pool = Account()
        long_escrow = Account()
        short_escrow = Account()
        long_mint = Account()
        short_mint = Account()
        # List non-derived accounts
        pool_account = pool.public_key()
        escrow_mint_account = PublicKey(escrow_mint)
        escrow_account = long_escrow.public_key()
        long_token_mint_account = long_mint.public_key()
        short_token_mint_account = short_mint.public_key()
        mint_authority_account = source_account.public_key()
        update_authority_account = source_account.public_key()
        token_account = PublicKey(TOKEN_PROGRAM_ID)
        system_account = PublicKey(SYSTEM_PROGRAM_ID)
        rent_account = PublicKey(SYSVAR_RENT_ID)
        msg += " | Gathered accounts"
        # List signers
        signers = [source_account, long_mint, short_mint, long_escrow, short_escrow, pool]
        # Start transaction
        tx = Transaction()
        # Create Token Metadata
        init_binary_option_ix =  initialize_binary_option_instruction(
            pool_account,
            escrow_mint_account,
            escrow_account,
            long_token_mint_account,
            short_token_mint_account,
            mint_authority_account,
            update_authority_account,
            token_account,
            system_account,
            rent_account,
            decimals,
        )
        tx = tx.add(init_binary_option_ix)
        msg += f" | Creating binary option"
        # Send request
        try:
            response = client.send_transaction(tx, *signers, opts=types.TxOpts(skip_confirmation=skip_confirmation))
            return json.dumps(
                {
                    'status': HTTPStatus.OK,
                    'binary_option': str(pool_account),
                    'msg': msg + f" | Successfully created binary option {str(pool_account)}",
                    'tx': response.get('result') if skip_confirmation else response['result']['transaction']['signatures'],
                }
            )
        except Exception as e:
            msg += f" | ERROR: Encountered exception while attempting to send transaction: {e}"
            raise(e)


    def trade(self, api_endpoint, pool_account, buyer_encrypted_private_key, seller_encrypted_private_key, size, buyer_price, seller_price, skip_confirmation=True):
        msg = ""
        client = Client(api_endpoint)
        msg += "Initialized client"
        # Create account objects
        buyer_private_key = list(self.cipher.decrypt(buyer_encrypted_private_key))
        seller_private_key = list(self.cipher.decrypt(seller_encrypted_private_key))
        assert(len(buyer_private_key) == 32)
        assert(len(seller_private_key) == 32)
        source_account = Account(self.private_key)
        buyer = Account(buyer_private_key)
        seller = Account(seller_private_key)
        # Signers
        signers = [buyer, seller, source_account]
        pool = self.load_binary_option(api_endpoint, pool_account)
        # List non-derived accounts
        pool_account = PublicKey(pool_account) 
        escrow_account = PublicKey(pool["escrow"]) 
        escrow_mint_account = PublicKey(pool["escrow_mint"]) 
        long_token_mint_account = PublicKey(pool["long_mint"]) 
        short_token_mint_account = PublicKey(pool["short_mint"]) 
        buyer_account = buyer.public_key()
        seller_account = seller.public_key()
        token_account = PublicKey(TOKEN_PROGRAM_ID)
        escrow_owner_account = PublicKey.find_program_address(
            [bytes(long_token_mint_account), bytes(short_token_mint_account), bytes(token_account), bytes(PublicKey(BINARY_OPTION_PROGRAM_ID))],
            PublicKey(BINARY_OPTION_PROGRAM_ID),
        )[0]
        # Transaction
        tx = Transaction()
        atas = []
        for acct in [buyer_account, seller_account]:
            acct_atas = []
            for mint_account in (long_token_mint_account, short_token_mint_account, escrow_mint_account):
                token_pda_address = get_associated_token_address(acct, mint_account)
                associated_token_account_info = client.get_account_info(token_pda_address)
                account_info = associated_token_account_info['result']['value']
                if account_info is not None: 
                    account_state = ACCOUNT_LAYOUT.parse(base64.b64decode(account_info['data'][0])).state
                else:
                    account_state = 0
                if account_state == 0:
                    msg += f" | Creating PDA: {token_pda_address}"
                    associated_token_account_ix = create_associated_token_account(
                        payer=source_account.public_key(),
                        owner=acct,
                        mint=mint_account,
                    )
                    tx = tx.add(associated_token_account_ix)
                else:
                    msg += f" | Fetched PDA: {token_pda_address}"
                acct_atas.append(token_pda_address)
            atas.append(acct_atas)
        trade_ix = trade_instruction(
            pool_account,
            escrow_account,
            long_token_mint_account,
            short_token_mint_account,
            buyer_account,
            seller_account,
            atas[0][2],
            atas[1][2],
            atas[0][0],
            atas[0][1],
            atas[1][0],
            atas[1][1],
            escrow_owner_account,
            token_account,
            int(size),
            int(buyer_price),
            int(seller_price),
        )
        tx = tx.add(trade_ix)
        # Send request
        try:
            response = client.send_transaction(tx, *signers, opts=types.TxOpts(skip_confirmation=skip_confirmation))
            return json.dumps(
                {
                    'status': HTTPStatus.OK,
                    'msg': msg + f" | Trade successful",
                    'tx': response.get('result') if skip_confirmation else response['result']['transaction']['signatures'],
                }
            )
        except Exception as e:
            msg += f" | ERROR: Encountered exception while attempting to send transaction: {e}"
            raise(e)

    def settle(self, api_endpoint, pool_account, winning_mint, skip_confirmation=True):
        msg = ""
        client = Client(api_endpoint)
        msg += "Initialized client"
        # Create account objects
        source_account = Account(self.private_key)
        # Signers
        signers = [source_account]
        # List non-derived accounts
        pool_account = PublicKey(pool_account) 
        winning_mint_account = PublicKey(winning_mint) 
        tx = Transaction()
        settle_ix = settle_instruction(
            pool_account,
            winning_mint_account,
            source_account.public_key(),
        )
        tx = tx.add(settle_ix)
        # Send request
        try:
            response = client.send_transaction(tx, *signers, opts=types.TxOpts(skip_confirmation=skip_confirmation))
            return json.dumps(
                {
                    'status': HTTPStatus.OK,
                    'msg': msg + f" | Settle successful, winner: {str(winning_mint_account)}",
                    'tx': response.get('result') if skip_confirmation else response['result']['transaction']['signatures'],
                }
            )
        except Exception as e:
            msg += f" | ERROR: Encountered exception while attempting to send transaction: {e}"
            raise(e)

    def collect(self, api_endpoint, pool_account, collector, skip_confirmation=True):
        msg = ""
        client = Client(api_endpoint)
        msg += "Initialized client"
        signers = [Account(self.private_key)]
        pool = self.load_binary_option(api_endpoint, pool_account)
        pool_account = PublicKey(pool_account) 
        collector_account = PublicKey(collector)
        escrow_account = PublicKey(pool["escrow"]) 
        escrow_mint_account = PublicKey(pool["escrow_mint"]) 
        long_token_mint_account = PublicKey(pool["long_mint"]) 
        short_token_mint_account = PublicKey(pool["short_mint"]) 
        token_account = PublicKey(TOKEN_PROGRAM_ID)
        escrow_authority_account = PublicKey.find_program_address(
            [bytes(long_token_mint_account), bytes(short_token_mint_account), bytes(token_account), bytes(PublicKey(BINARY_OPTION_PROGRAM_ID))],
            PublicKey(BINARY_OPTION_PROGRAM_ID),
        )[0]
        # Transaction
        tx = Transaction()
        atas = []
        for mint_account in (long_token_mint_account, short_token_mint_account, escrow_mint_account):
            token_pda_address = get_associated_token_address(collector_account, mint_account)
            associated_token_account_info = client.get_account_info(token_pda_address)
            account_info = associated_token_account_info['result']['value']
            if account_info is not None: 
                account_state = ACCOUNT_LAYOUT.parse(base64.b64decode(account_info['data'][0])).state
            else:
                account_state = 0
            if account_state == 0:
                msg += f" | Error Fetching PDA: {token_pda_address}"
                raise Exception()
            else:
                msg += f" | Fetched PDA: {token_pda_address}"
            atas.append(token_pda_address)
        collect_ix = collect_instruction(
            pool_account,
            collector_account,
            atas[0],
            atas[1],
            atas[2],
            long_token_mint_account,
            short_token_mint_account,
            escrow_account,
            escrow_authority_account,
            token_account,
        )
        tx = tx.add(collect_ix) 
        try:
            response = client.send_transaction(tx, *signers, opts=types.TxOpts(skip_confirmation=skip_confirmation))
            return json.dumps(
                {
                    'status': HTTPStatus.OK,
                    'msg': msg + f" | Collect successful",
                    'tx': response.get('result') if skip_confirmation else response['result']['transaction']['signatures'],
                }
            )
        except Exception as e:
            msg += f" | ERROR: Encountered exception while attempting to send transaction: {e}"
            print(msg)
            raise(e)        


    def load_binary_option(self, api_endpoint, pool_account):
        client = Client(api_endpoint)
        try:
            pool_data = base64.b64decode(client.get_account_info(pool_account)['result']['value']['data'][0])
        except Exception as e:
            return json.dumps(
                {
                    'status': HTTPStatus.BAD_REQUEST,
                    'msg': str(e),
                }
            )
        pubkey = 'B' * 32
        raw_bytes = struct.unpack(f"<BQ?{pubkey}{pubkey}{pubkey}{pubkey}{pubkey}{pubkey}", pool_data)
        i = 0
        pool = {}
        pool["decimals"] = raw_bytes[i] 
        i += 1
        pool["circulation"] = raw_bytes[i] 
        i += 1
        pool["settled"] = raw_bytes[i] 
        i += 1
        pool["escrow_mint"] = base58.b58encode(bytes(raw_bytes[i:i+32])).decode('ascii')
        i += 32
        pool["escrow"] = base58.b58encode(bytes(raw_bytes[i:i+32])).decode('ascii')
        i += 32
        pool["long_mint"] = base58.b58encode(bytes(raw_bytes[i:i+32])).decode('ascii')
        i += 32
        pool["short_mint"] = base58.b58encode(bytes(raw_bytes[i:i+32])).decode('ascii')
        i += 32
        pool["owner"] = base58.b58encode(bytes(raw_bytes[i:i+32])).decode('ascii')
        i += 32
        pool["winning_side"] = base58.b58encode(bytes(raw_bytes[i:i+32])).decode('ascii')
        i += 32
        return pool

    def topup(self, api_endpoint, to, amount=None, skip_confirmation=True):
        """
        Send a small amount of native currency to the specified wallet to handle gas fees. Return a status flag of success or fail and the native transaction data.
        """
        msg = ""
        try:
            # Connect to the api_endpoint
            client = Client(api_endpoint)
            msg += "Initialized client"
            # List accounts 
            sender_account = Account(self.private_key)
            dest_account = PublicKey(to)
            msg += " | Gathered accounts"
            # List signers
            signers = [sender_account]
            # Start transaction
            tx = Transaction()
            # Determine the amount to send 
            try:
                if amount is None:
                    min_rent_reseponse = client.get_minimum_balance_for_rent_exemption(ACCOUNT_LAYOUT.sizeof())
                    lamports = min_rent_reseponse["result"]
                else:
                    lamports = int(amount)
                msg += f" | Fetched lamports: {lamports * 1e-9} SOL"
            except Exception as e:
                msg += " | ERROR: couldn't process lamports" 
                raise(e)
            # Generate transaction
            transfer_ix = transfer(TransferParams(from_pubkey=sender_account.public_key(), to_pubkey=dest_account, lamports=lamports))
            tx = tx.add(transfer_ix)
            msg += f" | Transferring funds"
            # Send request
            try:
                response = client.send_transaction(tx, *signers, opts=types.TxOpts(skip_confirmation=skip_confirmation))
                return json.dumps(
                    {
                        'status': HTTPStatus.OK,
                        'msg': f"Successfully sent {lamports * 1e-9} SOL to {to}",
                        'tx': response.get('result') if skip_confirmation else response['result']['transaction']['signatures'],
                    }
                )
            except Exception as e:
                msg += f" | ERROR: Encountered exception while attempting to send transaction: {e}"
                raise(e)
        except Exception as e:
            return json.dumps(
                {
                    'status': HTTPStatus.BAD_REQUEST,
                    'msg': msg,
                }
            )
            
    def mint_to(self, api_endpoint, pool_account, dest, amount, skip_confirmation=True):
        msg = ""
        client = Client(api_endpoint)
        msg += "Initialized client"
        # Create account objects
        source_account = Account(self.private_key)
        signers = [source_account]
        pool = self.load_binary_option(api_endpoint, pool_account)
        # List non-derived accounts
        pool_account = PublicKey(pool_account) 
        dest_account = PublicKey(dest)
        escrow_mint_account = PublicKey(pool["escrow_mint"]) 
        mint_authority_account = source_account.public_key()
        payer_account = source_account.public_key()
        token_account = PublicKey(TOKEN_PROGRAM_ID)
        tx = Transaction()
        token_pda_address = get_associated_token_address(dest_account, escrow_mint_account)
        associated_token_account_ix = create_associated_token_account(
            payer=payer_account,
            owner=dest_account,
            mint=escrow_mint_account,
        )
        tx = tx.add(associated_token_account_ix)
        mint_to_ix = mint_to(
            MintToParams(
                program_id=token_account,
                mint=escrow_mint_account,
                dest=token_pda_address,
                mint_authority=mint_authority_account,
                amount=int(amount),
                signers=[mint_authority_account],
            )
        )
        tx = tx.add(mint_to_ix) 
        # Send request
        try:
            response = client.send_transaction(tx, *signers, opts=types.TxOpts(skip_confirmation=skip_confirmation))
            return json.dumps(
                {
                    'status': HTTPStatus.OK,
                    'msg': msg + f" | MintTo {dest} successful",
                    'tx': response.get('result') if skip_confirmation else response['result']['transaction']['signatures'],
                }
            )
        except Exception as e:
            msg += f" | ERROR: Encountered exception while attempting to send transaction: {e}"
            raise(e)
