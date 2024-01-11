#ifndef _RANDOM_GLSL_
#define _RANDOM_GLSL_

#include "common.glsl"
#include "push_constants.glsl"

uint seed;
uint PCGHash() {
  seed = seed * 747796405u + 2891336453u;
  uint state = seed;
  uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
  return (word >> 22u) ^ word;
}

float rnd1() { return PCGHash() / float(0xFFFFFFFFU); }

vec2 rnd2() { return vec2(rnd1(), rnd1()); }

void init_random() {
  seed =
      pushConstants.seed +
      (gl_LaunchIDEXT.x + gl_LaunchSizeEXT.x * gl_LaunchIDEXT.y) * 0x12345678u;
}

#endif
