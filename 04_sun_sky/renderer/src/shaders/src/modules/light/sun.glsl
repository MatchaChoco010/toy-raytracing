#ifndef _LIGHT_SUN_GLSL_
#define _LIGHT_SUN_GLSL_

#include "../common.glsl"

// push constantsで渡された仰角と方位角からworld spaceの太陽の方向を計算する
vec3 sunDirection() {
  float phi = pushConstants.sunDirection.x;
  float theta = -pushConstants.sunDirection.y + PI / 2;
  return vec3(sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi));
}

// 与えられたdirectionが太陽の立体角に含まれるかを判定する。
// 引数のdirectionはworld space。
bool isSunDirection(vec3 direction) {
  return dot(direction, sunDirection()) >=
         cos(pushConstants.sunAngle / 2 + 0.0001);
}

// 頂角がsunAngleの円錐の立体角の中から一様にランダムに方向をサンプリングして方向を返す。
// 返される方向はworld space。
// thetaとphiはそれぞれ逆関数法で求めた[0, 1)の範囲の乱数の変換方法。
vec3 sampleSunDirection(float[2] u) {
  float theta = acos(1 - u[0] + u[0] * cos(pushConstants.sunAngle / 2));
  float phi = 2 * PI * u[1];
  vec3 w = vec3(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta));

  vec3 sunDirection = sunDirection();
  vec3 tangent;
  if (abs(dot(sunDirection, vec3(0.0, 0.0, 1.0))) < 0.999) {
    tangent = normalize(cross(sunDirection, vec3(0.0, 0.0, 1.0)));
  } else {
    tangent = normalize(cross(sunDirection, vec3(0.0, 1.0, 0.0)));
  }
  vec3 bitangent = cross(sunDirection, tangent);
  mat3 tbn = mat3(tangent, bitangent, sunDirection);

  return tbn * w;
}

// 太陽の方向のサンプリングに対応したpdfを返す。
// 引数のdirectionはworld space。
float getSunPdf(vec3 direction) {
  if (!isSunDirection(direction)) {
    return 0.0;
  }
  return 1.0 / (2 * PI * (1 - cos(pushConstants.sunAngle / 2)));
}

// 太陽の垂直放射照度(W/m^2)と色から、太陽の放射輝度(W/m^2/sr)を計算する。
// 放射輝度にcos(delta)をかけて天球で積分をすると放射照度になる。
// ここでdeltaは天球上の方向に垂直な平面とy-upな平面のなす角。
// したがって、delta = PI/2 - thetaとして
// 太陽の方向と放射輝度の方向のなす角thetaに変換すると
// sin(theta)かけて放射輝度を積分する形になる。
// 太陽の立体角の中で放射輝度が一定であるとすると、放射輝度以外の部分の係数は
// sin(theta)を0<=theta<sunAngle/2, 0<=phi<2*PIで積分したとなる。
// これは4 * PI * (sin(sunAngle/4))^2となるので、
// 垂直放射照度から放射輝度を求めるには
// 垂直放射照度を 4 * PI * (sin(sunAngle/4))^2で割れば良い。
vec3 getSunStrength() {
  return pushConstants.sunStrength * pushConstants.sunColor /
         (4 * PI * sin(pushConstants.sunAngle / 4) *
          sin(pushConstants.sunAngle / 4));
}

#endif
