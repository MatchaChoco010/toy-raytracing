#version 460
#extension GL_GOOGLE_include_directive : enable

#include "../modules/common.glsl"
#include "../modules/payload.glsl"

layout(location = 0) rayPayloadInEXT Prd prd;

void main() { prd.miss = 1; }
