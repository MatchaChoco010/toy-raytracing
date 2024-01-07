#version 460
#extension GL_EXT_ray_tracing : enable

struct Material {
  vec3 color;
  uint ty;
};

struct Prd {
  Material material;
  uint miss;
  vec3 hitPosition;
  vec3 hitGeometryNormal;
  vec3 hitShadingNormal;
};

layout(location = 0) rayPayloadInEXT Prd prd;

void main() { prd.miss = 1; }
