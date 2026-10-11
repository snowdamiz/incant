#include "imports.h"
static uint32_t invoke(struct w2c_incant__v1* host,uint32_t op,uint64_t e,uint32_t a,uint32_t b,uint32_t c,uint32_t d) {
  int32_t out=0;
  /* Rust returns normally before this C frame can trap. No longjmp crosses Rust. */
  int32_t status=host->callback(host->context,host->memory->data,host->memory->size,op,(int64_t)e,(int32_t)a,(int32_t)b,(int32_t)c,(int32_t)d,&out);
  if(status) wasm_rt_trap(WASM_RT_TRAP_UNREACHABLE);
  return (uint32_t)out;
}
#include "generated.inc"
