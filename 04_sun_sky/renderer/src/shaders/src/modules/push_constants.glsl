#ifndef _PUSH_CONSTANTS_GLSL_
#define _PUSH_CONSTANTS_GLSL_

// レンダラーからpush constants経由で渡されるパラメータ。
layout(push_constant) uniform PushConstants {
  mat4 cameraRotate;
  vec3 cameraTranslate;
  float cameraFov;
  uint sampleIndex;
  uint maxRecursionDepth;
  uint storageImageIndex;
  uint instanceParamsIndex;
  uint materialsIndex;
  float sunStrength;
  uint[2] padding0;
  vec3 sunColor;
  uint[1] padding1;
  vec2 sunDirection;
  float sunAngle;
  uint sunEnabled;
}
pushConstants;

#endif
