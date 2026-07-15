/* Smoke test / usage example for the Tavra C API. Exercises parse/format,
 * binary encode/decode, and envelope seal/open (key + password modes,
 * with a signature). Exits nonzero on any failure. */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "tavra.h"

static void die(const char *step) {
    const char *msg = tav_last_error();
    fprintf(stderr, "FAIL at %s: %s\n", step, msg ? msg : "(no error message)");
    exit(1);
}

static void check(int32_t rc, const char *step) {
    if (rc != 0) {
        die(step);
    }
}

int main(void) {
    /* --- parse / format --- */
    const char *src = "name = \"demo\"\nport = 8080\n";
    TavValue *value = NULL;
    check(tav_parse((const uint8_t *)src, strlen(src), &value), "tav_parse");

    uint8_t *formatted = NULL;
    size_t formatted_len = 0;
    check(tav_format(value, &formatted, &formatted_len), "tav_format");
    printf("formatted (%zu bytes):\n%.*s\n", formatted_len, (int)formatted_len, formatted);
    tav_free_bytes(formatted, formatted_len);

    /* --- binary round-trip --- */
    uint8_t *encoded = NULL;
    size_t encoded_len = 0;
    check(tav_encode(value, &encoded, &encoded_len), "tav_encode");
    printf("encoded to %zu bytes of .tavb\n", encoded_len);

    TavValue *decoded = NULL;
    check(tav_decode(encoded, encoded_len, &decoded), "tav_decode");
    tav_free_bytes(encoded, encoded_len);
    tav_value_free(decoded);

    /* --- envelope: key mode + signature --- */
    uint8_t key[32];
    check(tav_genkey(key), "tav_genkey");

    uint8_t secret[32];
    uint8_t public_key[32];
    check(tav_gensignkey(secret, public_key), "tav_gensignkey");

    uint8_t *sealed = NULL;
    size_t sealed_len = 0;
    check(tav_seal_key(value, key, true, secret, &sealed, &sealed_len), "tav_seal_key");
    printf("sealed (key+compress+sign) to %zu bytes of .tave\n", sealed_len);

    TavValue *opened = NULL;
    check(tav_open_key(sealed, sealed_len, key, public_key, &opened), "tav_open_key");
    tav_value_free(opened);

    /* wrong key must fail */
    uint8_t wrong_key[32];
    check(tav_genkey(wrong_key), "tav_genkey (wrong_key)");
    TavValue *should_fail = NULL;
    int32_t rc = tav_open_key(sealed, sealed_len, wrong_key, NULL, &should_fail);
    if (rc == 0) {
        fprintf(stderr, "FAIL: expected wrong-key open to fail, it succeeded\n");
        return 1;
    }
    printf("wrong-key open correctly failed: %s\n", tav_last_error());

    tav_free_bytes(sealed, sealed_len);

    /* --- envelope: password mode --- */
    const char *password = "correct horse battery staple";
    uint8_t *pw_sealed = NULL;
    size_t pw_sealed_len = 0;
    check(tav_seal_password(value, (const uint8_t *)password, strlen(password), false, NULL, &pw_sealed, &pw_sealed_len), "tav_seal_password");

    TavValue *pw_opened = NULL;
    check(tav_open_password(pw_sealed, pw_sealed_len, (const uint8_t *)password, strlen(password), NULL, &pw_opened), "tav_open_password");
    tav_value_free(pw_opened);
    tav_free_bytes(pw_sealed, pw_sealed_len);

    tav_value_free(value);

    printf("all checks passed\n");
    return 0;
}
