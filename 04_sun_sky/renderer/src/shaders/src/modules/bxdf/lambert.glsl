#ifndef _BXDF_LAMBERT_GLSL_
#define _BXDF_LAMBERT_GLSL_

#include "bxdf_common.glsl"

// Lambert反射用の方向サンプリング方法。
// cosine weighted hemisphere sampling
vec3 sampleLambertDirection(vec2 uu, BrdfData brdf) {
  vec3 normal = brdf.N;
  float up = sqrt(uu.x);
  float over = sqrt(1.0 - up * up);
  float around = uu.y * 2 * PI;
  vec3 u = normalize(abs(normal.x) < 0.999 ? cross(normal, vec3(1, 0, 0))
                                           : cross(normal, vec3(0, 1, 0)));
  vec3 v = cross(normal, u);
  return normalize(u * cos(around) * over + v * sin(around) * over +
                   normal * up);
}

// Lambert反射の方向サンプリングに対応したpdfの値を計算する。
float evalLambertPdf(BrdfData brdf, MaterialData material) {
  return max(dot(brdf.N, brdf.L), 0.0) / PI;
}

// Lambert反射のBRDFの値を計算する。
// Lambert反射はbrdf.VやLに依存しない。
vec3 evalLambertBrdf(BrdfData brdf, MaterialData material) {
  return brdf.diffuseReflectance / PI;
}

#endif
