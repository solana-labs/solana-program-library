// goes away when the sdk incorporates it
int printf(const char * restrictformat, ... );

#include "custom-heap.c"
#include <criterion/criterion.h>

bool is_aligned(void *ptr, uint64_t align) {
  if (0 == ((uint64_t)ptr & (align - 1))) {
    return true;
  }
  return false;
}

uint64_t test_heap(uint64_t start, uint64_t size) {

  // alloc the entire
  {
    BumpAllocator heap = {start, size};
    for (int i = 0; i < size - sizeof(uint8_t); i++) {
      void *ptr = alloc(&heap, 1, sizeof(uint8_t));
      sol_assert(NULL != ptr);
      sol_assert(ptr == (void *)(start + size - 1 - i));
    }
    sol_assert(NULL == alloc(&heap, 1, sizeof(uint8_t)));
  }
  // check alignment
  {
    sol_memset((void *)start, 0, size);
    BumpAllocator heap = {start, size};
    void *ptr = NULL;
    ptr = alloc(&heap, 1, sizeof(uint16_t));
    sol_assert(is_aligned(ptr, sizeof(uint16_t)));
    ptr = alloc(&heap, 1, sizeof(uint32_t));
    sol_assert(is_aligned(ptr, sizeof(uint32_t)));
    ptr = alloc(&heap, 1, sizeof(uint64_t));
    sol_assert(is_aligned(ptr, sizeof(uint64_t)));
    ptr = alloc(&heap, 1, 64);
    sol_assert(is_aligned(ptr, 64));
  }
  // alloc entire block (minus the pos ptr)
  {
    sol_memset((void *)start, 0, size);
    BumpAllocator heap = {start, size};
    void *ptr = alloc(&heap, size - 8, sizeof(uint8_t));
    sol_assert(ptr != NULL);
  }

  return SUCCESS;
}

Test(custom_heap, sanity) {
  uint8_t heap[128] = {0};
  cr_assert(SUCCESS == test_heap((uint64_t)heap, 128));
}
