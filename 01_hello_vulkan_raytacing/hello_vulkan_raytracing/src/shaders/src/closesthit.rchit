#version 460
#extension GL_EXT_ray_tracing : enable

layout(location = 0) rayPayloadInEXT vec3 hitValue;

hitAttributeEXT vec2 attribs;

void main() {
  vec3 barycentricCoords= vec3(
    1.0 - attribs.x - attribs.y,
    attribs.x,
    attribs.y
  );

  hitValue = barycentricCoords;
}
