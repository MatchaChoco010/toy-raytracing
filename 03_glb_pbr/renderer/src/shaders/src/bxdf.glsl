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
  float alpha;
  // local to world for shading normal
  mat3 tbn;
};

MaterialData getMaterialData(Prd prd, Material material, vec3 viewDirection) {
  vec3 baseColor;
  float alpha;
  if (material.baseColorTextureIndex == -1) {
    baseColor = material.baseColorFactor.rgb;
    alpha = material.baseColorFactor.a;
  } else {
    vec4 pixel =
        texture(images[material.baseColorTextureIndex], prd.hitTexCoord);
    baseColor = material.baseColorFactor.rgb * pixel.rgb;
    alpha = material.baseColorFactor.a * pixel.a;
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
    if (dot(geometryNormal, viewDirection) < 0.0) {
      geometryNormal = -geometryNormal;
    }
    if (dot(shadingNormal, geometryNormal) < 0.0) {
      shadingNormal = -shadingNormal;
    }
  } else {
    geometryNormal = normalize(prd.hitGeometryNormal);
    shadingNormal = normalize(prd.hitShadingNormal);
    vec3 tangent = normalize(prd.hitTangent);
    if (dot(geometryNormal, viewDirection) < 0.0) {
      geometryNormal = -geometryNormal;
    }
    if (dot(shadingNormal, geometryNormal) < 0.0) {
      shadingNormal = -shadingNormal;
      tangent = -tangent;
    }
    vec3 bitangent = cross(shadingNormal, tangent);
    tangent = cross(bitangent, shadingNormal);
    mat3 tbn = mat3(tangent, bitangent, shadingNormal);

    vec3 normalFromTexture =
        texture(images[material.normalTextureIndex], prd.hitTexCoord).rgb;
    normalFromTexture = normalize(normalFromTexture * 2.0 - 1.0);
    normalFromTexture = normalize(tbn * normalFromTexture);

    shadingNormal =
        normalize(mix(shadingNormal, normalFromTexture, material.normalFactor));
  }

  MaterialData data;
  data.baseColor = baseColor;
  data.alpha = alpha;
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
  mat3 tbn = mat3(tangent, bitangent, material.shadingNormal);

  BrdfData data;
  data.specularF0 =
      baseColorToSpecularF0(material.baseColor, material.metallic);
  data.diffuseReflectance = material.baseColor;
  // baseColorToDiffuseReflectance(material.baseColor, material.metallic);
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

// piecewise functionの分布に従ってindexをサンプリングする
float samplePdfDistribute1D(float[3] func, out uint index) {
  int n = func.length();
  float[4] cdf;
  cdf[0] = 0.0;
  for (int i = 0; i < n; i++) {
    cdf[i + 1] = cdf[i] + func[i];
  }
  float funcSum = cdf[n];
  for (int i = 0; i < n + 1; i++) {
    cdf[i] /= funcSum;
  }

  float u = rnd1();
  int first = 0;
  int len = cdf.length();
  while (len > 0) {
    int h = len >> 1;
    int middle = first + h;
    if (cdf[middle] <= u) {
      first = middle + 1;
      len = len - h - 1;
    } else {
      len = h;
    }
  }
  index = clamp(first - 1, 0, n - 1);

  float pdf = func[index] / funcSum;
  return pdf;
}

float getPdfDistribute1D(float[3] func, uint index) {
  float funcSum = 0.0;
  for (int i = 0; i < func.length(); i++) {
    funcSum += func[i];
  }
  float pdf = func[index] / funcSum;
  return pdf;
}

// viewDirectionとoutDirectionを与えたときのBSDFの重みを計算する
void evaluateBsdfWeight(Prd prd, Material material, vec3 viewDirection,
                        vec3 outDirection, out vec3 bsdfWeight,
                        out vec3 emissive) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(prd, materialData, viewDirection);

  emissive = materialData.emissive;

  float NoV = clamp(dot(brdfData.N, brdfData.V), 0.00001, 1.0);
  float kD = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  kD *= 1.0 - materialData.metallic;
  kD = clamp(kD, 0.0, 1.0);

  vec3 L = normalize(inverse(brdfData.tbn) * outDirection);

  if (dot(viewDirection, outDirection) > 0.9999) {
    // 透過の場合
    vec3 transparentBrdf = vec3(1.0);
    bsdfWeight = (1.0 - materialData.alpha) * transparentBrdf;
  } else if (dot(outDirection, prd.hitGeometryNormal) > 0.0) {
    // 反射の場合
    bsdfWeight = vec3(0.0);

    // diffuse
    vec3 diffuseBrdf = getDiffuseBrdf(brdfData, materialData);
    bsdfWeight += (kD / (1.0 + kD)) * materialData.alpha * diffuseBrdf;

    // specular
    float specularPdf = getPdfGGX(brdfData, materialData, L);
    vec3 specularWeight = sampleGGXVNDF(brdfData, L);
    vec3 specularBrdf = specularWeight * specularPdf;
    bsdfWeight += (1.0 / (1.0 + kD)) * materialData.alpha * specularBrdf;
  } else {
    bsdfWeight = vec3(0.0);
  }
}

