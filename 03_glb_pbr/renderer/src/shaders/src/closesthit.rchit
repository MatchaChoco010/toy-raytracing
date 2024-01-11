#version 460
#extension GL_GOOGLE_include_directive : enable

#include "common.glsl"
#include "payload.glsl"
#include "push_constants.glsl"

layout(location = 0) rayPayloadInEXT Prd prd;

hitAttributeEXT vec2 attribs;

void main() {
  vec3 barycentricCoords =
      vec3(1.0 - attribs.x - attribs.y, attribs.x, attribs.y);

  InstanceParam instanceParam =
      GetResource(InstanceParams, pushConstants.instanceParamsIndex)
          .items[gl_InstanceID];
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

  vec2 texCoord = barycentricCoords.x * v0.texCoord +
                  barycentricCoords.y * v1.texCoord +
                  barycentricCoords.z * v2.texCoord;

  Material material = GetResource(Materials, pushConstants.materialsIndex)
                          .items[instanceParam.materialIndex];

  vec3 hitPosition = barycentricCoords.x * v0.position +
                     barycentricCoords.y * v1.position +
                     barycentricCoords.z * v2.position;
  vec3 geometryNormal =
      normalize(cross(v1.position - v0.position, v2.position - v0.position));

  prd.hitPosition = hitPosition;
  prd.hitGeometryNormal = geometryNormal;
  prd.hitShadingNormal = normal;
  prd.hitTexCoord = texCoord;
  prd.material = material;
  prd.miss = 0;
}
