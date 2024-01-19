#ifndef _BXDF_BXDF_COMMON_GLSL_
#define _BXDF_BXDF_COMMON_GLSL_

#include "../payload.glsl"

// 非導電体のF0
const float MIN_DIELECTRICS_F0 = 0.04;

// metallicに応じてspecularのF0の値を計算する。
// metallicワークフローを参照のこと。
vec3 baseColorToSpecularF0(vec3 baseColor, float metallic) {
  vec3 specularF0 = mix(vec3(MIN_DIELECTRICS_F0), baseColor, metallic);
  return specularF0;
}

// 拡散反射の反射率を計算する。
// metallicでは拡散反射は0になる。
vec3 baseColorToDiffuseReflectance(vec3 baseColor, float metallic) {
  vec3 diffuseReflectance = baseColor * (1.0 - metallic);
  return diffuseReflectance;
}

// フレネル項をF0とビューベクトルと法線ベクトルの内積から計算する。
vec3 Fresnel(vec3 F0, float NoV) {
  vec3 n = (1 + sqrt(F0)) / (1 - sqrt(F0) + 0.00001);
  float c = NoV;
  vec3 g2 = n * n + c * c - 1;
  vec3 g = sqrt(g2);
  vec3 f = (1 * (g - c) * (g - c)) / (2 * (g + c) * (g + c)) *
           (1 + (c * (g + c) - 1) * (c * (g + c) - 1) /
                    ((c * (g - c) + 1) * (c * (g - c) + 1)));
  return f;
}

// テクスチャとhit情報から読みだしたマテリアルのデータ。
struct MaterialData {
  vec3 baseColor;
  float metallic;
  float roughness;
  vec3 emissive;
  vec3 shadingNormal;  // world space
  vec3 geometryNormal; // world space
  float alpha;
};

// テクスチャとhit情報のPrdからマテリアルのデータを取得する。
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
  // if (material.normalTextureIndex == -1) {
  geometryNormal = normalize(prd.hitGeometryNormal);
  shadingNormal = normalize(prd.hitShadingNormal);
  if (dot(geometryNormal, viewDirection) < 0.0) {
    geometryNormal = -geometryNormal;
  }
  if (dot(shadingNormal, geometryNormal) < 0.0) {
    shadingNormal = -shadingNormal;
  }
  // } else {
  //   geometryNormal = normalize(prd.hitGeometryNormal);
  //   shadingNormal = normalize(prd.hitShadingNormal);
  //   vec3 tangent = normalize(prd.hitTangent);
  //   if (dot(geometryNormal, viewDirection) < 0.0) {
  //     geometryNormal = -geometryNormal;
  //   }
  //   if (dot(shadingNormal, geometryNormal) < 0.0) {
  //     shadingNormal = -shadingNormal;
  //     tangent = -tangent;
  //   }
  //   vec3 bitangent = cross(shadingNormal, tangent);
  //   tangent = cross(bitangent, shadingNormal);
  //   mat3 tbn = mat3(tangent, bitangent, shadingNormal);

  //   vec3 normalFromTexture =
  //       texture(images[material.normalTextureIndex], prd.hitTexCoord).rgb;
  //   normalFromTexture = normalize(normalFromTexture * 2.0 - 1.0);
  //   normalFromTexture = normalize(tbn * normalFromTexture);

  //   shadingNormal =
  //       normalize(mix(shadingNormal, normalFromTexture,
  //       material.normalFactor));
  // }

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

// BRDFの計算に必要なデータをまとめた構造体。
struct BrdfData {
  vec3 specularF0;
  vec3 diffuseReflectance;

  float alpha;

  vec3 V; // view direction in local space for shading normal

  mat3 tbn; // local space to world space for shading normal
};

// マテリアルのデータからBRDFの計算に必要なデータを取得する。
BrdfData getBrdfData(MaterialData material, vec3 viewDirection) {
  vec3 tangent;
  if (abs(dot(material.shadingNormal, vec3(0.0, 0.0, 1.0))) < 0.999) {
    tangent = normalize(cross(material.shadingNormal, vec3(0.0, 0.0, 1.0)));
  } else {
    tangent = normalize(cross(material.shadingNormal, vec3(0.0, 1.0, 0.0)));
  }
  vec3 bitangent = normalize(cross(material.shadingNormal, tangent));
  mat3 tbn = mat3(tangent, bitangent, material.shadingNormal);

  BrdfData data;
  data.specularF0 =
      baseColorToSpecularF0(material.baseColor, material.metallic);
  data.diffuseReflectance =
      baseColorToDiffuseReflectance(material.baseColor, material.metallic);
  data.alpha = material.roughness * material.roughness;

  data.V = normalize(inverse(tbn) * viewDirection);

  data.tbn = tbn;

  return data;
}

#endif
