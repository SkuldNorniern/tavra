/* Tavra C API — core round-trip only: parse/format .tav text, encode/
 * decode .tavb binary, seal/open .tave envelopes.
 *
 * Hand-maintained: cbindgen (as of 0.27) does not recognize Rust 2024's
 * `#[unsafe(no_mangle)]` attribute form, so it can't auto-generate this
 * header from tavra-c's source. Keep this file in sync with the sources
 * under src/ by hand until that's fixed upstream.
 *
 * Every fallible function returns 0 on success, nonzero on error — call
 * tav_last_error() for a message on failure. Every function that returns
 * an owned buffer or handle documents its matching free function.
 */

#ifndef TAVRA_H
#define TAVRA_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a parsed/decoded document root. */
typedef struct TavValue TavValue;

/* ---- Errors ---- */

/* Returns the last error message set on this thread, or NULL if none.
 * Valid until the next tavra-c call on this thread. */
const char *tav_last_error(void);

/* ---- Memory ---- */

/* Frees a buffer returned by tav_format, tav_encode, or tav_seal_*.
 * ptr/len must be exactly what was returned together. NULL is a no-op. */
void tav_free_bytes(uint8_t *ptr, size_t len);

/* Frees a handle returned by tav_parse, tav_decode, or tav_open_*.
 * NULL is a no-op. */
void tav_value_free(TavValue *value);

/* ---- Text (.tav) ---- */

/* Parses UTF-8 .tav text (data/len, not required to be NUL-terminated)
 * into a value, written to *out_value on success (free with
 * tav_value_free). Returns 0 on success, nonzero on error. */
int32_t tav_parse(const uint8_t *data, size_t len, TavValue **out_value);

/* Formats a value as canonical .tav text, written to out_data/out_len
 * on success (free with tav_free_bytes). Returns 0 on success, nonzero
 * on error. */
int32_t tav_format(const TavValue *value, uint8_t **out_data, size_t *out_len);

/* ---- Binary (.tavb) ---- */

/* Encodes a value as canonical .tavb (magic + version + value), written
 * to out_data/out_len on success (free with tav_free_bytes). Returns 0
 * on success, nonzero on error. */
int32_t tav_encode(const TavValue *value, uint8_t **out_data, size_t *out_len);

/* Decodes a .tavb document, rejecting any non-canonical or malformed
 * input, written to *out_value on success (free with tav_value_free).
 * Returns 0 on success, nonzero on error. */
int32_t tav_decode(const uint8_t *data, size_t len, TavValue **out_value);

/* ---- Envelope (.tave) ---- */

/* Seals a value unencrypted, optionally compressed and/or signed.
 * sign_key is NULL or 32 bytes (Ed25519 signing seed). Output written to
 * out_data/out_len on success (free with tav_free_bytes). */
int32_t tav_seal_none(const TavValue *value, bool compress, const uint8_t *sign_key, uint8_t **out_data, size_t *out_len);

/* Seals a value with a raw 32-byte XChaCha20-Poly1305 key. */
int32_t tav_seal_key(const TavValue *value, const uint8_t *key, bool compress, const uint8_t *sign_key, uint8_t **out_data, size_t *out_len);

/* Seals a value with an Argon2id-derived key from a password
 * (password/password_len, not required to be NUL-terminated). */
int32_t tav_seal_password(const TavValue *value, const uint8_t *password, size_t password_len, bool compress, const uint8_t *sign_key, uint8_t **out_data, size_t *out_len);

/* Opens an unencrypted .tave document. verify_key is NULL or 32 bytes
 * (Ed25519 public key) — if non-NULL, a present signature is checked.
 * Output written to *out_value on success (free with tav_value_free). */
int32_t tav_open_none(const uint8_t *data, size_t len, const uint8_t *verify_key, TavValue **out_value);

/* Opens a .tave document encrypted with a raw 32-byte key. */
int32_t tav_open_key(const uint8_t *data, size_t len, const uint8_t *key, const uint8_t *verify_key, TavValue **out_value);

/* Opens a .tave document encrypted with an Argon2id-derived password key. */
int32_t tav_open_password(const uint8_t *data, size_t len, const uint8_t *password, size_t password_len, const uint8_t *verify_key, TavValue **out_value);

/* ---- Keys ---- */

/* Generates a fresh 32-byte XChaCha20-Poly1305 key into out (32 writable
 * bytes). Always succeeds. */
int32_t tav_genkey(uint8_t *out);

/* Generates a fresh Ed25519 signing key seed and its matching public key
 * into out_secret/out_public (32 writable bytes each). Always succeeds. */
int32_t tav_gensignkey(uint8_t *out_secret, uint8_t *out_public);

#ifdef __cplusplus
}
#endif

#endif /* TAVRA_H */
