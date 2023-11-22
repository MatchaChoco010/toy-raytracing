REM shader compile script
glslc.exe src/shaders/src/raygen.rgen -O --target-env=vulkan1.2 -o src/shaders/spv/raygen.rgen.spv
glslc.exe src/shaders/src/miss.rmiss -O --target-env=vulkan1.2 -o src/shaders/spv/miss.rmiss.spv
glslc.exe src/shaders/src/closesthit.rchit -O --target-env=vulkan1.2 -o src/shaders/spv/closesthit.rchit.spv
