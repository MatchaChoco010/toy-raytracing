#ifndef _PUSH_CONSTANTS_GLSL_
#define _PUSH_CONSTANTS_GLSL_

// レンダラーからpush constants経由で渡されるパラメータ。
layout(push_constant) uniform PushConstants {
  uint accumulateImageIndex;
  uint baseColorImageIndex;
  uint normalImageIndex;
  uint sampleIndex;
  mat4 cameraRotate;
  vec3 cameraTranslate;
  float cameraFov;
  uint maxRecursionDepth;
  uint instanceParamsIndex;
  uint materialsIndex;
  uint[1] padding0;
  vec3 sunColor;
  float sunStrength;
  vec2 sunDirection;
  float sunAngle;
  uint sunEnabled;
  uint skyWidth;
  uint skyHeight;
  float skyRotation;
  float skyStrength;
  uint skyEnabled;
  uint[3] padding1;
  uint64_t skyBufferAddress;
  uint64_t skyCdfRowBufferAddress;
  uint64_t skyPdfRowBufferAddress;
  uint64_t skyCdfColumnBufferAddress;
  uint64_t skyPdfColumnBufferAddress;
  uint[2] padding2;
}
pushConstants;

#endif
