/**
 * @brief A program demonstrating cross program invocations
 */
#include <solana_sdk.h>

/// Amount of bytes of account data to allocate
#define SIZE 42

extern uint64_t do_invoke(SolParameters *params) {
  // As part of the program specification the first account is the system
  // program's executable account and the second is the account to allocate
  if (params->ka_num != 2) {
    return ERROR_NOT_ENOUGH_ACCOUNT_KEYS;
  }
  SolAccountInfo *system_program_info = &params->ka[0];
  SolAccountInfo *allocated_info = &params->ka[1];

  uint8_t seed[] = {'Y', 'o', 'u', ' ', 'p', 'a', 's', 's',
                    ' ', 'b', 'u', 't', 't', 'e', 'r'};
  const SolSignerSeed seeds[] = {{seed, SOL_ARRAY_SIZE(seed)},
                                 {&params->data[0], 1}};
  const SolSignerSeeds signers_seeds[] = {{seeds, SOL_ARRAY_SIZE(seeds)}};

  SolPubkey expected_allocated_key;
  if (SUCCESS != sol_create_program_address(seeds, SOL_ARRAY_SIZE(seeds),
                                            params->program_id,
                                            &expected_allocated_key)) {
    return ERROR_INVALID_INSTRUCTION_DATA;
  }
  if (!SolPubkey_same(&expected_allocated_key, allocated_info->key)) {
    return ERROR_INVALID_ARGUMENT;
  }

  SolAccountMeta arguments[] = {{allocated_info->key, true, true}};
  uint8_t data[4 + 8];            // Enough room for the Allocate instruction
  *(uint16_t *)data = 8;          // Allocate instruction enum value
  *(uint64_t *)(data + 4) = SIZE; // Size to allocate
  const SolInstruction instruction = {system_program_info->key, arguments,
                                      SOL_ARRAY_SIZE(arguments), data,
                                      SOL_ARRAY_SIZE(data)};
  return sol_invoke_signed(&instruction, params->ka, params->ka_num,
                           signers_seeds, SOL_ARRAY_SIZE(signers_seeds));
}

extern uint64_t entrypoint(const uint8_t *input) {
  SolAccountInfo accounts[2];
  SolParameters params = (SolParameters){.ka = accounts};

  if (!sol_deserialize(input, &params, SOL_ARRAY_SIZE(accounts))) {
    return ERROR_INVALID_ARGUMENT;
  }

  return do_invoke(&params);
}
