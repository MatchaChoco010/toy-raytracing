#ifndef _LIGHT_SKY_GLSL_
#define _LIGHT_SKY_GLSL_

#include "../common.glsl"

// Skyの方向のサンプリングに対応したpdfを返す。
// 引数のnormalとdirectionはworld space。
float getSkyPdf(vec3 direction) {
  float theta = acos(direction.y);
  float phi = atan(direction.x, direction.z) + pushConstants.skyRotation;
  while (phi < 0.0) {
    phi += 2.0 * PI;
  }
  while (phi >= 2.0 * PI) {
    phi -= 2.0 * PI;
  }
  uint x = clamp(uint(phi / (2.0 * PI) * pushConstants.skyWidth), 0,
                 (pushConstants.skyWidth) - 1);
  uint y = clamp(uint(theta / PI * pushConstants.skyHeight), 0,
                 (pushConstants.skyHeight) - 1);

  SkyPdfBuffer pdfColumn =
      SkyPdfBuffer(pushConstants.skyPdfColumnBufferAddress);
  SkyPdfBuffer pdfRow = SkyPdfBuffer(pushConstants.skyPdfRowBufferAddress);

  float pdfY = pdfColumn.p[y];
  float pdfX = pdfRow.p[y * pushConstants.skyWidth + x];

  float pdfPhi = pdfX * pushConstants.skyWidth;
  float pdfTheta = pdfY * pushConstants.skyHeight;

  float pdf = pdfPhi * pdfTheta;
  // // Jacobian for uv -> direction
  pdf /= 2.0 * PI * PI * sin(theta) + 0.00001;

  return pdf;
}

// skyのテクスチャからdirectionの方向の放射輝度を線形補間して取得する。
// directionはworld space。
vec3 getSkyColor(vec3 direction) {
  float theta = acos(direction.y);
  float phi = atan(direction.x, direction.z) + pushConstants.skyRotation;
  while (phi < 0.0) {
    phi += 2.0 * PI;
  }
  while (phi >= 2.0 * PI) {
    phi -= 2.0 * PI;
  }
  float x = phi / (2.0 * PI) * pushConstants.skyWidth;
  float y = theta / PI * pushConstants.skyHeight;
  uint x1 = clamp(uint(x) % (pushConstants.skyWidth - 1), 0,
                  pushConstants.skyWidth - 1);
  uint x2 = clamp(uint(x + 1.0) % (pushConstants.skyWidth - 1), 0,
                  pushConstants.skyWidth - 1);
  uint y1 = clamp(uint(y), 0, pushConstants.skyHeight - 1);
  uint y2 = clamp(uint(y + 1.0), 0, pushConstants.skyHeight - 1);

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
  float phi = atan(direction.x, direction.z) + pushConstants.skyRotation;
  while (phi < 0.0) {
    phi += 2.0 * PI;
  }
  while (phi >= 2.0 * PI) {
    phi -= 2.0 * PI;
  }
  uint x = clamp((uint(phi / (2.0 * PI) * pushConstants.skyWidth)) %
                     (pushConstants.skyWidth - 1),
                 0, pushConstants.skyWidth - 1);
  uint y = clamp(uint(theta / PI * pushConstants.skyHeight), 0,
                 pushConstants.skyHeight - 1);

  SkyBuffer skyBuffer = SkyBuffer(pushConstants.skyBufferAddress);
  return pushConstants.skyStrength *
         skyBuffer.pixel[y * pushConstants.skyWidth + x];
}

void sampleSky(float[2] u, out vec3 direction, out float pdf,
               out vec3 radiance) {
  uint y;
  float pdfY;
  {
    SkyCdfBuffer cdfColumn =
        SkyCdfBuffer(pushConstants.skyCdfColumnBufferAddress);
    SkyPdfBuffer pdfColumn =
        SkyPdfBuffer(pushConstants.skyPdfColumnBufferAddress);
    uint first = 0;
    uint len = pushConstants.skyHeight + 1;
    while (len > 0) {
      uint h = len >> 1;
      uint middle = first + h;
      if (cdfColumn.value[middle] <= u[0]) {
        first = middle + 1;
        len = len - h - 1;
      } else {
        len = h;
      }
    }
    y = clamp(first - 1, 0, pushConstants.skyHeight - 1);
    pdfY = pdfColumn.p[y];
  }

  uint x;
  float pdfX;
  {
    SkyCdfBuffer cdfRow = SkyCdfBuffer(pushConstants.skyCdfRowBufferAddress);
    SkyPdfBuffer pdfRow = SkyPdfBuffer(pushConstants.skyPdfRowBufferAddress);
    uint first = 0;
    uint len = pushConstants.skyWidth + 1;
    while (len > 0) {
      uint h = len >> 1;
      uint middle = first + h;
      if (cdfRow.value[y * (pushConstants.skyWidth + 1) + middle] <= u[1]) {
        first = middle + 1;
        len = len - h - 1;
      } else {
        len = h;
      }
    }
    x = clamp(first - 1, 0, pushConstants.skyWidth - 1);
    pdfX = pdfRow.p[y * pushConstants.skyWidth + x];
  }

  float theta = float(y) / pushConstants.skyHeight * PI;
  float phi = (float(x) / (pushConstants.skyWidth - 1)) * 2.0 * PI -
              pushConstants.skyRotation;
  while (phi < 0.0) {
    phi += 2.0 * PI;
  }
  while (phi >= 2.0 * PI) {
    phi -= 2.0 * PI;
  }
  direction = vec3(sin(theta) * sin(phi), cos(theta), sin(theta) * cos(phi));

  float pdfPhi = pdfX * pushConstants.skyWidth;
  float pdfTheta = pdfY * pushConstants.skyHeight;

  pdf = pdfPhi * pdfTheta;
  // // Jacobian for uv -> direction
  pdf /= 2.0 * PI * PI * sin(theta) + 0.00001;

  SkyBuffer skyBuffer = SkyBuffer(pushConstants.skyBufferAddress);
  radiance = pushConstants.skyStrength *
             skyBuffer.pixel[y * pushConstants.skyWidth + x];
}

#endif
