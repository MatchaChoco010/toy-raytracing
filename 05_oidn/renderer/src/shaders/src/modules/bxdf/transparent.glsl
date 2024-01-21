#ifndef _BXDF_TRANSPARENT_GLSL_
#define _BXDF_TRANSPARENT_GLSL_

#include "bxdf_common.glsl"

// 透過色による減衰を計算する。
// brdf.Lとbrdf.Vが完全に反対を向いていることを想定している。
vec3 evalTransparentBtdf(BrdfData brdf, MaterialData material, vec3 L) {
  // transmissionColorはユーザーが与えるべき値だけど、
  // 今回はbaseColorとalphaから適当に決める。
  // 厚さ1mでbaseColorだけ吸収する材質をalpha(m)の厚さだけ通り抜けたときに吸収される値を
  // 適当に透過色として決めた。
  vec3 transmissionColor =
      exp(log(clamp(material.baseColor, 0.00001, 1.0)) * material.alpha);

  // Absorption coefficient from Disney BSDF:
  // http://blog.selfshadow.com/publications/s2015-shading-course/burley/s2015_pbs_disney_bsdf_notes.pdf
  // をもとに次のように計算する。
  // ```
  // // 5mmの厚さとする
  // float thinDepth = 5.0 / 100.0;
  // vec3 absorption = -log(transmissionColor) / max(thinDepth, 0.0001);
  // vec3 transparentBtdf = exp(-absorption * thinDepth);
  // ```
  // これは次のように簡略化できる。
  vec3 transparentBtdf = transmissionColor;
  return transparentBtdf;
}

#endif