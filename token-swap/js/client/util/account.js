export async function loadAccount(connection, address, programId) {
    const accountInfo = await connection.getAccountInfo(address);
    if (accountInfo === null) {
        throw new Error('Failed to find account');
    }
    if (!accountInfo.owner.equals(programId)) {
        throw new Error(`Invalid owner: ${JSON.stringify(accountInfo.owner)}`);
    }
    return Buffer.from(accountInfo.data);
}
