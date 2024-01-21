#ifndef _BXDF_STANDARD_GLSL_
#define _BXDF_STANDARD_GLSL_

#include "../common.glsl"
#include "../distribute_1d.glsl"
#include "ggx.glsl"
#include "lambert.glsl"
#include "transparent.glsl"

// NEEで利用するためにviewDirectionとoutDirectionを与えたときのBSDFの減衰を計算する
vec3 evalStandardBsdfNEE(Prd prd, Material material, vec3 viewDirection,
                         vec3 outDirection) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);

  vec3 L = normalize(inverse(brdfData.tbn) * outDirection);

  float weightSpecular = 1.0;
  weightSpecular *= materialData.alpha;
  float NoV = clamp(brdfData.V.z, 0.0, 1.0);
  float weightDiffuse = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  weightDiffuse *= 1.0 - materialData.metallic;
  weightDiffuse = clamp(weightDiffuse, 0.0, 1.0);
  weightDiffuse *= materialData.alpha;
  float weightTransparent = 1.0 - materialData.alpha;

  // NEEではperfect
  // specular面はNEEでサンプリングしないので透過と完全鏡面は無視する
  if (dot(outDirection, materialData.shadingNormal) > 0.0) {
    // 反射の場合
    vec3 bsdf = vec3(0.0);

    // diffuse
    vec3 diffuseBrdf = evalLambertBrdf(brdfData, materialData, L);
    bsdf += weightDiffuse * diffuseBrdf;

    // specular
    if (materialData.roughness != 0.0) {
      // GGX反射
      vec3 specularBrdf = evalGGXBrdf(brdfData, materialData, L);
      bsdf += weightSpecular * specularBrdf;
    }

    return bsdf;
  } else {
    return vec3(0.0);
  }
}

// AnyHit
// shaderでの透過量を決めるために透過したときのときのBSDFの減衰を計算する。
vec3 evalStandardBsdfTransparentAnyHit(Prd prd, Material material,
                                       vec3 viewDirection) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);
  vec3 L = normalize(inverse(brdfData.tbn) * -viewDirection);

  vec3 transparentBtdf = evalTransparentBtdf(brdfData, materialData, L);
  return (1.0 - materialData.alpha) * transparentBtdf;
}

// viewDirectionとoutDirectionを与えたときのBSDFのpdfを計算する。
// One Sample ModelにMISによるサンプリングとしている。
float evalStandardPdf(Prd prd, Material material, vec3 viewDirection,
                      vec3 outDirection) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);

  vec3 L = normalize(inverse(brdfData.tbn) * outDirection);
  vec3 N = vec3(0.0, 0.0, 1.0);

  float weightSpecular = 1.0;
  weightSpecular *= materialData.alpha;
  float NoV = clamp(dot(N, brdfData.V), 0.0, 1.0);
  float weightDiffuse = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  weightDiffuse *= 1.0 - materialData.metallic;
  weightDiffuse = clamp(weightDiffuse, 0.0, 1.0);
  weightDiffuse *= materialData.alpha;
  float weightTransparent = 1.0 - materialData.alpha;
  float[3] func = float[3](weightSpecular, weightDiffuse, weightTransparent);

  float specularPdf = 0.0;
  float diffusePdf = 0.0;
  float transparentPdf = 0.0;
  if (dot(outDirection, prd.hitGeometryNormal) > 0.0) {
    // 反射の場合
    if (materialData.roughness == 0.0) {
      // 完全鏡面反射
      specularPdf = 1.0;
    } else {
      // GGX反射
      specularPdf = evalGGXPdf(brdfData, materialData, L);
    }
    diffusePdf = evalLambertPdf(brdfData, materialData, L);
  } else if (dot(-viewDirection, outDirection) > 0.9999) {
    // 透過の場合
    transparentPdf = 1.0;
  }

  return getPdfDistribute1D(func, 0) * specularPdf +
         getPdfDistribute1D(func, 1) * diffusePdf +
         getPdfDistribute1D(func, 2) * transparentPdf;
}

