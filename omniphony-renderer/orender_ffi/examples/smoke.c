/* Minimal link/ABI smoke test for liborender.
 *
 * Build & run (from the workspace root):
 *   cc -I orender_ffi/include orender_ffi/examples/smoke.c \
 *      -L target/debug -lorender -o /tmp/orender_smoke
 *   LD_LIBRARY_PATH=target/debug /tmp/orender_smoke
 *
 * Verifies the header compiles, the symbols link, version reporting works, and
 * the panic-safe boundary returns errors (not crashes) on bad input. It does
 * NOT exercise real decoding — that needs a TrueHD file + the truehd bridge .so
 * and is the job of the full parity test.
 */
#include <stdio.h>
#include "orender.h"

int main(void) {
    printf("orender ABI %u.%u\n", orender_version_major(), orender_version_minor());

    /* bridge_path is required, so creation must fail gracefully (NULL),
     * exercising the catch_unwind boundary rather than crashing. */
    OrenderConfig cfg = {0};
    cfg.sample_rate = 48000;
    OrenderRenderer *r = orender_create(&cfg);
    if (r != NULL) {
        printf("FAIL: got a renderer without a bridge_path\n");
        orender_destroy(r);
        return 1;
    }
    printf("orender_create(no bridge) -> NULL (expected)\n");

    /* NULL-safety on the remaining entry points. */
    if (orender_channel_count(NULL) != 0) { printf("FAIL: channel_count(NULL)\n"); return 2; }
    if (orender_is_spatial(NULL) >= 0)    { printf("FAIL: is_spatial(NULL)\n"); return 3; }
    orender_reset(NULL);
    orender_destroy(NULL);
    if (orender_create(NULL) != NULL)     { printf("FAIL: create(NULL)\n"); return 4; }

    printf("smoke OK\n");
    return 0;
}
