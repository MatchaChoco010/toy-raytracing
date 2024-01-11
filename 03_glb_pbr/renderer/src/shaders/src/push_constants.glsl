#ifndef _PUSH_CONSTANTS_GLSL_
#define _PUSH_CONSTANTS_GLSL_

#include "common.glsl"

layout(push_constant) uniform PushConstants {
  mat4 cameraRotate;
  vec3 cameraTranslate;
  uint seed;
  uint maxRecursionDepth;
  float lWhite;
  uint storageImageIndex;
  uint instanceParamsIndex;
  uint materialsIndex;
  uint padding;
  uint64_t padding2;
}
pushConstants;

#endif
