/* Incant's C-only trap bridge. Licensed under the repository license. */
#include "wasm-rt-impl.h"
#include <stdint.h>
typedef int32_t (*incant_import)(void*,uint8_t*,size_t,uint32_t,int64_t,int32_t,int32_t,int32_t,int32_t,int32_t*);
struct w2c_incant__v1 { incant_import callback; void* context; wasm_rt_memory_t* memory; };
struct w2c_incant__input__v1 { struct w2c_incant__v1 core; };
/* Exceptions are rejected before translation; no exception unwind target is
 * needed. The ordinary trap target remains wholly inside the C facade. */
static inline void incant_trap_prepare(void) {
  WASM_RT_SAVE_STACK_DEPTH();
  g_wasm_rt_jmp_buf.initialized=true;
}
/* Keep setjmp directly in a controlling-expression comparison, not a variable
 * initializer or comma expression. Exceptions/reentrant guest calls are absent. */
#define incant_try() WASM_RT_SETJMP_TRAP_SETBUF(g_wasm_rt_jmp_buf.buffer)
