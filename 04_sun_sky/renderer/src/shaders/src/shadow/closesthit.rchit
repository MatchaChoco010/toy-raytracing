#version 460
#extension GL_GOOGLE_include_directive : enable

#include "../payload.glsl"

layout(location = 1) rayPayloadInEXT ShadowPrd shadowPrd;

void main() { shadowPrd.shadow = 1; }
