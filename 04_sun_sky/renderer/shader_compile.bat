REM shader compile script

glslc.exe src/shaders/src/raygen.rgen -O --target-env=vulkan1.2 -o src/shaders/spv/raygen.rgen.spv

glslc.exe src/shaders/src/material/closesthit.rchit -O --target-env=vulkan1.2 -o src/shaders/spv/material/closesthit.rchit.spv
glslc.exe src/shaders/src/material/miss.rmiss -O --target-env=vulkan1.2 -o src/shaders/spv/material/miss.rmiss.spv

glslc.exe src/shaders/src/shadow/closesthit.rchit -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/closesthit.rchit.spv
glslc.exe src/shaders/src/shadow/anyhit.rahit -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/anyhit.rahit.spv
glslc.exe src/shaders/src/shadow/miss.rmiss -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/miss.rmiss.spv

glslc.exe src/shaders/src/final.comp -O --target-env=vulkan1.2 -o src/shaders/spv/final.comp.spv
