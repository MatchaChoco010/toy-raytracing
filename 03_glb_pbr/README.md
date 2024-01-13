# 03_glb_pbr

glbを並べて描画するテスト。

リアルタイムレンダリングに近い挙動にするため、半透明の扱いはalphaブレンドでもglTFのtransmissionでもなく、色付きでレイの向きを変えない透過としている。

```
MIN_DIELECTRICS_F0 = 0.04
F0 = mix(vec3(MIN_DIELECTRICS_F0), baseColor, metallic)
kD = (1.0 - luminance(Fresnel(F0, NoV))) * (1.0 - metallic)
BSDF = 1.0 / (1.0 + kD) * GGX + kD / (1.0 + kD) * NormalizedLambert + (1.0 - alpha) * TransparentBTDF
```

透過BTDFの透過の色はglTFのtransmission拡張を使ってユーザーが与えるべき値だけど、今回は拡張を使わずにbaseColorとalphaから適当に決める。
厚さ1mでbaseColorだけ吸収する材質をalpha(m)の厚さだけ通り抜けたときに吸収される値を適当に透過色として決めた。

```
cargo run --release
```

![screenshot](./screenshot.png)
