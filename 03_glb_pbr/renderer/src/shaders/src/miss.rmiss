#version 460
#extension GL_EXT_ray_tracing : enable

struct Material {
  vec4 baseColorFactor;
  int baseColorTextureIndex;
  vec3 emissiveFactor;
  int emissiveTextureIndex;
  float metallicFactor;
  int metallicTextureIndex;
  float roughnessFactor;
  int roughnessTextureIndex;
  float normalFactor;
  int normalTextureIndex;
  uint ty;
};

struct Prd {
  Material material;
  uint miss;
  vec3 hitPosition;
  vec3 hitGeometryNormal;
  vec3 hitShadingNormal;
  vec2 hitTexCoord;
};

layout(location = 0) rayPayloadInEXT Prd prd;

void main() { prd.miss = 1; }
