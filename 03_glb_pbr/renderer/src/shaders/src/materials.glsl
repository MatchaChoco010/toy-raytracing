#ifndef _MATERIALS_GLSL_
#define _MATERIALS_GLSL_

#include "common.glsl"
#include "payload.glsl"
#include "random.glsl"

const float MIN_DIELECTRICS_F0 = 0.04;

float luminance(vec3 color) {
  return 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
}

vec3 baseColorToSpecularF0(vec3 baseColor, float metallic) {
  vec3 specularF0 = mix(vec3(MIN_DIELECTRICS_F0), baseColor, metallic);
  return specularF0;
}

vec3 baseColorToDiffuseReflectance(vec3 baseColor, float metallic) {
  vec3 diffuseReflectance = baseColor * (1.0 - metallic);
  return diffuseReflectance;
}

vec3 Fresnel(vec3 F0, float LoH) {
  vec3 n = (1 + sqrt(F0)) / (1 - sqrt(F0));
  float c = LoH;
  vec3 g2 = n * n + c * c - 1;
  vec3 g = sqrt(g2);
  vec3 f = (1 * (g - c) * (g - c)) / (2 * (g + c) * (g + c)) *
           (1 + (c * (g + c) - 1) * (c * (g + c) - 1) /
                    ((c * (g - c) + 1) * (c * (g - c) + 1)));
  return f;
}

struct MaterialData {
  vec3 baseColor;
  float metallic;
  float roughness;
  vec3 emissive;
  vec3 shadingNormal;
  vec3 geometryNormal;
  // local to world for shading normal
  mat3 tbn;
};

MaterialData getMaterialData(Prd prd, Material material, vec3 viewDirection) {
  vec3 baseColor;
  if (material.baseColorTextureIndex == -1) {
    baseColor = material.baseColorFactor.rgb;
  } else {
    baseColor =
        material.baseColorFactor.rgb *
        texture(images[material.baseColorTextureIndex], prd.hitTexCoord).rgb;
  }

  vec3 emissive;
  if (material.emissiveTextureIndex == -1) {
    emissive = material.emissiveFactor;
  } else {
    emissive =
        material.emissiveFactor *
        texture(images[material.emissiveTextureIndex], prd.hitTexCoord).rgb;
  }

  float metallic;
  if (material.metallicTextureIndex == -1) {
    metallic = material.metallicFactor;
  } else {
    metallic =
        material.metallicFactor *
        texture(images[material.metallicTextureIndex], prd.hitTexCoord).r;
  }

  float roughness;
  if (material.roughnessTextureIndex == -1) {
    roughness = material.roughnessFactor;
  } else {
    roughness =
        material.roughnessFactor *
        texture(images[material.roughnessTextureIndex], prd.hitTexCoord).r;
  }

  vec3 geometryNormal;
  vec3 shadingNormal;
  if (material.normalTextureIndex == -1) {
    geometryNormal = normalize(prd.hitGeometryNormal);
    shadingNormal = normalize(prd.hitShadingNormal);
  } else {
    geometryNormal = normalize(prd.hitGeometryNormal);
    shadingNormal = normalize(prd.hitShadingNormal);
    vec3 normalFromTexture = normalize(
        texture(images[material.normalTextureIndex], prd.hitTexCoord).rgb *
            2.0 -
        1.0);
    vec3 tangent;
    if (abs(dot(shadingNormal, vec3(0.0, 0.0, 1.0))) < 0.999) {
      tangent = normalize(cross(shadingNormal, vec3(0.0, 0.0, 1.0)));
    } else {
      tangent = normalize(cross(shadingNormal, vec3(0.0, 1.0, 0.0)));
    }
    vec3 bitangent = cross(shadingNormal, tangent);
    mat3 tbn = mat3(tangent, bitangent, prd.hitShadingNormal);
    shadingNormal = normalize(tbn * normalFromTexture);
  }
  if (dot(geometryNormal, viewDirection) < 0.0) {
    geometryNormal = -geometryNormal;
  }
  if (dot(shadingNormal, geometryNormal) < 0.0) {
    shadingNormal = -shadingNormal;
  }

  MaterialData data;
  data.baseColor = baseColor;
  data.metallic = metallic;
  data.roughness = roughness;
  data.emissive = emissive;
  data.shadingNormal = shadingNormal;
  data.geometryNormal = geometryNormal;
  return data;
}

