#ifndef _PAYLOAD_GLSL_
#define _PAYLOAD_GLSL_

#include "common.glsl"

// hit情報を詰め込むPayload
struct Prd {
  Material material;
  uint miss;
  vec3 hitPosition;
  vec3 hitGeometryNormal;
  vec3 hitShadingNormal;
  vec3 hitTangent;
  vec2 hitTexCoord;
  uint depth;
};

// shadow rayの結果を詰め込むPayload
struct ShadowPrd {
  vec3 transparent;
  uint shadow;
};

#endif
