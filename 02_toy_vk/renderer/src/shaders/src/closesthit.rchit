#version 460
#extension GL_EXT_ray_tracing : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout : enable
#extension GL_EXT_shader_explicit_arithmetic_types : enable
#extension GL_EXT_nonuniform_qualifier : enable

struct Material {
  vec3 color;
  uint ty;
};

struct Prd {
  Material material;
  uint miss;
  vec3 hitPosition;
  vec3 hitGeometryNormal;
  vec3 hitShadingNormal;
};

layout(location = 0) rayPayloadInEXT Prd prd;

layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;

struct InstanceParam {
  uint64_t indexBuffer;
  uint64_t vertexBuffer;
  mat4 transform;
  uint materialIndex;
  float lWhite;
  uint64_t padding2;
};

struct Vertex {
  vec3 position;
  vec3 normal;
};

layout(binding = 2, set = 0) readonly buffer InstanceParamsBuffer {
  InstanceParam instanceParams[];
};
layout(binding = 3, set = 0) readonly buffer MaterialsBuffer {
  Material materials[];
};

layout(push_constant) uniform PushConstants {
  mat4 cameraRotate;
  vec3 cameraTranslate;
  uint seed;
  uint maxRecursionDepth;
  uint padding1;
  uint64_t padding2;
}
pushConstants;

layout(buffer_reference, buffer_reference_align = 4, scalar) buffer Vertices {
  Vertex v[];
};
layout(buffer_reference, scalar) buffer Indices { uvec3 i[]; };

hitAttributeEXT vec2 attribs;

void main() {
  vec3 barycentricCoords =
      vec3(1.0 - attribs.x - attribs.y, attribs.x, attribs.y);

  InstanceParam instanceParam = instanceParams[gl_InstanceID];
  Indices indices = Indices(instanceParam.indexBuffer);
  Vertices vertices = Vertices(instanceParam.vertexBuffer);

  uvec3 index = indices.i[gl_PrimitiveID];
  Vertex v0 = vertices.v[index.x];
  Vertex v1 = vertices.v[index.y];
  Vertex v2 = vertices.v[index.z];

  vec3 normal = normalize(barycentricCoords.x * v0.normal +
                          barycentricCoords.y * v1.normal +
                          barycentricCoords.z * v2.normal);
  normal = transpose(inverse(mat3(instanceParam.transform))) * normal;

  Material material = materials[instanceParam.materialIndex];

  vec3 hitPosition = barycentricCoords.x * v0.position +
                     barycentricCoords.y * v1.position +
                     barycentricCoords.z * v2.position;
  vec3 geometryNormal =
      normalize(cross(v1.position - v0.position, v2.position - v0.position));

  prd.hitPosition = hitPosition;
  prd.hitGeometryNormal = geometryNormal;
  prd.hitShadingNormal = normal;
  prd.material = material;
  prd.miss = 0;
}
