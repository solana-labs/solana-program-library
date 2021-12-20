/**
 * @brief A program demonstrating the transfer of lamports
 */
#include <solana_sdk.h>

extern uint64_t transfer(SolParameters *params) {
  // As part of the program specification the first account is the source
  // account and the second is the destination account
  if (params->ka_num != 2) {
    return ERROR_NOT_ENOUGH_ACCOUNT_KEYS;
  }
  SolAccountInfo *source_info = &params->ka[0];
  SolAccountInfo *destination_info = &params->ka[1];

  // Withdraw five lamports from the source
  *source_info->lamports -= 5;
  // Deposit five lamports into the destination
  *destination_info->lamports += 5;

  return SUCCESS;
}

extern uint64_t entrypoint(const uint8_t *input) {
  SolAccountInfo accounts[2];
  SolParameters params = (SolParameters){.ka = accounts};

  if (!sol_deserialize(input, &params, SOL_ARRAY_SIZE(accounts))) {
    return ERROR_INVALID_ARGUMENT;
  }

  return transfer(&params);
}
