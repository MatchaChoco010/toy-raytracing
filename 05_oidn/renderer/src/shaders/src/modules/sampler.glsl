#ifndef _SAMPLER_GLSL_
#define _SAMPLER_GLSL_

#include "common.glsl"

// T: strength of OA (0 < T <=D)
// D: dimension (< S)
// S: number of levels/strata

// permute function
#define SAMPLER_GLSL_DEFINE_PERMUTE_FUNC(T, D, S)                              \
  uint permute_##T##_##D##_##S(uint i, uint l, uint p) {                       \
    if (p == 0)                                                                \
      return i;                                                                \
    uint w = l - 1;                                                            \
    w |= w >> 1;                                                               \
    w |= w >> 2;                                                               \
    w |= w >> 4;                                                               \
    w |= w >> 8;                                                               \
    w |= w >> 16;                                                              \
    do {                                                                       \
      i ^= p;                                                                  \
      i *= 0xe170893d;                                                         \
      i ^= p >> 16;                                                            \
      i ^= (i & w) >> 4;                                                       \
      i ^= p >> 8;                                                             \
      i *= 0x0929eb3f;                                                         \
      i ^= p >> 23;                                                            \
      i ^= (i & w) >> 1;                                                       \
      i *= 1 | p >> 27;                                                        \
      i *= 0x6935fa69;                                                         \
      i ^= (i & w) >> 11;                                                      \
      i *= 0x74dcb303;                                                         \
      i ^= (i & w) >> 2;                                                       \
      i *= 0x9e501cc3;                                                         \
      i ^= (i & w) >> 2;                                                       \
      i *= 0xc860a3df;                                                         \
      i &= w;                                                                  \
      i ^= i >> 5;                                                             \
    } while (i >= l);                                                          \
    return (i + p) % l;                                                        \
  }
#define PERMUTE(T, D, S) permute_##T##_##D##_##S

// Compute the digits of decimal value ‘i‘ expressed in base ‘b‘
#define SAMPLER_GLSL_DEFINE_TO_BASE_S_FUNC(T, D, S)                            \
  uint[T] toBaseS_##T##_##D##_##S(uint i, uint b) {                            \
    uint[T] digits;                                                            \
    for (uint ii = 0; ii < T; i /= b, ++ii)                                    \
      digits[ii] = i % b;                                                      \
    return digits;                                                             \
  }
#define TO_BASE_S(T, D, S) toBaseS_##T##_##D##_##S

// Evaluate polynomial with coefficients a at location arg
#define SAMPLER_GLSL_DEFINE_EVAL_POLY_FUNC(T, D, S)                            \
  uint evalPoly_##T##_##D##_##S(const uint[T] a, uint arg) {                   \
    uint ans = 0;                                                              \
    for (uint l = T; 0 != l--;)                                                \
      ans = (ans * arg) + a[l];                                                \
    return ans;                                                                \
  }
#define EVAL_POLY(T, D, S) evalPoly_##T##_##D##_##S

// Compute substrata offsets
#define SAMPLER_GLSL_DEFINE_OFFSET_FUNC(T, D, S)                               \
  uint offset_##T##_##D##_##S(uint i, uint numSS, uint p) {                    \
    return PERMUTE(T, D, S)((i / S) % numSS, numSS, p);                        \
  }
#define OFFSET(T, D, S) offset_##T##_##D##_##S

float randfloat(uint i, uint p) {
  if (p == 0)
    return 0.5f; // always 0.5 when p == 0
  i ^= p;
  i ^= i >> 17;
  i ^= i >> 10;
  i *= 0xb36534e5;
  i ^= i >> 12;
  i ^= i >> 21;
  i *= 0x93fc4795;
  i ^= 0xdf6e307f;
  i ^= i >> 17;
  i *= 1 | p >> 18;
  return i * (1.0f / 4294967808.0f);
}

// Orthogonal Array Sampling for Monte Carlo Rendering
// https://cs.dartmouth.edu/~wjarosz/publications/jarosz19orthogonal.pdf
// 層化された多次元の乱数を生成する。
// i: sample index
// p: pseudo-random permutation seed
#define SAMPLER_GLSL_DEFINE_BUSH_OA_FUNC(T, D, S)                              \
  float bushOA_##T##_##D##_##S(uint i, uint p) {                               \
    uint N = uint(pow(S, T));                                                  \
    i = PERMUTE(T, D, S)(i, N, p);                                             \
    uint[T] iDigits = TO_BASE_S(T, D, S)(i, S);                                \
    uint stm = N / S;                                                          \
    uint k = (D % 2 != 0) ? D - 1 : D + 1;                                     \
    uint phi = EVAL_POLY(T, D, S)(iDigits, D);                                 \
    uint stratum = PERMUTE(T, D, S)(phi % S, S, p * (D + 1) * 0x51633e2d);     \
    uint subStratum = OFFSET(T, D, S)(i, stm, p * (D + 1) * 0x68bc21eb);       \
    float jitter = randfloat(i, p * (D + 1) * 0x02e5be93);                     \
    return (stratum + (subStratum + jitter) / stm) / S;                        \
  }
#define BUSH_OA(T, D, S) bushOA_##T##_##D##_##S

// 多次元の層化された乱数をOAで求めて配列で返す。
#define SAMPLER_GLSL_DEFINE_SAMPLE_RANDOM_FUNC(T, D, S)                        \
  float[D] sampleRandom_##T##_##D##_##S(uint depth) {                          \
    seed = seed * 747796405u + 2891336453u;                                    \
                                                                               \
    float[D] result;                                                           \
    for (uint i = 0; i < D; ++i)                                               \
      result[i] = BUSH_OA(T, D, S)(i, seed + i * 0x03245768u);                 \
    return result;                                                             \
  }
#define SAMPLE_RANDOM(T, D, S) sampleRandom_##T##_##D##_##S

SAMPLER_GLSL_DEFINE_PERMUTE_FUNC(2, 2, 3)
SAMPLER_GLSL_DEFINE_TO_BASE_S_FUNC(2, 2, 3)
SAMPLER_GLSL_DEFINE_EVAL_POLY_FUNC(2, 2, 3)
SAMPLER_GLSL_DEFINE_OFFSET_FUNC(2, 2, 3)
SAMPLER_GLSL_DEFINE_BUSH_OA_FUNC(2, 2, 3)
SAMPLER_GLSL_DEFINE_SAMPLE_RANDOM_FUNC(2, 2, 3)

SAMPLER_GLSL_DEFINE_PERMUTE_FUNC(3, 3, 4)
SAMPLER_GLSL_DEFINE_TO_BASE_S_FUNC(3, 3, 4)
SAMPLER_GLSL_DEFINE_EVAL_POLY_FUNC(3, 3, 4)
SAMPLER_GLSL_DEFINE_OFFSET_FUNC(3, 3, 4)
SAMPLER_GLSL_DEFINE_BUSH_OA_FUNC(3, 3, 4)
SAMPLER_GLSL_DEFINE_SAMPLE_RANDOM_FUNC(3, 3, 4)

#endif
