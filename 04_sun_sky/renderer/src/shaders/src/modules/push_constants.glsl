#ifndef _PUSH_CONSTANTS_GLSL_
#define _PUSH_CONSTANTS_GLSL_

// レンダラーからpush constants経由で渡されるパラメータ。
layout(push_constant) uniform PushConstants {
  mat4 cameraRotate;
  vec3 cameraTranslate;
  uint sampleIndex;
  uint maxRecursionDepth;
  uint storageImageIndex;
  uint instanceParamsIndex;
  uint materialsIndex;
  vec2 sunDirection;
  float sunAngle;
  float sunStrength;
  vec3 sunColor;
  uint sunEnabled;
}
pushConstants;

#endif
