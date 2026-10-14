#ifndef INCANT_APPLE_H
#define INCANT_APPLE_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Main-thread-only CAMetalLayer host. Retain layer until close returns.
 * The handle is opaque and process-unique. All callbacks must carry their
 * original handle; a touch also carries the delivery epoch captured at Down.
 * Status: 0 success, 1 invalid/capacity, 2 reentrant/busy, 3 stale/closed,
 * 4 native failure, 5 wrong creation thread, 6 identity exhausted.
 * Creation writes zero on failure. Path is absolute UTF-8, 1..4096 bytes.
 * Native errors retire gameplay; close still releases the retained surface.
 * The application must not mutate/destroy a layer during a native callback. */
int32_t incant_apple_open_v1(void *layer, const uint8_t *path, size_t length,
    uint32_t width, uint32_t height, float logical_width, float logical_height,
    uint64_t *handle);
int32_t incant_apple_close_v1(uint64_t handle);
int32_t incant_apple_active_v1(uint64_t handle, int32_t active);
int32_t incant_apple_resize_v1(uint64_t handle, uint32_t width, uint32_t height,
    float logical_width, float logical_height);
int32_t incant_apple_frame_v1(uint64_t handle);
uint64_t incant_apple_epoch_v1(uint64_t handle);
/* Phase: 0 down, 1 move, 2 up, 3 cancel. IDs are slots 0..15; coordinates
 * are logical points. The caller maps native touch identity to stable slots. */
int32_t incant_apple_touch_v1(uint64_t handle, uint64_t epoch, uint32_t id,
    uint32_t phase, float x, float y);
/* Copies <=4096 error bytes, no NUL; returns bytes written. */
size_t incant_apple_error_v1(uint8_t *output, size_t capacity);
#ifdef __cplusplus
}
#endif
#endif
