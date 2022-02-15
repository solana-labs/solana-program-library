// #include "logging.c"
// #include <criterion/criterion.h>

// Test(logging, sanity) {
//   uint8_t instruction_data[] = {10, 11, 12, 13, 14};
//   SolPubkey program_id = {.x = {
//                               1,
//                           }};
//   SolPubkey key = {.x = {
//                        2,
//                    }};
//   uint64_t lamports = 1;
//   uint8_t data[] = {0, 0, 0, 0};
//   SolAccountInfo accounts[] = {};
//   SolParameters params = {accounts, sizeof(accounts) /
//   sizeof(SolAccountInfo), instruction_data,
//                           sizeof(instruction_data), &program_id};

//   cr_assert(SUCCESS == logging(&params));
// }
