/**
 * @brief A program demonstrating the implementation of a custom heap
 */
#include <solana_sdk.h>

/// Start address of the memory region used for program heap.
#define HEAP_START_ADDRESS_ (uint64_t)0x300000000
/// Length of the heap memory region used for program heap.
#define HEAP_LENGTH_ (uint64_t)(32 * 1024)

typedef struct BumpAllocator {
  uint64_t start;
  uint64_t size;
} BumpAllocator;
void *alloc(BumpAllocator *self, uint64_t size, uint64_t align) {
  uint64_t *pos_ptr = (uint64_t *)self->start;

  uint64_t pos = *pos_ptr;
  if (pos == 0) {
    // First time, set starting position
    pos = self->start + self->size;
  }
  if (pos < size) {
    pos = 0;
  } else {
    pos = pos - size;
  }
  pos &= ~(align - 1);
  if (pos < self->start + sizeof(uint8_t)) {
    return NULL;
  }
  *pos_ptr = pos;
  return (void *)pos;
}
void dealloc(BumpAllocator *self, void *ptr) {
  // I'm a bump allocator, I don't free
}

extern uint64_t entrypoint(const uint8_t *input) {
  SolAccountInfo accounts[2];
  SolParameters params = (SolParameters){.ka = accounts};

  if (!sol_deserialize(input, &params, SOL_ARRAY_SIZE(accounts))) {
    return ERROR_INVALID_ARGUMENT;
  }

  BumpAllocator heap = {HEAP_START_ADDRESS_, HEAP_LENGTH_};
  sol_assert(0 != alloc(&heap, 1, sizeof(uint64_t)));

  return SUCCESS;
}
