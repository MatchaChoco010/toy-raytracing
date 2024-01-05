use ash::vk;

/// ComputePipelineを作成するヘルパー関数
pub fn create_compute_pipeline(
    device: &crate::DeviceHandle,
    pipeline_layout: &crate::PipelineLayoutHandle,
    shader_module: &crate::ShaderModuleHandle,
) -> crate::ComputePipelineHandle {
    let entry_name = std::ffi::CString::new("main").unwrap();
    let create_info = vk::ComputePipelineCreateInfo::builder()
        .stage(
            *vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(**shader_module)
                .name(entry_name.as_c_str()),
        )
        .layout(**pipeline_layout);
    device
        .create_compute_pipelines(
            vk::PipelineCache::null(),
            pipeline_layout,
            std::slice::from_ref(&create_info),
        )
        .into_iter()
        .next()
        .unwrap()
}
