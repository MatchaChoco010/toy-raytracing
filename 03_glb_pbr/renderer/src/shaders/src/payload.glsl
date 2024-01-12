#ifndef _PAYLOAD_GLSL_
#define _PAYLOAD_GLSL_

#include "common.glsl"

struct Prd {
  Material material;
  uint miss;
  vec3 hitPosition;
  vec3 hitGeometryNormal;
  vec3 hitShadingNormal;
  vec3 hitTangent;
  vec2 hitTexCoord;
};

#endif
