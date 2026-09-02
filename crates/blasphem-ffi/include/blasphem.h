/* C ABI over the blasphem bytes-in engine. See crates/blasphem-ffi/src/lib.rs. */
#ifndef BLASPHEM_H
#define BLASPHEM_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct blasphem_builder blasphem_builder;
typedef struct blasphem_engine blasphem_engine;

typedef struct blasphem_judgement {
  bool safe;
  double score;
  char *locale;   /* lowercase code or NULL; free with blasphem_judgement_free */
  char *grawlix;  /* masked text or NULL; free with blasphem_judgement_free */
} blasphem_judgement;

blasphem_builder *blasphem_builder_new(bool detect_language, bool grawlix);

/* Returns 0 on success, 1 on failure with the message in blasphem_builder_error. Digest strings may be NULL. */
int32_t blasphem_builder_add(blasphem_builder *builder, const char *locale,
                             const uint8_t *pack, size_t pack_len, const char *pack_sha256,
                             const uint8_t *detect, size_t detect_len, const char *detect_sha256);

/* On success consumes the builder. On failure returns NULL and leaves the builder alive
   with the message in blasphem_builder_error; free it with blasphem_builder_free. */
blasphem_engine *blasphem_builder_build(blasphem_builder *builder);

/* The builder's last failure or NULL. Safe across threads; valid until the next call on it. */
const char *blasphem_builder_error(const blasphem_builder *builder);
void blasphem_builder_free(blasphem_builder *builder);

blasphem_judgement blasphem_engine_judge(const blasphem_engine *engine, const char *text);
size_t blasphem_engine_locale_count(const blasphem_engine *engine);
char *blasphem_engine_locale(const blasphem_engine *engine, size_t index);
void blasphem_judgement_free(blasphem_judgement judgement);
void blasphem_text_free(char *text);
void blasphem_engine_free(blasphem_engine *engine);

/* Bytes for pointer arguments when the host cannot share memory with this library, such as a
   WebAssembly runtime. Aligned to 8. NULL when len is 0 or memory is exhausted. Free with
   blasphem_free and the same len. */
uint8_t *blasphem_alloc(size_t len);
void blasphem_free(uint8_t *pointer, size_t len);

/* Thread-local fallback. Prefer blasphem_builder_error. Valid until the next failing call on the same thread. */
const char *blasphem_last_error(void);

#ifdef __cplusplus
}
#endif

#endif
