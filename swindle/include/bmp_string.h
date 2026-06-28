
/**
 * @file bmp_string.h
 * @brief String builder/wrapper for constructing debug and GDB response strings
 */

#pragma once
#include "esprit.h"

/**
 * @brief Simple string builder/wrapper class for constructing debug and GDB response strings.
 *
 * Maintains an internal C-string buffer with a hard limit.
 * Content can be appended as plain text, hex-formatted, or decimal numbers.
 * When the buffer limit is reached, the string is truncated and terminated.
 */
class stringWrapper
{
  public:
    /** @brief Construct an empty stringWrapper with a default buffer size. */
    stringWrapper();
    /** @brief Destroy the stringWrapper and free the internal buffer. */
    ~stringWrapper();

    /** @brief Double the internal buffer limit (up to a maximum). */
    void doubleLimit();

    /** @brief Append a plain C-string. */
    void append(const char *a);

    /** @brief Append a uint32 as 8 hex digits (no prefix). */
    void appendHex32(const uint32_t value);

    /**
     * @brief Append a C-string with each non-printable byte hex-escaped.
     * @param a Input string to hex-escape and append.
     */
    void appendHexified(const char *a);

    /** @brief Append a uint32 as decimal digits. */
    void appendU32(uint32_t val);

    /** @return Pointer to the internal C-string buffer. */
    char *string()
    {
        return _st;
    }

  protected:
    char *_st;  /**< Internal string buffer. */
    int _limit; /**< Current buffer size limit. */
};
