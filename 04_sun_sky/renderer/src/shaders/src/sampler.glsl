#ifndef _SAMPLER_GLSL_
#define _SAMPLER_GLSL_

#include "common.glsl"
#include "push_constants.glsl"

// strength of OA (0 < t <=DIMENSION)
#define T 3
// dimension (<s)
#define DIMENSION 4
// number of levels/strata
#define S 5

uint permute(uint i, uint l, uint p) {
  if (p == 0)
    return i; // identity permutation when p == 0
  uint w = l - 1;
  w |= w >> 1;
  w |= w >> 2;
  w |= w >> 4;
  w |= w >> 8;
  w |= w >> 16;
  do {
    i ^= p;
    i *= 0xe170893d;
    i ^= p >> 16;
    i ^= (i & w) >> 4;
    i ^= p >> 8;
    i *= 0x0929eb3f;
    i ^= p >> 23;
    i ^= (i & w) >> 1;
    i *= 1 | p >> 27;
    i *= 0x6935fa69;
    i ^= (i & w) >> 11;
    i *= 0x74dcb303;
    i ^= (i & w) >> 2;
    i *= 0x9e501cc3;
    i ^= (i & w) >> 2;
    i *= 0xc860a3df;
    i &= w;
    i ^= i >> 5;

  } while (i >= l);
  return (i + p) % l;
}

// Compute the digits of decimal value ‘i‘ expressed in base ‘b‘
uint[T] toBaseS(uint i, uint b) {
  uint[T] digits;
  for (uint ii = 0; ii < T; i /= b, ++ii)
    digits[ii] = i % b;
  return digits;
}

// Evaluate polynomial with coefficients a at location arg
uint evalPoly(const uint[T] a, uint arg) {
  uint ans = 0;
  for (uint l = T; 0 != l--;)
    ans = (ans * arg) + a[l]; // Horner’s rule
  return ans;
}

// Compute substrata offsets
uint offset(uint i, uint numSS, uint p) {
  return permute((i / S) % numSS, numSS, p); // MJ
}

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

// i: sample index
// p: pseudo-random permutation seed
float bushOA(uint i, uint p) {
  uint N = uint(pow(S, T));
  i = permute(i, N, p);
  uint[T] iDigits = toBaseS(i, S);
  uint stm = N / S; // pow(s, T-1)
  uint k = (DIMENSION % 2 != 0) ? DIMENSION - 1 : DIMENSION + 1;
  uint phi = evalPoly(iDigits, DIMENSION);
  uint stratum = permute(phi % S, S, p * (DIMENSION + 1) * 0x51633e2d);
  uint subStratum = offset(i, stm, p * (DIMENSION + 1) * 0x68bc21eb);
  float jitter = randfloat(i, p * (DIMENSION + 1) * 0x02e5be93);
  return (stratum + (subStratum + jitter) / stm) / S;
}

float[DIMENSION] sampleRandom(uint depth) {
  seed = seed * 747796405u + 2891336453u;

  float[DIMENSION] result;
  for (uint i = 0; i < DIMENSION; ++i)
    result[i] = bushOA(i, seed + i * 0x03245768u);
  return result;
}

#endif
