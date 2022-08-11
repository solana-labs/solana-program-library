#!/usr/bin/env python3

import argparse
import subprocess
import json
import sys
import os
from datetime import datetime


def main():
    parser = argparse.ArgumentParser(description='Vaults metadata generator')

    parser.add_argument('-v',
                        '--vaults-file',
                        help='Output file path for vaults',
                        default='vaults.json')
    parser.add_argument('-t',
                        '--tokens-file',
                        help='Output file path for vault tokens',
                        default='tokens.json')
    parser.add_argument('-f',
                        '--farm-binaries-dir',
                        help='Path to farm binaries, e.g. ../target/release',
                        default='')
    parser.add_argument('-a',
                        '--vault-program-address',
                        help='Address of the vault program',
                        required=True)
    parser.add_argument('-p',
                        '--protocol',
                        help='Protocol',
                        choices=['RDM', 'ORC', 'SBR'],
                        required=True)
    args = parser.parse_args()

    vaults_out = open(args.vaults_file, 'w')
    tokens_out = open(args.tokens_file, 'w')
    vault_program = args.vault_program_address
    bin_dir = args.farm_binaries_dir
    protocol = args.protocol

    p = subprocess.Popen(os.path.join(bin_dir, 'solana-farm-client') +
                         ' list-all farm',
                         shell=True,
                         stdout=subprocess.PIPE)
    data = '['
    for line in p.stdout.readlines():
        if line[:4].decode("utf-8") != protocol + '.':
            continue
        farm = line.decode("utf-8").split(':')[0]
        if len('VT.' + protocol + '.STC.' + farm[4:]) >= 32:
            raise ValueError("Len exceeded " + farm)
        p2 = subprocess.Popen(os.path.join(bin_dir, 'solana-farm-ctrl') +
                              ' generate Vault ' + vault_program + ' ' +
                              protocol + '.STC.' + farm[4:] + ' VT.' +
                              protocol + '.STC.' + farm[4:],
                              shell=True,
                              stdout=subprocess.PIPE)
        for log in p2.stdout.readlines():
            data += log.decode("utf-8").rstrip('\n')
        if p2.wait() == 0:
            data += ','

    data = data[:-1]
    data += ']'

    parsed = json.loads(data)
    timestamp = datetime.now().isoformat()
    vaults_out.write(
        f'{{"name": "Solana Vaults List", "timestamp": "{timestamp}", "vaults":['
    )
    tokens_out.write(
        f'{{"name": "Solana Token List", "timestamp": "{timestamp}", "tokens":['
    )
    first_token = True
    first_vault = True
    for obj in parsed:
        if 'chainId' in obj:
            if not first_token:
                tokens_out.write(',\n')
            else:
                first_token = False
            tokens_out.write(json.dumps(obj, indent=2, sort_keys=False))
        else:
            if not first_vault:
                vaults_out.write(',\n')
            else:
                first_vault = False
            vaults_out.write(json.dumps(obj, indent=2, sort_keys=False))

    vaults_out.write(']}')
    tokens_out.write(']}')
    vaults_out.close()
    tokens_out.close()

    print('Done.')


if __name__ == '__main__':
    main()
