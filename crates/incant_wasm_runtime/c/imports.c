#include "imports.h"
static uint32_t invoke(struct w2c_incant__v1* host,uint32_t op,uint64_t e,uint32_t a,uint32_t b,uint32_t c,uint32_t d) {
  int32_t out=0;
  /* Rust returns normally before this C frame can trap. No longjmp crosses Rust. */
  int32_t status=host->callback(host->context,host->memory->data,host->memory->size,op,(int64_t)e,(int32_t)a,(int32_t)b,(int32_t)c,(int32_t)d,&out);
  if(status) wasm_rt_trap(WASM_RT_TRAP_UNREACHABLE);
  return (uint32_t)out;
}
uint32_t w2c_incant__v1_chunk_count(struct w2c_incant__v1* h,uint64_t e){return invoke(h,0,e,0,0,0,0);}
uint32_t w2c_incant__v1_chunk_len(struct w2c_incant__v1* h,uint64_t e,uint32_t a){return invoke(h,1,e,a,0,0,0);}
uint32_t w2c_incant__v1_chunk_flags(struct w2c_incant__v1* h,uint64_t e,uint32_t a){return invoke(h,2,e,a,0,0,0);}
#define COPY(name,op) void w2c_incant__v1_##name(struct w2c_incant__v1* h,uint64_t e,uint32_t a,uint32_t b){invoke(h,op,e,a,b,0,0);}
COPY(read_ids,3)
COPY(read_transforms,4)
COPY(write_transforms,5)
COPY(read_velocities,6)
COPY(write_velocities,7)
COPY(set_velocity,10)
COPY(set_parent,11)
void w2c_incant__v1_spawn(struct w2c_incant__v1* h,uint64_t e,uint32_t a,uint32_t b,uint32_t c,uint32_t d){invoke(h,8,e,a,b,c,d);}
void w2c_incant__v1_despawn(struct w2c_incant__v1* h,uint64_t e,uint32_t a){invoke(h,9,e,a,0,0,0);}

uint32_t w2c_incant__input__v1_snapshot_len(struct w2c_incant__input__v1* h,uint64_t e){return invoke(&h->core,12,e,0,0,0,0);}
uint32_t w2c_incant__input__v1_read_snapshot(struct w2c_incant__input__v1* h,uint64_t e,uint32_t p,uint32_t capacity){return invoke(&h->core,13,e,p,capacity,0,0);}
uint32_t w2c_incant__flipbook__v1_snapshot_len(struct w2c_incant__flipbook__v1* h,uint64_t e){return invoke(&h->core,14,e,0,0,0,0);}
uint32_t w2c_incant__flipbook__v1_read_snapshot(struct w2c_incant__flipbook__v1* h,uint64_t e,uint32_t p,uint32_t capacity){return invoke(&h->core,15,e,p,capacity,0,0);}
void w2c_incant__flipbook__v1_control(struct w2c_incant__flipbook__v1* h,uint64_t e,uint32_t id,uint32_t kind,uint32_t frame){invoke(&h->core,16,e,id,kind,frame,0);}