struct BrdfData {
  vec3 specularF0;
  vec3 diffuseReflectance;

  float alpha;

  vec3 V; // view direction in local space for shading normal
  vec3 N; // shading normal in local space for shading normal

  mat3 tbn;
};

BrdfData getBrdfData(Prd prd, MaterialData material, vec3 viewDirection) {
  vec3 tangent;
  if (abs(dot(material.shadingNormal, vec3(0.0, 0.0, 1.0))) < 0.999) {
    tangent = normalize(cross(material.shadingNormal, vec3(0.0, 0.0, 1.0)));
  } else {
    tangent = normalize(cross(material.shadingNormal, vec3(0.0, 1.0, 0.0)));
  }
  vec3 bitangent = cross(material.shadingNormal, tangent);
  mat3 tbn = mat3(tangent, bitangent, prd.hitShadingNormal);

  BrdfData data;
  data.specularF0 =
      baseColorToSpecularF0(material.baseColor, material.metallic);
  data.diffuseReflectance =
      baseColorToDiffuseReflectance(material.baseColor, material.metallic);
  data.alpha = material.roughness * material.roughness;
  data.V = normalize(inverse(tbn) * viewDirection);
  data.N = vec3(0.0, 0.0, 1.0);
  data.tbn = tbn;
  return data;
}

vec3 cosineWeightedDirection(BrdfData brdf) {
  vec2 rnd = rnd2();
  vec3 normal = brdf.N;
  float up = sqrt(rnd.x);
  float over = sqrt(1.0 - up * up);
  float around = rnd.y * 2 * PI;
  vec3 u = normalize(abs(normal.x) < 0.999 ? cross(normal, vec3(1, 0, 0))
                                           : cross(normal, vec3(0, 1, 0)));
  vec3 v = cross(normal, u);
  return normalize(u * cos(around) * over + v * sin(around) * over +
                   normal * up);
}

vec3 getDiffuseBrdf(BrdfData brdf, MaterialData material) {
  return brdf.diffuseReflectance / PI;
  // return vec3(1.0, 0.5, 0.3) / PI;
}

float getDiffusePdf(BrdfData brdf, MaterialData material, vec3 L) {
  return max(dot(brdf.N, L), 0.0) / PI;
}

// Source: "Sampling Visible GGX Normals with Spherical Caps" by Dupuy & Benyoub
vec3 sampleGGXVNDF(BrdfData brdf) {
  vec2 rnd = rnd2();
  vec2 alpha2 = vec2(brdf.alpha, brdf.alpha);
  vec3 Vh = normalize(vec3(alpha2.x * brdf.V.x, alpha2.y * brdf.V.y, brdf.V.z));
  float phi = 2.0 * PI * rnd.x;
  float z = ((1.0 - rnd.y) * (1.0 + Vh.z)) - Vh.z;
  float sinTheta = sqrt(clamp(1.0 - z * z, 0.0, 1.0));
  float x = cos(phi) * sinTheta;
  float y = sin(phi) * sinTheta;
  vec3 Nh = vec3(x, y, z) + Vh;
  return normalize(vec3(alpha2.x * Nh.x, alpha2.y * Nh.y, max(Nh.z, 0.0)));
}

