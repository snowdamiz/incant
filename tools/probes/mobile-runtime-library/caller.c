/* The caller owns a bounded stable request allocation for the synchronous ABI. */
#include "incant_game.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) return 64;
    int32_t status;
    if (strcmp(argv[1], "--null") == 0) {
        status = incant_game_run_v1(NULL, 1);
    } else {
        FILE *file = fopen(argv[1], "rb");
        if (!file) return 65;
        uint8_t *bytes = malloc(65537);
        if (!bytes) { fclose(file); return 66; }
        size_t length = fread(bytes, 1, 65537, file);
        int failed = ferror(file);
        fclose(file);
        if (failed) { free(bytes); return 67; }
        status = incant_game_run_v1(bytes, length);
        free(bytes);
    }
    printf("{\"abi_status\":%d}\n", status);
    return status;
}
