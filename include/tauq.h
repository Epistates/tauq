#ifndef TAUQ_H
#define TAUQ_H

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Get the last error message.
 * If buffer is NULL, returns the length of the error message.
 * If buffer is not NULL, copies up to size bytes into buffer, ensuring null-termination.
 * Returns the number of bytes copied (excluding null terminator).
 */
size_t tauq_get_last_error(char* buffer, size_t size);

/**
 * Parse Tauq source to JSON string.
 * Returns NULL on error. Use tauq_get_last_error to retrieve error details.
 * Caller must free result with tauq_free_string.
 */
char* tauq_to_json(const char* input);

/**
 * Execute Tauq Query (TQQ) source to JSON string.
 * Returns NULL on error. Use tauq_get_last_error to retrieve error details.
 * Caller must free result with tauq_free_string.
 * @param safe_mode If true, disables !run, !pipe, !emit, !import, !json, !read, !env directives.
 */
char* tauq_exec_query(const char* input, bool safe_mode);

/**
 * Minify Tauq source to single-line Tauq string.
 * Returns NULL on error. Use tauq_get_last_error to retrieve error details.
 * Caller must free result with tauq_free_string.
 */
char* tauq_minify(const char* input);

/**
 * Format JSON string to Tauq.
 * Returns NULL on error. Use tauq_get_last_error to retrieve error details.
 * Caller must free result with tauq_free_string.
 */
char* json_to_tauq_c(const char* input);

/**
 * Free string returned by tauq functions.
 */
void tauq_free_string(char* s);

/**
 * Convert JSON or Tauq string to TBF bytes.
 * Returns pointer to bytes. Sets *out_len to length of bytes.
 * Returns NULL on error.
 * Caller must free result with tauq_free_buffer(ptr, len).
 */
unsigned char* tauq_to_tbf(const char* input, size_t* out_len);

/**
 * Convert TBF bytes to JSON string.
 * Returns NULL on error.
 * Caller must free result with tauq_free_string.
 */
char* tauq_tbf_to_json(const unsigned char* data, size_t len);

/**
 * Convert TBF bytes to Tauq string.
 * Returns NULL on error.
 * Caller must free result with tauq_free_string.
 */
char* tauq_tbf_to_tauq(const unsigned char* data, size_t len);

/**
 * Free buffer returned by tauq_to_tbf.
 * Requires the length that was returned.
 */
void tauq_free_buffer(unsigned char* ptr, size_t len);

#ifdef __cplusplus
}
#endif

#endif
