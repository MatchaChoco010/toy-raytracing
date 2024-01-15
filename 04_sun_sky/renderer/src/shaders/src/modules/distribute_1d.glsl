#ifndef _DISTRIBUTE_1D_GLSL_
#define _DISTRIBUTE_1D_GLSL_

#define DISTRIBUTE_1D_COUNT 3

// piecewise functionの分布に従ってindexをサンプリングする
float samplePdfDistribute1D(float u, float[DISTRIBUTE_1D_COUNT] func,
                            out uint index) {
  int n = func.length();
  float[DISTRIBUTE_1D_COUNT + 1] cdf;
  cdf[0] = 0.0;
  for (int i = 0; i < n; i++) {
    cdf[i + 1] = cdf[i] + func[i];
  }
  float funcSum = cdf[n];
  for (int i = 0; i < n + 1; i++) {
    cdf[i] /= funcSum;
  }

  int first = 0;
  int len = cdf.length();
  while (len > 0) {
    int h = len >> 1;
    int middle = first + h;
    if (cdf[middle] <= u) {
      first = middle + 1;
      len = len - h - 1;
    } else {
      len = h;
    }
  }
  index = clamp(first - 1, 0, n - 1);

  float pdf = func[index] / funcSum;
  return pdf;
}

float getPdfDistribute1D(float[DISTRIBUTE_1D_COUNT] func, uint index) {
  float funcSum = 0.0;
  for (int i = 0; i < func.length(); i++) {
    funcSum += func[i];
  }
  float pdf = func[index] / funcSum;
  return pdf;
}

#endif
