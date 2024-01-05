use ash::vk;

/// 指定したcommand_poolからPrimaryレベルのcommand bufferをallocateする関数
pub fn allocate_command_buffers(
    device: &crate::DeviceHandle,
    command_pool: &crate::CommandPoolHandle,
    count: u32,
) -> Vec<crate::CommandBufferHandle> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(**command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count);
    let command_buffers =
        device.allocate_command_buffers(&command_pool, &command_buffer_allocate_info);
    command_buffers
}

/// command bufferをリセットしてone time submit用にbeginする関数
pub fn begin_onetime_command_buffer(command_buffer: &crate::CommandBufferHandle) {
    // reset command buffer
    command_buffer.reset_command_buffer(vk::CommandBufferResetFlags::RELEASE_RESOURCES);

    // begin command buffer
    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    command_buffer.begin_command_buffer(&begin_info);
}

/// image barrierのコマンドを積むヘルパー関数
pub fn cmd_image_barriers(
    command_buffer: &crate::CommandBufferHandle,
    src_stage_mask: vk::PipelineStageFlags2,
    src_access_mask: vk::AccessFlags2,
    old_layout: vk::ImageLayout,
    dst_stage_mask: vk::PipelineStageFlags2,
    dst_access_mask: vk::AccessFlags2,
    new_layout: vk::ImageLayout,
    image: &vk::Image,
) {
    // 画像レイアウト変更のコマンドのレコード
    command_buffer.cmd_pipeline_barrier2(
        &vk::DependencyInfoKHR::builder().image_memory_barriers(std::slice::from_ref(
            &vk::ImageMemoryBarrier2::builder()
                .src_stage_mask(src_stage_mask)
                .src_access_mask(src_access_mask)
                .old_layout(old_layout)
                .dst_stage_mask(dst_stage_mask)
                .dst_access_mask(dst_access_mask)
                .new_layout(new_layout)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .image(*image),
        )),
    );
}