// sampleStandardBsdfの返り値
struct SampleStandardBsdfResult {
  vec3 outDirection;
  float cosTheta;
  vec3 bsdf;
  float pdf;
  vec3 emissive;
  bool traceNext;
};

// viewDirectionを与えたときにoutDirectionをサンプリングして
// BSDFの減衰と発光を計算し、またそのサンプリングのpdfを計算する。
SampleStandardBsdfResult
sampleStandardBsdf(float[3] u, Prd prd, Material material, vec3 viewDirection) {
  SampleStandardBsdfResult result;

  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(materialData, viewDirection);

  result.emissive = materialData.emissive;

  if (brdfData.V.z == 0.0) {
    result.traceNext = false;
    return result;
  }

  float weightSpecular = 1.0;
  weightSpecular *= materialData.alpha;
  float NoV = clamp(brdfData.V.z, 0.0, 1.0);
  float weightDiffuse = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  weightDiffuse *= 1.0 - materialData.metallic;
  weightDiffuse = clamp(weightDiffuse, 0.0, 1.0);
  weightDiffuse *= materialData.alpha;
  float weightTransparent = 1.0 - materialData.alpha;
  float[3] func = float[3](weightSpecular, weightDiffuse, weightTransparent);

  uint bsdfType;
  float pdfBsdfSelect = samplePdfDistribute1D(u[0], func, bsdfType);

  switch (bsdfType) {
  case 0: {
    // specular
    vec3 L;
    if (materialData.roughness == 0.0) {
      // 完全鏡面反射
      L = inverse(brdfData.tbn) *
          reflect(-viewDirection, materialData.shadingNormal);
      result.outDirection = normalize(brdfData.tbn * L);

      result.pdf = 1.0;
      result.pdf *= pdfBsdfSelect;

      result.bsdf = weightSpecular * vec3(1.0);

      // perfect specularなのでcos項は無視できるので1.0とする
      result.cosTheta = 1.0;
    } else {
      // GGX反射
      vec2 uu = vec2(u[1], u[2]);
      L = sampleGGXDirection(uu, brdfData);

      result.outDirection = normalize(brdfData.tbn * L);

      if (dot(result.outDirection, materialData.geometryNormal) < 0.0) {
        result.traceNext = false;
        return result;
      }

      result.pdf = evalGGXPdf(brdfData, materialData, L);
      result.pdf *= pdfBsdfSelect;

      result.bsdf = weightSpecular * evalGGXBrdf(brdfData, materialData, L);

      result.cosTheta =
          max(dot(result.outDirection, materialData.shadingNormal), 0.0);
    }
  } break;
  case 1: {
    // diffuse
    vec2 uu = vec2(u[1], u[2]);
    vec3 L = sampleLambertDirection(uu, brdfData);

    result.outDirection = normalize(brdfData.tbn * L);

    if (dot(result.outDirection, materialData.geometryNormal) < 0.0) {
      result.traceNext = false;
      return result;
    }

    result.pdf = evalLambertPdf(brdfData, materialData, L);
    result.pdf *= pdfBsdfSelect;

    result.bsdf = weightDiffuse * evalLambertBrdf(brdfData, materialData, L);

    result.cosTheta =
        max(dot(result.outDirection, materialData.shadingNormal), 0.0);
  } break;
  case 2: {
    // transparent
    vec3 L = -brdfData.V;
    result.outDirection = normalize(brdfData.tbn * L);

    result.pdf = 1.0;
    result.pdf *= pdfBsdfSelect;

    result.bsdf =
        weightTransparent * evalTransparentBtdf(brdfData, materialData, L);

    // perfect specularなのでcos項は無視できるので1.0とする
    result.cosTheta = 1.0;
  } break;
  }

  if (luminance(result.bsdf) == 0.0 || isnan(luminance(result.bsdf))) {
    result.traceNext = false;
    return result;
  }

  result.traceNext = true;
  return result;
}

#endif
