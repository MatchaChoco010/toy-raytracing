#ifndef _BXDF_GGX_GLSL_
#define _BXDF_GGX_GLSL_

#include "bxdf_common.glsl"

// Source: "Sampling Visible GGX Normals with Spherical Caps" by Dupuy & Benyoub
vec3 sampleGGXVNDF(vec2 u, BrdfData brdf) {
  vec2 alpha2 = vec2(brdf.alpha, brdf.alpha);
  vec3 Vh = normalize(vec3(alpha2.x * brdf.V.x, alpha2.y * brdf.V.y, brdf.V.z));
  float phi = 2.0 * PI * u.x;
  float z = ((1.0 - u.y) * (1.0 + Vh.z)) - Vh.z;
  float sinTheta = sqrt(clamp(1.0 - z * z, 0.0, 1.0));
  float x = cos(phi) * sinTheta;
  float y = sin(phi) * sinTheta;
  vec3 Nh = vec3(x, y, z) + Vh;
  return normalize(vec3(alpha2.x * Nh.x, alpha2.y * Nh.y, max(Nh.z, 0.0)));
}

// Smith G1 term (masking function)のGGX distribution向けoptimizedバージョン (by
// substituting G_a into G1_GGX)
float Smith_G1_GGX(float alpha, float NoS) {
  float a2 = alpha * alpha;
  float NoS2 = NoS * NoS;
  return 2.0f / (sqrt(((a2 * (1.0f - NoS2)) + NoS2) / NoS2) + 1.0f);
}

// G2/G1のheight correlatedはG1項だけで書ける
// Source: "Implementing a Simple Anisotropic Rough Diffuse Material with
// Stochastic Evaluation", Appendix A by Heitz & Dupuy
float Smith_G2_Over_G1_Height_Correlated(float alpha, float NoV, float NoL) {
  float G1V = Smith_G1_GGX(alpha, NoV);
  float G1L = Smith_G1_GGX(alpha, NoL);
  return G1L / (G1V + G1L - G1V * G1L);
}

// GGXの法線分布関数を返す。
float GGX_D(float alpha, float NoH) {
  float b = ((alpha * alpha - 1.0f) * NoH * NoH + 1.0f);
  return alpha * alpha / (PI * b * b);
}

// (brdf * VNDFサンプリングのpdf)の値を返す。
// VNDFのpdfとGGXのBRDFは打ち消し合って最終的にはF * (G2 / G1)になる。
vec3 evalWeightGGXVNDF(BrdfData brdf) {
  vec3 H = normalize(brdf.V + brdf.L);
  float HoL = clamp(dot(H, brdf.L), 0.00001, 1.0);
  float NoL = clamp(dot(brdf.N, brdf.L), 0.00001, 1.0);
  float NoV = clamp(dot(brdf.N, brdf.V), 0.00001, 1.0);
  float NoH = clamp(dot(brdf.N, H), 0.00001, 1.0);

  vec3 F = Fresnel(brdf.specularF0, HoL);

  vec3 weight = F * Smith_G2_Over_G1_Height_Correlated(brdf.alpha, NoV, NoL);

  return weight;
}

// GGX反射用の方向サンプリング方法。
// VNDFサンプリングをして、その方向をハーフベクとする方向を反射方向とする。
// roughnessが0の場合はサンプリングせずにハーフベクトルを(0, 0, 1)にしている。
vec3 sampleGGXDirection(vec2 u, BrdfData brdf) {
  vec3 H;
  if (brdf.alpha == 0.0) {
    H = vec3(0.0, 0.0, 1.0);
  } else {
    H = sampleGGXVNDF(u, brdf);
  }

  vec3 L = normalize(reflect(-brdf.V, H));

  return L;
}

// GGX反射用方向サンプリングに対応したpdfの値を計算する。
float evalGGXPdf(BrdfData brdf, MaterialData material) {
  float alpha = material.roughness * material.roughness;

  vec3 H = normalize(brdf.V + brdf.L);
  float NoH = clamp(dot(brdf.N, H), 0.00001, 1.0);
  float HoL = clamp(dot(H, brdf.L), 0.00001, 1.0);

  float D = GGX_D(alpha, NoH);
  return D * NoH / (4.0 * HoL);
}

// GGXのBRDFの値を計算する。
// weightの計算とpdfの計算からbrdfの値を求めている。
vec3 evalGGXBrdf(BrdfData brdf, MaterialData material) {
  // weight = brdf / pdf
  vec3 weight = evalWeightGGXVNDF(brdf);
  float pdf = evalGGXPdf(brdf, material);
  return weight * pdf;
}

#endif
