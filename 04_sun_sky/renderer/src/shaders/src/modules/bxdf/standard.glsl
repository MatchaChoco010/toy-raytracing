#ifndef _BXDF_STANDARD_GLSL_
#define _BXDF_STANDARD_GLSL_
#extension GL_EXT_debug_printf : enable

#include "../common.glsl"
#include "../distribute_1d.glsl"
#include "ggx.glsl"
#include "lambert.glsl"
#include "transparent.glsl"

// viewDirectionとoutDirectionを与えたときのBSDFの減衰と発光を計算する
void evalStandardBsdf(Prd prd, Material material, vec3 viewDirection,
                      vec3 outDirection, out vec3 bsdfWeight,
                      out vec3 emissive) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);

  emissive = materialData.emissive;

  vec3 L = normalize(inverse(brdfData.tbn) * outDirection);

  vec3 H = normalize(L + brdfData.V);
  float HoL = clamp(dot(H, L), 0.0, 1.0);
  float kD = 1.0 - luminance(Fresnel(brdfData.specularF0, HoL));
  kD *= 1.0 - materialData.metallic;
  kD = clamp(kD, 0.0, 1.0);

  if (dot(-viewDirection, outDirection) > 0.9999) {
    // 透過の場合
    vec3 transparentBtdf = evalTransparentBtdf(brdfData, materialData, L);
    bsdfWeight = (1.0 - materialData.alpha) * transparentBtdf;
  } else if (dot(outDirection, materialData.shadingNormal) > 0.0) {
    // 反射の場合
    bsdfWeight = vec3(0.0);

    // diffuse
    vec3 diffuseBrdf = evalLambertBrdf(brdfData, materialData, L);
    bsdfWeight += kD * diffuseBrdf;

    // specular
    vec3 specularBrdf = evalGGXBrdf(brdfData, materialData, L);
    bsdfWeight += 1.0 * specularBrdf;
  } else {
    bsdfWeight = vec3(0.0);
  }
}

// 透過したときのときのBSDFの減衰を計算する。
vec3 evalStandardBsdfTransparent(Prd prd, Material material,
                                 vec3 viewDirection) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);
  vec3 L = normalize(inverse(brdfData.tbn) * -viewDirection);

  vec3 transparentBtdf = evalTransparentBtdf(brdfData, materialData, L);
  return (1.0 - materialData.alpha) * transparentBtdf;
}

// viewDirectionとoutDirectionを与えたときのBSDFのpdfを計算する。
// One Sample ModelのMIS
// weightとpdfを組み合わせて新しいpdfを計算している形となっている。
float evalStandardPdf(Prd prd, Material material, vec3 viewDirection,
                      vec3 outDirection) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);

  vec3 L = normalize(inverse(brdfData.tbn) * outDirection);

  float weightSpecular = 1.0;
  float NoV = clamp(brdfData.V.z, 0.0, 1.0);
  float weightDiffuse = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  weightDiffuse *= 1.0 - materialData.metallic;
  weightDiffuse = clamp(weightDiffuse, 0.0, 1.0);
  float weightTransparent = 1.0 - materialData.alpha;
  float[3] func = float[3](weightSpecular, weightDiffuse, weightTransparent);

  float specularPdf = 0.0;
  float diffusePdf = 0.0;
  float transparentPdf = 0.0;
  if (dot(outDirection, prd.hitGeometryNormal) > 0.0) {
    // 反射の場合
    specularPdf = evalGGXPdf(brdfData, materialData, L);
    diffusePdf = evalLambertPdf(brdfData, materialData, L);
  } else if (dot(-viewDirection, outDirection) > 0.9999) {
    // 透過の場合
    transparentPdf = evalTransparentPdf(brdfData, materialData, L);
  }

  return getPdfDistribute1D(func, 0) * specularPdf +
         getPdfDistribute1D(func, 1) * diffusePdf +
         getPdfDistribute1D(func, 2) * transparentPdf;
}

// viewDirectionを与えたときにoutDirectionをサンプリングして
// BsDFの減衰と発光を計算し、またそのサンプリングのpdfを計算する。
// 返り値のboolは次をサンプリングするかどうかを表す。
// 法線とサンプリング方向が逆の場合や、BSDFの重みが0 or NaNの場合はfalseを返す。
bool sampleStandardBsdf(float[3] u, Prd prd, Material material,
                        vec3 viewDirection, out vec3 outDirection,
                        out float cosTheta, out vec3 bsdf, out float pdf,
                        out vec3 emissive) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);

  emissive = materialData.emissive;

  float weightSpecular = 1.0;

  float NoV = clamp(brdfData.V.z, 0.0, 1.0);
  if (NoV == 0.0) {
    return false;
  }

  float weightDiffuse = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  weightDiffuse *= 1.0 - materialData.metallic;
  weightDiffuse = clamp(weightDiffuse, 0.0, 1.0);

  float weightTransparent = 1.0 - materialData.alpha;

  float[3] func = float[3](weightSpecular, weightDiffuse, weightTransparent);

  uint bsdfType;
  float pdfBsdfSelect = samplePdfDistribute1D(u[0], func, bsdfType);

  switch (bsdfType) {
  case 0: {
    // specular
    vec2 uu = vec2(u[1], u[2]);
    vec3 L = sampleGGXDirection(uu, brdfData);
    pdf = evalGGXPdf(brdfData, materialData, L);
    pdf *= pdfBsdfSelect;

    bsdf = weightSpecular * evalGGXBrdf(brdfData, materialData, L);

    outDirection = normalize(brdfData.tbn * L);

    cosTheta = max(dot(outDirection, materialData.shadingNormal), 0.0);

    if (dot(outDirection, materialData.geometryNormal) < 0.0) {
      return false;
    }
  } break;
  case 1: {
    // diffuse
    vec2 uu = vec2(u[1], u[2]);
    vec3 L = sampleLambertDirection(uu, brdfData);
    pdf = evalLambertPdf(brdfData, materialData, L);
    pdf *= pdfBsdfSelect;

    bsdf = weightDiffuse * evalLambertBrdf(brdfData, materialData, L);

    outDirection = normalize(brdfData.tbn * L);

    // cosTheta = abs(dot(outDirection, materialData.shadingNormal));
    cosTheta = max(dot(outDirection, materialData.shadingNormal), 0.0);

    if (dot(outDirection, materialData.geometryNormal) < 0.0) {
      return false;
    }
  } break;
  case 2:
    // transparent
    vec2 uu = vec2(u[1], u[2]);
    vec3 L = sampleTransparentDirection(uu, brdfData);
    pdf = evalTransparentPdf(brdfData, materialData, L);
    pdf *= pdfBsdfSelect;

    bsdf = weightTransparent * evalTransparentBtdf(brdfData, materialData, L);

    outDirection = normalize(brdfData.tbn * L);

    cosTheta = 1.0;
    break;
  }

  if (luminance(bsdf) == 0.0 || isnan(luminance(bsdf))) {
    return false;
  }

  return true;
}

#endif