// viewDirectionとoutDirectionを与えたときのBSDFのpdfを計算する
float evaluateBsdfPdf(Prd prd, Material material, vec3 viewDirection,
                      vec3 outDirection) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(prd, materialData, viewDirection);

  float NoV = clamp(dot(brdfData.N, brdfData.V), 0.00001, 1.0);
  float kD = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  kD *= 1.0 - materialData.metallic;
  kD = clamp(kD, 0.0, 1.0);

  vec3 L = normalize(inverse(brdfData.tbn) * outDirection);

  float weightSpecular = 1.0 / (1.0 + kD) * materialData.alpha;
  float weightDiffuse = kD / (1.0 + kD) * materialData.alpha;
  float weightTransparent = 1.0 - materialData.alpha;
  float[3] func = float[3](weightSpecular, weightDiffuse, weightTransparent);

  float specularPdf = 0.0;
  float diffusePdf = 0.0;
  float transparentPdf = 0.0;
  if (dot(viewDirection, outDirection) > 0.9999) {
    // 透過の場合
    transparentPdf = 1.0;
  } else if (dot(outDirection, prd.hitGeometryNormal) > 0.0) {
    // 反射の場合
    diffusePdf = getDiffusePdf(brdfData, materialData, L);
    specularPdf = getPdfGGX(brdfData, materialData, L);
  }

  return getPdfDistribute1D(func, 0) * specularPdf +
         getPdfDistribute1D(func, 1) * diffusePdf +
         getPdfDistribute1D(func, 2) * transparentPdf;
}

// viewDirectionを与えたときにoutDirectionをサンプリングしてBxDFの重みを計算する
bool sampleStandardBrdf(Prd prd, Material material, vec3 viewDirection,
                        out vec3 outDirection, out vec3 bsdfWeight,
                        out vec3 emissive) {
  MaterialData materialData = getMaterialData(prd, material, viewDirection);
  BrdfData brdfData = getBrdfData(prd, materialData, viewDirection);

  emissive = materialData.emissive;

  float NoV = clamp(dot(brdfData.N, brdfData.V), 0.00001, 1.0);
  float kD = 1.0 - luminance(Fresnel(brdfData.specularF0, NoV));
  kD *= 1.0 - materialData.metallic;
  kD = clamp(kD, 0.0, 1.0);

  // transmissionColorはユーザーが与えるべき値だけど、
  // 今回はbaseColorとalphaから適当に決める。
  // 厚さ1mでbaseColorだけ吸収する材質をalpha(m)の厚さだけ通り抜けたときに吸収される値を
  // 適当に透過色として決めた。
  vec3 transmissionColor =
      exp(log(clamp(materialData.baseColor, 0.0001, 1.0)) * materialData.alpha);

  float weightSpecular = 1.0 / (1.0 + kD);
  float weightDiffuse = kD / (1.0 + kD);
  float weightTransparent =
      (1.0 - materialData.alpha) * luminance(transmissionColor);
  float[3] func = float[3](weightSpecular, weightDiffuse, weightTransparent);

  uint bsdfType;
  float pdfBsdfSelect = samplePdfDistribute1D(func, bsdfType);

  switch (bsdfType) {
  case 0: {
    // specular
    vec3 L = sampleDirectionGGX(brdfData);
    float pdf = getPdfGGX(brdfData, materialData, L);
    pdf *= pdfBsdfSelect;

    // specularWeight は specularBRDF / specularPdf
    vec3 specularWeight = sampleGGXVNDF(brdfData, L);
    vec3 specularBrdf = specularWeight * getPdfGGX(brdfData, materialData, L);
    bsdfWeight = 1.0 / (1.0 + kD) * specularBrdf / pdf;

    outDirection = normalize(brdfData.tbn * L);
    if (dot(outDirection, materialData.geometryNormal) <= 0.0) {
      return false;
    }
  } break;
  case 1: {
    // diffuse
    vec3 L = cosineWeightedDirection(brdfData);
    float pdf = getDiffusePdf(brdfData, materialData, L);
    pdf *= pdfBsdfSelect;

    vec3 diffuseBrdf = getDiffuseBrdf(brdfData, materialData);
    bsdfWeight = kD / (1.0 + kD) * diffuseBrdf / pdf;

    outDirection = normalize(brdfData.tbn * L);
    if (dot(outDirection, materialData.geometryNormal) <= 0.0) {
      return false;
    }
  } break;
  case 2:
    // transparent
    vec3 L = -brdfData.V;
    float pdf = 1.0;
    pdf *= pdfBsdfSelect;

    // Absorption coefficient from Disney BSDF:
    // http://blog.selfshadow.com/publications/s2015-shading-course/burley/s2015_pbs_disney_bsdf_notes.pdf
    // // 5mmの厚さとする
    // float thinDepth = 5.0 / 100.0;
    // vec3 absorption = -log(transmissionColor) / max(thinDepth, 0.0001);
    // vec3 transparentBtdf = exp(-absorption * thinDepth);
    vec3 transparentBtdf = transmissionColor;
    bsdfWeight = (1.0 - materialData.alpha) * transparentBtdf / pdf;

    outDirection = normalize(brdfData.tbn * L);
    if (dot(outDirection, materialData.geometryNormal) > 0.0) {
      return false;
    }
    break;
  }

  if (luminance(bsdfWeight) == 0.0 || isnan(luminance(bsdfWeight))) {
    return false;
  }

  return true;
}

#endif
