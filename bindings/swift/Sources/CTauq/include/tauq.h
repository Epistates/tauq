#ifndef TAUQ_H
#define TAUQ_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Parse Tauq source to JSON string.
 * Returns NULL on error.
 * Caller must free result with tauq_free_string.
 */
char* tauq_to_json(const char* input);

/**
 * Execute Tauq Query (TQQ) source to JSON string.
 * Returns NULL on error.
 * Caller must free result with tauq_free_string.
 * @param safe_mode If true, disables !run, !pipe, !emit directives.
 */
char* tauq_exec_query(const char* input, bool safe_mode);

/**
 * Minify Tauq source to single-line Tauq string.
 * Returns NULL on error.
 * Caller must free result with tauq_free_string.
 */
char* tauq_minify(const char* input);

/**
 * Format JSON string to Tauq.
 * Returns NULL on error.
 * Caller must free result with tauq_free_string.
 */
char* json_to_tauq_c(const char* input);

/**
 * Free string returned by tauq functions.
 */
void tauq_free_string(char* s);

#ifdef __cplusplus
}
#endif

#endif
