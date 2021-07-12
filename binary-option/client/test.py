import pandas as pd
from .binary_option import *
import time
from spl.token.client import Token


api_endpoint = "https://api.devnet.solana.com/"
balance_data = []

def await_confirmation(client, txn):
    elapsed_time = 0
    while elapsed_time < 30:
        sleep_time = 1
        time.sleep(sleep_time)
        resp = client.get_confirmed_transaction(txn)
        while 'result' not in resp:
            resp = client.get_confirmed_transaction(txn)
        if resp["result"]:
            break
        elapsed_time += sleep_time

def get_ata(pk, mint):
    try:
        token_pda_address = PublicKey.find_program_address(
            [bytes(PublicKey(pk)), bytes(PublicKey(TOKEN_PROGRAM_ID)), bytes(PublicKey(mint))],
            PublicKey(ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID),
        )[0]
        return get_account(str(token_pda_address))
    except:
        return None

def get_account(pk):
    c = Client(api_endpoint)
    data = base64.b64decode(c.get_account_info(pk)['result']['value']['data'][0])
    return ACCOUNT_LAYOUT.parse(data)

def update_and_print_state():
    pool_data = bp.load_binary_option(api_endpoint, pool)
    state = {}
    try:
        state["N"] = 1
        state["c"] = pool_data["circulation"] 
        state["e_A"] = get_account(pool_data["escrow"]).amount
        state["a1_LT"] = get_ata(str(a1.public_key()), pool_data['long_mint']).amount
        state["a1_ST"] = get_ata(str(a1.public_key()), pool_data['short_mint']).amount
        state["a1_A"] =  get_ata(str(a1.public_key()), pool_data['escrow_mint']).amount
        state["a2_LT"] = get_ata(str(a2.public_key()), pool_data['long_mint']).amount
        state["a2_ST"] = get_ata(str(a2.public_key()), pool_data['short_mint']).amount
        state["a2_A"] =  get_ata(str(a2.public_key()), pool_data['escrow_mint']).amount
        state["a3_LT"] = get_ata(str(a3.public_key()), pool_data['long_mint']).amount
        state["a3_ST"] = get_ata(str(a3.public_key()), pool_data['short_mint']).amount
        state["a3_A"] =  get_ata(str(a3.public_key()), pool_data['escrow_mint']).amount
    except:
        pass
    balance_data.append(state)
    print(pd.DataFrame(balance_data).fillna(0).astype(int))


account = Account()

bp = BinaryOption(
    {
        'PRIVATE_KEY': base58.b58encode(account.secret_key()).decode('ascii'),
        'PUBLIC_KEY': str(account.public_key()),
        'DECRYPTION_KEY': Fernet.generate_key(),
    }
)

client = Client(api_endpoint)
opts = types.TxOpts()
resp = {}
while 'result' not in resp:
    resp = client.request_airdrop(account.public_key(), int(1e10))
txn = resp['result']
await_confirmation(client, txn)

a1 = Account() 
a2 = Account()
a3 = Account()
ek1 = bp.cipher.encrypt(a1.secret_key())
ek2 = bp.cipher.encrypt(a2.secret_key())
ek3 = bp.cipher.encrypt(a3.secret_key())

tu1 = json.loads(bp.topup(api_endpoint, str(a1.public_key())))
print(tu1)
tu2 = json.loads(bp.topup(api_endpoint, str(a2.public_key())))
print(tu2)
tu3 = json.loads(bp.topup(api_endpoint, str(a3.public_key())))
print(tu3)


token = Token.create_mint(
    client,
    Account(bp.private_key),
    PublicKey(bp.public_key),
    0,
    PublicKey(TOKEN_PROGRAM_ID),
    PublicKey(bp.public_key),
    skip_confirmation=False,
)

mint = str(token.pubkey)

res = json.loads(bp.initialize(api_endpoint, mint, skip_confirmation=False))
print(res)

pool = res.get("binary_option")
print(bp.mint_to(api_endpoint, pool, str(a1.public_key()), 1e6, skip_confirmation=False))
print(bp.mint_to(api_endpoint, pool, str(a2.public_key()), 1e6, skip_confirmation=False))
print(bp.mint_to(api_endpoint, pool, str(a3.public_key()), 1e6, skip_confirmation=False))

pool_data = bp.load_binary_option(api_endpoint, pool)

print(bp.trade(api_endpoint, pool, ek1, ek2, 10, 30, 70, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek2, ek3, 1, 30, 70, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek3, ek1, 10, 40, 60, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek1, ek2, 2, 1, 99, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek2, ek1, 1, 50, 50, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek3, ek1, 1, 50, 50, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek3, ek1, 1, 50, 50, skip_confirmation=False))
update_and_print_state()
print(bp.trade(api_endpoint, pool, ek3, ek1, 1, 50, 50, skip_confirmation=False))
update_and_print_state()

long_mint = pool_data['long_mint']
print(bp.settle(api_endpoint, pool, long_mint, skip_confirmation=False))
print(bp.collect(api_endpoint, pool, a1.public_key(), skip_confirmation=False))
print(bp.collect(api_endpoint, pool, a2.public_key(), skip_confirmation=False))
print(bp.collect(api_endpoint, pool, a3.public_key(), skip_confirmation=False))
update_and_print_state()