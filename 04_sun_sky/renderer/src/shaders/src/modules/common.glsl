#ifndef _COMMON_GLSL_
#define _COMMON_GLSL_

#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_shader_explicit_arithmetic_types : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout : enable

#define GetLayoutVariableName(Name) u##Name##Register
#define RegisterStorage(Layout, BufferAccess, Name, Struct)                    \
  layout(Layout, set = 2, binding = 0) BufferAccess buffer Name Struct         \
  GetLayoutVariableName(Name)[]
#define GetResource(Name, Index) GetLayoutVariableName(Name)[Index]

#define PI 3.1415926535897932384626433832795

struct Material {
  vec4 baseColorFactor;
  int baseColorTextureIndex;
  vec3 emissiveFactor;
  int emissiveTextureIndex;
  float metallicFactor;
  int metallicTextureIndex;
  float roughnessFactor;
  int roughnessTextureIndex;
  float normalFactor;
  int normalTextureIndex;
  uint ty;
};

struct InstanceParam {
  uint64_t indexBuffer;
  uint64_t vertexBuffer;
  mat4 transform;
  uint materialIndex;
  uint padding1;
  uint64_t padding2;
};

struct Vertex {
  vec3 position;
  vec3 normal;
  vec3 tangent;
  vec2 texCoord;
};

// descriptor bindings
layout(set = 1, binding = 0) uniform sampler2D images[];
RegisterStorage(scalar, readonly, Materials, { Material items[]; });
RegisterStorage(scalar, readonly, InstanceParams, { InstanceParam items[]; });
layout(set = 3, binding = 0,
       rgba32f) uniform readonly image2D storageReadImages[];
layout(set = 3, binding = 0,
       rgba32f) uniform writeonly image2D storageWriteImages[];
layout(set = 4, binding = 0) uniform accelerationStructureEXT topLevelAS;

// buffer reference
layout(buffer_reference, buffer_reference_align = 4, scalar) buffer Vertices {
  Vertex v[];
};
layout(buffer_reference, scalar) buffer Indices { uvec3 i[]; };

vec3 rgbToHsv(vec3 rgb) {
  float cmax = max(rgb.r, max(rgb.g, rgb.b));
  float cmin = min(rgb.r, min(rgb.g, rgb.b));
  float diff = cmax - cmin;
  float h = 0.0;
  if (diff == 0.0) {
    h = 0.0;
  } else if (cmax == rgb.r) {
    h = mod((rgb.g - rgb.b) / diff, 6.0);
  } else if (cmax == rgb.g) {
    h = (rgb.b - rgb.r) / diff + 2.0;
  } else if (cmax == rgb.b) {
    h = (rgb.r - rgb.g) / diff + 4.0;
  }
  h /= 6.0;
  float s = cmax == 0.0 ? 0.0 : diff / cmax;
  float v = cmax;
  return vec3(h, s, v);
}

vec3 hsvToRgb(vec3 hsv) {
  float h = hsv.x * 6.0;
  float s = hsv.y;
  float v = hsv.z;
  float c = v * s;
  float x = c * (1.0 - abs(mod(h, 2.0) - 1.0));
  float m = v - c;
  vec3 rgb;
  if (h < 1.0) {
    rgb = vec3(c, x, 0.0);
  } else if (h < 2.0) {
    rgb = vec3(x, c, 0.0);
  } else if (h < 3.0) {
    rgb = vec3(0.0, c, x);
  } else if (h < 4.0) {
    rgb = vec3(0.0, x, c);
  } else if (h < 5.0) {
    rgb = vec3(x, 0.0, c);
  } else {
    rgb = vec3(c, 0.0, x);
  }
  return rgb + vec3(m);
}

#endif
