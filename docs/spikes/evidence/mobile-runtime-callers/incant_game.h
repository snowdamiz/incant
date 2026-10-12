#ifndef INCANT_GAME_H
#define INCANT_GAME_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* UTF-8 JSON, at most 65536 bytes; readable and stable for the whole call.
 * Synchronous offline CPU run; writes a fresh output directory.
 * 0=passed, 1=player failed including its I/O (result.json), 2=invalid request,
 * 3=directory/result publication I/O error. */
int32_t incant_game_run_v1(const uint8_t *request, size_t length);
#ifdef __cplusplus
}
#endif
#endif
