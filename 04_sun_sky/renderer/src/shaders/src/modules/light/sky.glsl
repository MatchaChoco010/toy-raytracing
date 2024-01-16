#ifndef _LIGHT_SKY_GLSL_
#define _LIGHT_SKY_GLSL_

#include "../common.glsl"

// cosine weighted hemisphereな方向サンプリング。
// 引数のnormal及び返り値の方向はworld space。
vec3 sampleSkyDirection(float[2] uu, vec3 normal) {
  float up = sqrt(uu[0]);
  float over = sqrt(1.0 - up * up);
  float around = uu[1] * 2 * PI;
  vec3 u = normalize(abs(normal.x) < 0.999 ? cross(normal, vec3(1, 0, 0))
                                           : cross(normal, vec3(0, 1, 0)));
  vec3 v = cross(normal, u);
  return normalize(u * cos(around) * over + v * sin(around) * over +
                   normal * up);
}

// Skyの方向のサンプリングに対応したpdfを返す。
// 引数のnormalとdirectionはworld space。
float getSkyPdf(vec3 normal, vec3 direction) {
  return max(dot(normal, direction), 0) / PI;
}

// skyのテクスチャからdirectionの方向の放射輝度を線形補間して取得する。
// directionはworld space。
vec3 getSkyColor(vec3 direction) {
  float theta = acos(direction.y);
  float phi = atan(direction.z, direction.x) - PI + pushConstants.skyRotation;
  while (phi < 0.0) {
    phi += 2.0 * PI;
  }
  float x = phi / (2.0 * PI) * pushConstants.skyWidth;
  float y = theta / PI * pushConstants.skyHeight;
  uint x1 = clamp(uint(x), 0, pushConstants.skyWidth - 1);
  uint x2 = clamp(uint(x + 1.0) % (pushConstants.skyWidth - 1), 0,
                  pushConstants.skyWidth - 1);
  uint y1 = clamp(uint(y), 0, pushConstants.skyHeight - 1);
  uint y2 = clamp(uint(y + 1.0) % (pushConstants.skyHeight - 1), 0,
                  pushConstants.skyHeight - 1);

  float weightX = x - floor(x);
  float weightY = y - floor(y);

  SkyBuffer skyBuffer = SkyBuffer(pushConstants.skyBufferAddress);
  vec3 color1 = skyBuffer.pixel[y1 * pushConstants.skyWidth + x1];
  vec3 color2 = skyBuffer.pixel[y1 * pushConstants.skyWidth + x2];
  vec3 color3 = skyBuffer.pixel[y2 * pushConstants.skyWidth + x1];
  vec3 color4 = skyBuffer.pixel[y2 * pushConstants.skyWidth + x2];

  return pushConstants.skyStrength *
         (color1 * (1.0 - weightX) * (1.0 - weightY) +
          color2 * weightX * (1.0 - weightY) +
          color3 * (1.0 - weightX) * weightY + color4 * weightX * weightY);
}

// skyのテクスチャからdirectionの方向の放射輝度を取得する。
// directionはworld space。
vec3 getSkyStrength(vec3 direction) {
  float theta = acos(direction.y);
  float phi = atan(direction.z, direction.x) - PI + pushConstants.skyRotation;
  while (phi < 0.0) {
    phi += 2.0 * PI;
  }
  uint x = clamp(uint(phi / (2.0 * PI) * pushConstants.skyWidth), 0,
                 pushConstants.skyWidth - 1);
  uint y = clamp(uint(theta / PI * pushConstants.skyHeight), 0,
                 pushConstants.skyHeight - 1);

  SkyBuffer skyBuffer = SkyBuffer(pushConstants.skyBufferAddress);
  return pushConstants.skyStrength *
         skyBuffer.pixel[y * pushConstants.skyWidth + x];
}

#endif
