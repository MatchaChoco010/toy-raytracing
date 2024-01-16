#ifndef _COMMON_GLSL_
#define _COMMON_GLSL_

// extensions
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_shader_explicit_arithmetic_types : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout : enable

// define types

#include "push_constants.glsl"

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

#define GetLayoutVariableName(Name) u##Name##Register
#define RegisterStorage(Layout, BufferAccess, Name, Struct)                    \
  layout(Layout, set = 2, binding = 0) BufferAccess buffer Name Struct         \
  GetLayoutVariableName(Name)[]
#define GetResource(Name, Index) GetLayoutVariableName(Name)[Index]

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

// utilities

#define PI 3.1415926535897932384626433832795

float luminance(vec3 color) {
  return 0.299 * color.r + 0.587 * color.g + 0.114 * color.b;
}

// init random seed

uint seed;

void init_random(uint depth) {
  seed =
      pushConstants.sampleIndex +
      (gl_LaunchIDEXT.x + gl_LaunchSizeEXT.x * gl_LaunchIDEXT.y) * 0x12345678u +
      depth * 0x87654321u;
}

#endif
