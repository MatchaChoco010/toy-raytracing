#version 460
#extension GL_GOOGLE_include_directive : enable

#include "../modules/common.glsl"
#include "../modules/payload.glsl"

layout(location = 1) rayPayloadInEXT ShadowPrd prd;

void main() { prd.shadow = 0; }
