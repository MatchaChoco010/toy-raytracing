#ifndef _BXDF_GGX_GLSL_
#define _BXDF_GGX_GLSL_
#extension GL_EXT_debug_printf : enable

#include "bxdf_common.glsl"

float Smith_G1_std(vec3 N, vec3 S) {
  float NoS = clamp(dot(N, S), 0.00001, 1.0);
  float sigma_std = (1 + S.z) / 2;
  return NoS * S.z / sigma_std;
}

// GGXのMasking-shadowing関数
// Source: Sampling Visible GGX Normals with Spherical Caps
float Smith_G1_GGX(float alpha, vec3 S) {
  vec3 M = vec3(1.0 / alpha, 1.0 / alpha, 1.0);
  vec3 Mi = vec3(alpha, alpha, 1.0);
  vec3 N = vec3(0.0, 0.0, 1.0);
  return Smith_G1_std(M * N / sqrt(dot(M * N, M * N)),
                      Mi * S / sqrt(dot(Mi * S, Mi * S)));
}

float D_std(vec3 H) {
  if (H.z <= 0.0) {
    return 0.0;
  }
  return 1.0 / PI;
}

// GGXの法線分布関数
// Source: Sampling Visible GGX Normals with Spherical Caps
float D_GGX(float alpha, vec3 H) {
  vec3 M = vec3(1.0 / alpha, 1.0 / alpha, 1.0);

  float detMt = abs(M.x * M.z);
  vec3 MtH = M * H;
  float MtH2 = dot(MtH, MtH);
  float MtH4 = MtH2 * MtH2;
  float J = detMt / MtH4;

  vec3 v = M * H / sqrt(dot(M * H, M * H));

  return D_std(v) * J;
}

// Sampling the visible hemisphere as half vectors
// Source: Sampling Visible GGX Normals with Spherical Caps
vec3 SampleVndf_Hemisphere(vec2 u, vec3 wi) {
  // sample a spherical cap in (-wi.z, 1]
  float phi = 2.0f * PI * u.x;
  float z = fma((1.0f - u.y), -wi.z, (1.0f + wi.z));
  float sinTheta = sqrt(clamp(1.0f - z * z, 0.0f, 1.0f));
  float x = sinTheta * cos(phi);
  float y = sinTheta * sin(phi);
  vec3 c = vec3(x, y, z);
  // compute halfway direction;
  vec3 h = c + wi;
  // return without normalization (as this is done later)
  return h;
}

// Source: Sampling Visible GGX Normals with Spherical Caps
vec3 SampleVndf_GGX(vec2 u, vec3 wi, float alpha) {
  // warp to the hemisphere configuration
  vec3 wiStd = normalize(vec3(wi.x * alpha, wi.y * alpha, wi.z));
  // sample the hemisphere (see implementation 2 or 3)
  vec3 wmStd = SampleVndf_Hemisphere(u, wiStd);
  // warp back to the ellipsoid configuration
  vec3 wm = normalize(vec3(wmStd.x * alpha, wmStd.y * alpha, wmStd.z));
  // return final normal
  return wm;
}

// GGX反射用の方向サンプリング方法。
// VNDFサンプリングをして、その方向をハーフベクとする方向を反射方向とする。
// roughnessが0の場合は完全鏡面として扱い、
// サンプリングせずにハーフベクトルを(0, 0, 1)にしている。
vec3 sampleGGXDirection(vec2 u, BrdfData brdf) {
  if (brdf.alpha == 0.0) {
    vec3 H = vec3(0.0, 0.0, 1.0);
    return normalize(reflect(-brdf.V, H));
  } else {
    vec3 H = SampleVndf_GGX(u, brdf.V, brdf.alpha);
    return normalize(reflect(-brdf.V, H));
  }
}

// GGX反射用方向サンプリングに対応したpdfの値を計算する。
float evalGGXPdf(BrdfData brdf, MaterialData material, vec3 L) {
  vec3 H = normalize(brdf.V + L);
  vec3 N = vec3(0.0, 0.0, 1.0);
  float NoH = dot(N, H);
  float HoL = dot(H, L);
  float NoL = max(dot(N, L), 0.00001);

  float G1l = Smith_G1_GGX(brdf.alpha, L);
  float D = D_GGX(brdf.alpha, H);

  // 完全鏡面の場合は幾何減衰を1、Dを1とする。
  if (brdf.alpha <= 0.00001 && NoH > 0.9999) {
    G1l = 1.0;
    D = 1.0;
  }

  return G1l * D / (4.0 * NoL);
}

// GGXのBRDFの値を計算する。
// weightの計算とpdfの計算からbrdfの値を求めている。
vec3 evalGGXBrdf(BrdfData brdf, MaterialData material, vec3 L) {
  vec3 H = normalize(brdf.V + L);
  vec3 N = vec3(0.0, 0.0, 1.0);

  float NoH = dot(N, H);
  float HoV = dot(H, brdf.V);
  float NoV = max(dot(N, brdf.V), 0.00001);
  float NoL = max(dot(N, L), 0.00001);

  vec3 F = Fresnel(brdf.specularF0, HoV);
  float G1v = Smith_G1_GGX(brdf.alpha, brdf.V);
  float G1l = Smith_G1_GGX(brdf.alpha, L);
  float G2 = G1l * G1v / (G1v + G1l - G1v * G1l);
  float D = D_GGX(brdf.alpha, H);

  // 完全鏡面の場合は幾何減衰を1、Dを1とする
  if (brdf.alpha <= 0.00001 && NoH > 0.9999) {
    G2 = 1.0;
    D = 1.0;
  }

  vec3 f = F * G2 * D / (4.0 * NoV * NoL);

  if (isnan(f.x) || isnan(f.y) || isnan(f.z)) {
    return vec3(0.0);
  }

  return f;
}

#endif
