#ifndef _RANDOM_GLSL_
#define _RANDOM_GLSL_

#include "common.glsl"
#include "payload.glsl"

// 乱数として使うPCGHash関数。
// https://www.reedbeta.com/blog/hash-functions-for-gpu-rendering/
uint PCGHash() {
  seed = seed * 747796405u + 2891336453u;
  uint state = seed;
  uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
  return (word >> 22u) ^ word;
}

// 0.0～1.0の範囲の乱数を返す。
float rnd() { return PCGHash() / float(0xFFFFFFFFU); }

#endif