vec3 sampleDirectionGGX(BrdfData brdf) {
  vec3 H;
  if (brdf.alpha == 0.0) {
    H = vec3(0.0, 0.0, 1.0);
  } else {
    H = sampleGGXVNDF(brdf);
  }

  vec3 L = normalize(reflect(-brdf.V, H));

  return L;
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

float GGX_D(float alpha, float NoH) {
  float b = ((alpha * alpha - 1.0f) * NoH * NoH + 1.0f);
  return alpha * alpha / (PI * b * b);
}

float getPdfGGX(BrdfData brdf, MaterialData material, vec3 L) {
  float alpha = material.roughness * material.roughness;

  vec3 H = normalize(brdf.V + L);
  float NoH = clamp(dot(brdf.N, H), 0.00001, 1.0);
  float HoL = clamp(dot(H, L), 0.00001, 1.0);

  float D = GGX_D(alpha, NoH);
  return D * NoH / (4.0 * HoL);
}

// VNDFのpdfとGGXのBRDFは打ち消し合って最終的にはF * (G2 / G1)になる
vec3 sampleGGXVNDF(BrdfData brdf, vec3 L) {
  vec3 H = normalize(brdf.V + L);
  float HoL = clamp(dot(H, L), 0.00001, 1.0);
  float NoL = clamp(dot(brdf.N, L), 0.00001, 1.0);
  float NoV = clamp(dot(brdf.N, brdf.V), 0.00001, 1.0);
  float NoH = clamp(dot(brdf.N, H), 0.00001, 1.0);

  vec3 F = Fresnel(brdf.specularF0, HoL);

  vec3 weight = F * Smith_G2_Over_G1_Height_Correlated(brdf.alpha, NoV, NoL);

  return weight;
}

bool evaluateStandardBrdf(Prd prd, Material material, vec3 viewDirection,
                          out vec3 outDirection, out vec3 brdfWeight,
                          out vec3 emissive) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);

  emissive = materialData.emissive;
  if (dot(materialData.shadingNormal, viewDirection) <= 0.0) {
    return false;
  }

  BrdfData brdfData = getBrdfData(prd, materialData, viewDirection);

  float russianRouletteProbability =
      max(max(max(brdfData.specularF0.r, brdfData.specularF0.g),
              brdfData.specularF0.b),
          max(max(brdfData.diffuseReflectance.r, brdfData.diffuseReflectance.g),
              brdfData.diffuseReflectance.b));
  if (rnd1() > russianRouletteProbability) {
    return false;
  }

  float NoV = clamp(dot(brdfData.N, brdfData.V), 0.00001, 1.0);
  float kD = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  kD *= 1.0 - materialData.metallic;
  kD = clamp(kD, 0.0, 1.0);

  vec3 L;
  float rnd = rnd1();
  if (rnd < kD) {
    L = cosineWeightedDirection(brdfData);
  } else {
    L = sampleDirectionGGX(brdfData);
  }
  outDirection = normalize(brdfData.tbn * L);

  if (dot(outDirection, prd.hitGeometryNormal) <= 0.0) {
    return false;
  }

  float aD = kD / (1.0 + kD);
  float aS = 1.0 / (1.0 + kD);

  float diffusePdf = getDiffusePdf(brdfData, materialData, L);
  float specularPdf = getPdfGGX(brdfData, materialData, L);

  if (diffusePdf == 0.0 && specularPdf == 0.0) {
    return false;
  } else if (isinf(diffusePdf) || isnan(diffusePdf) || diffusePdf == 0.0) {
    // specularWeight = specularBrdf / specularPdf
    vec3 specularWeight = sampleGGXVNDF(brdfData, L);
    brdfWeight = specularWeight;
  } else if (isinf(specularPdf) || isnan(specularPdf) || specularPdf == 0.0) {
    vec3 diffuseBrdf = getDiffuseBrdf(brdfData, materialData);
    brdfWeight = diffuseBrdf / diffusePdf;
  } else {
    vec3 diffuseBrdf = getDiffuseBrdf(brdfData, materialData);
    float diffuseMisWeight =
        aD * diffusePdf / (aD * diffusePdf + aS * specularPdf);
    brdfWeight = diffuseMisWeight * diffuseBrdf / diffusePdf;

    // specularWeight = specularBrdf / specularPdf
    vec3 specularWeight = sampleGGXVNDF(brdfData, L);
    float specularMisWeight =
        aS * specularPdf / (aS * diffusePdf + aS * specularPdf);
    brdfWeight += specularMisWeight * specularWeight;
  }

  brdfWeight /= russianRouletteProbability;

  if (luminance(brdfWeight) == 0.0 || isnan(luminance(brdfWeight))) {
    return false;
  }

  return true;
}

#endif
