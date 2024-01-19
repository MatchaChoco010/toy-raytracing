#ifndef _BXDF_LAMBERT_GLSL_
#define _BXDF_LAMBERT_GLSL_

#include "bxdf_common.glsl"

// Lambert反射用の方向サンプリング方法。
// cosine weighted hemisphere sampling
vec3 sampleLambertDirection(vec2 uu, BrdfData brdf) {
  float cosTheta = sqrt(uu.x);
  float sinTheta = sqrt(1.0 - uu.x);
  float phi = uu.x * 2 * PI;
  vec3 N = vec3(sinTheta * cos(phi), sinTheta * sin(phi), cosTheta);
  return normalize(N);
}

// Lambert反射の方向サンプリングに対応したpdfの値を計算する。
float evalLambertPdf(BrdfData brdf, MaterialData material, vec3 L) {
  vec3 N = vec3(0.0, 0.0, 1.0);
  return max(dot(N, L), 0.0) / PI;
}

// Lambert反射のBRDFの値を計算する。
// Lambert反射はbrdf.VやLに依存しない。
vec3 evalLambertBrdf(BrdfData brdf, MaterialData material, vec3 L) {
  return brdf.diffuseReflectance / PI;
}

#endif
