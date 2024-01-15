#version 460
#extension GL_GOOGLE_include_directive : enable

#include "../common.glsl"
#include "../payload.glsl"

layout(location = 1) rayPayloadInEXT ShadowPrd prd;

void main() { prd.shadow = 0; }
