REM shader compile script

glslc.exe src/shaders/src/entry/raygen.rgen -O --target-env=vulkan1.2 -o src/shaders/spv/raygen.rgen.spv

glslc.exe src/shaders/src/entry/material/closesthit.rchit -O --target-env=vulkan1.2 -o src/shaders/spv/material/closesthit.rchit.spv
glslc.exe src/shaders/src/entry/material/anyhit.rahit -O --target-env=vulkan1.2 -o src/shaders/spv/material/anyhit.rahit.spv
glslc.exe src/shaders/src/entry/material/miss.rmiss -O --target-env=vulkan1.2 -o src/shaders/spv/material/miss.rmiss.spv

glslc.exe src/shaders/src/entry/shadow/closesthit.rchit -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/closesthit.rchit.spv
glslc.exe src/shaders/src/entry/shadow/anyhit_alpha_blend.rahit -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/anyhit_alpha_blend.rahit.spv
glslc.exe src/shaders/src/entry/shadow/anyhit_alpha_mask.rahit -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/anyhit_alpha_mask.rahit.spv
glslc.exe src/shaders/src/entry/shadow/miss.rmiss -O --target-env=vulkan1.2 -o src/shaders/spv/shadow/miss.rmiss.spv

glslc.exe src/shaders/src/entry/resolve.comp -O --target-env=vulkan1.2 -o src/shaders/spv/resolve.comp.spv

glslc.exe src/shaders/src/entry/output.comp -O --target-env=vulkan1.2 -o src/shaders/spv/output.comp.spv
