#version 460
#extension GL_GOOGLE_include_directive : enable

#include "../../modules/payload.glsl"

layout(location = 1) rayPayloadInEXT ShadowPrd shadowPrd;

// 不透明物体にhitしたらshadowフラグを立てる。
void main() { shadowPrd.shadow = 1; }
