#version 460
#extension GL_GOOGLE_include_directive : enable

#include "common.glsl"
#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Prd prd;

void main() { prd.miss = 1; }
