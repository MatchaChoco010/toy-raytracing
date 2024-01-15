#ifndef _LIGHT_GLSL_
#define _LIGHT_GLSL_

#include "common.glsl"
#include "push_constants.glsl"

vec3 sunDirection() {
  float phi = pushConstants.sunDirection.x;
  float theta = pushConstants.sunDirection.y;
  return vec3(sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi));
}

// world space
bool isSunDirection(vec3 direction) {
  return dot(direction, sunDirection()) > cos(1 - pushConstants.sunAngle / 2);
}

// world space
float getSunPdf(vec3 direction) {
  if (!isSunDirection(direction)) {
    return 0.0;
  }

  float sunSolidAngle = 2 * PI * (1 - cos(pushConstants.sunAngle));
  return 1.0 / sunSolidAngle;
}

// world space
vec3 sampleSunDirection(float[2] u) {
  float theta = acos(1 - pushConstants.sunAngle / 2) * u[0];
  float phi = 2 * PI * u[1];
  vec3 dir = vec3(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));

  vec3 sunDirection = sunDirection();

  vec3 tangent;
  if (abs(dot(sunDirection, vec3(0.0, 0.0, 1.0))) < 0.999) {
    tangent = normalize(cross(sunDirection, vec3(0.0, 0.0, 1.0)));
  } else {
    tangent = normalize(cross(sunDirection, vec3(0.0, 1.0, 0.0)));
  }
  vec3 bitangent = cross(sunDirection, tangent);
  mat3 tbn = mat3(tangent, bitangent, sunDirection);

  return tbn * dir;
}

vec3 getSunStrength() {
  return pushConstants.sunStrength * pushConstants.sunColor;
}

#endif
