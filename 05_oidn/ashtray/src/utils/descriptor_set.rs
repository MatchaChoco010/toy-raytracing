use crate::utils::*;
use ash::vk;

/// Bindlessの最大リソース数
pub const MAX_BINDLESS_RESOURCES: u32 = 4096;

/// BindlessなUniformBufferのDescriptorSetをまとめた構造体
pub struct DescriptorSetUniformBufferHandles {
    /// DeviceHandle
    pub device: crate::DeviceHandle,
    /// Bindlessなdescriptor setのDescriptorPoolHandle
    pub pool: crate::DescriptorPoolHandle,
    /// Bindlessなdescriptor setのDescriptorSetLayoutHandle
    pub layout: crate::DescriptorSetLayoutHandle,
    /// Bindlessなdescriptor setのDescriptorSetHandle
    pub set: crate::DescriptorSetHandle,
}
impl DescriptorSetUniformBufferHandles {
    /// BindlessなUniformBufferのDescriptorSetをまとめた構造体を作成する
    pub fn create(device: &crate::DeviceHandle) -> Self {
        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(MAX_BINDLESS_RESOURCES)
            .build()];
        let flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT];
        let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&flags)
            .build();
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .push_next(&mut binding_flags)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        let layout = device.create_descriptor_set_layout(&create_info);
        let pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .flags(
                    vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
                        | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
                )
                .max_sets(1)
                .pool_sizes(&[vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(MAX_BINDLESS_RESOURCES)
                    .build()]),
        );
        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(&[MAX_BINDLESS_RESOURCES - 1]);
        let set = device.allocate_descriptor_sets(
            &pool,
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*pool)
                .set_layouts(&[*layout])
                .push_next(&mut count_info),
        );
        let set = set.into_iter().next().unwrap();
        Self {
            device: device.clone(),
            pool,
            layout,
            set,
        }
    }

    /// 指定したarray_elementのdescriptor setを更新する
    pub fn update(&self, uniform_buffer: &crate::BufferHandle, array_element: u32) {
        // uniform bufferの書き込み
        let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(**uniform_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(*self.set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .dst_array_element(array_element)
            .buffer_info(std::slice::from_ref(&descriptor_buffer_info));

        // descriptor setの更新
        let descriptor_writes = [buffer_write.build()];
        self.device.update_descriptor_sets(&descriptor_writes);
    }
}

/// BindlessなCombinedImageSamplerのDescriptorSetをまとめた構造体
pub struct DescriptorSetCombinedImageSamplerHandles {
    /// DeviceHandle
    pub device: crate::DeviceHandle,
    /// Bindlessなdescriptor setのDescriptorPoolHandle
    pub pool: crate::DescriptorPoolHandle,
    /// Bindlessなdescriptor setのDescriptorSetLayoutHandle
    pub layout: crate::DescriptorSetLayoutHandle,
    /// Bindlessなdescriptor setのDescriptorSetHandle
    pub set: crate::DescriptorSetHandle,
}
impl DescriptorSetCombinedImageSamplerHandles {
    /// BindlessなCombinedImageSamplerのDescriptorSetをまとめた構造体を作成する
    pub fn create(device: &crate::DeviceHandle) -> Self {
        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(MAX_BINDLESS_RESOURCES)
            .build()];
        let flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT];
        let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&flags)
            .build();
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .push_next(&mut binding_flags)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        let layout = device.create_descriptor_set_layout(&create_info);
        let pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .flags(
                    vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
                        | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
                )
                .max_sets(1)
                .pool_sizes(&[vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(MAX_BINDLESS_RESOURCES)
                    .build()]),
        );
        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(&[MAX_BINDLESS_RESOURCES - 1]);
        let set = device.allocate_descriptor_sets(
            &pool,
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*pool)
                .set_layouts(&[*layout])
                .push_next(&mut count_info),
        );
        let set = set.into_iter().next().unwrap();
        Self {
            device: device.clone(),
            pool,
            layout,
            set,
        }
    }

    /// 指定したarray_elementのdescriptor setを更新する
    pub fn update(&self, image: &ImageHandles, sampler: &crate::SamplerHandle, array_element: u32) {
        // image_viewとsamplerの書き込み
        let descriptor_image_info = vk::DescriptorImageInfo::builder()
            .image_view(*image.image_view)
            .sampler(**sampler)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let image_write = vk::WriteDescriptorSet::builder()
            .dst_set(*self.set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .dst_array_element(array_element)
            .image_info(std::slice::from_ref(&descriptor_image_info));

        // descriptor setの更新
        let descriptor_writes = [image_write.build()];
        self.device.update_descriptor_sets(&descriptor_writes);
    }
}

/// BindlessなStorageBufferのDescriptorSetをまとめた構造体
pub struct DescriptorSetStorageBufferHandles {
    /// DeviceHandle
    pub device: crate::DeviceHandle,
    /// Bindlessなdescriptor setのDescriptorPoolHandle
    pub pool: crate::DescriptorPoolHandle,
    /// Bindlessなdescriptor setのDescriptorSetLayoutHandle
    pub layout: crate::DescriptorSetLayoutHandle,
    /// Bindlessなdescriptor setのDescriptorSetHandle
    pub set: crate::DescriptorSetHandle,
}
impl DescriptorSetStorageBufferHandles {
    /// BindlessなStorageBufferのDescriptorSetをまとめた構造体を作成する
    pub fn create(device: &crate::DeviceHandle) -> Self {
        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(MAX_BINDLESS_RESOURCES)
            .build()];
        let flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT];
        let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&flags)
            .build();
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .push_next(&mut binding_flags)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        let layout = device.create_descriptor_set_layout(&create_info);
        let pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .flags(
                    vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
                        | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
                )
                .max_sets(1)
                .pool_sizes(&[vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(MAX_BINDLESS_RESOURCES)
                    .build()]),
        );
        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(&[MAX_BINDLESS_RESOURCES - 1]);
        let set = device.allocate_descriptor_sets(
            &pool,
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*pool)
                .set_layouts(&[*layout])
                .push_next(&mut count_info),
        );
        let set = set.into_iter().next().unwrap();
        Self {
            device: device.clone(),
            pool,
            layout,
            set,
        }
    }

    /// 指定したarray_elementのdescriptor setを更新する
    pub fn update(&self, storage_buffer: &crate::BufferHandle, array_element: u32) {
        // storage bufferの書き込み
        let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(**storage_buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let buffer_write = vk::WriteDescriptorSet::builder()
            .dst_set(*self.set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .dst_array_element(array_element)
            .buffer_info(std::slice::from_ref(&descriptor_buffer_info));

        // descriptor setの更新
        let descriptor_writes = [buffer_write.build()];
        self.device.update_descriptor_sets(&descriptor_writes);
    }
}

/// BindlessなStorageImageのDescriptorSetをまとめた構造体
pub struct DescriptorSetStorageImageHandles {
    /// DeviceHandle
    pub device: crate::DeviceHandle,
    /// Bindlessなdescriptor setのDescriptorPoolHandle
    pub pool: crate::DescriptorPoolHandle,
    /// Bindlessなdescriptor setのDescriptorSetLayoutHandle
    pub layout: crate::DescriptorSetLayoutHandle,
    /// Bindlessなdescriptor setのDescriptorSetHandle
    pub set: crate::DescriptorSetHandle,
}
impl DescriptorSetStorageImageHandles {
    /// BindlessなStorageImageのDescriptorSetをまとめた構造体を作成する
    pub fn create(device: &crate::DeviceHandle) -> Self {
        let bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .stage_flags(vk::ShaderStageFlags::ALL)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(MAX_BINDLESS_RESOURCES)
            .build()];
        let flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT];
        let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(&flags)
            .build();
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .push_next(&mut binding_flags)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL);
        let layout = device.create_descriptor_set_layout(&create_info);
        let pool = device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .flags(
                    vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET
                        | vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND,
                )
                .max_sets(1)
                .pool_sizes(&[vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(MAX_BINDLESS_RESOURCES)
                    .build()]),
        );
        let mut count_info = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
            .descriptor_counts(&[MAX_BINDLESS_RESOURCES - 1]);
        let set = device.allocate_descriptor_sets(
            &pool,
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*pool)
                .set_layouts(&[*layout])
                .push_next(&mut count_info),
        );
        let set = set.into_iter().next().unwrap();
        Self {
            device: device.clone(),
            pool,
            layout,
            set,
        }
    }

    /// 指定したarray_elementのdescriptor setを更新する
    pub fn update(&self, storage_image: &ImageHandles, array_element: u32) {
        // storage imageの書き込み
        let storage_image_info = vk::DescriptorImageInfo::builder()
            .image_view(*storage_image.image_view)
            .image_layout(vk::ImageLayout::GENERAL);
        let storage_image_write = vk::WriteDescriptorSet::builder()
            .dst_set(*self.set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .dst_array_element(array_element)
            .image_info(std::slice::from_ref(&storage_image_info));

        // descriptor setの更新
        let descriptor_writes = [storage_image_write.build()];
        self.device.update_descriptor_sets(&descriptor_writes);
    }
}

/// AccelerationStructureのDescriptorSetをまとめた構造体
pub struct DescriptorSetAccelerationStructureHandles {
    /// descriptor setのDescriptorPoolHandle
    pub pool: crate::DescriptorPoolHandle,
    /// descriptor setのDescriptorSetLayoutHandle
    pub layout: crate::DescriptorSetLayoutHandle,
    /// descriptor setのDescriptorSetHandle
    pub set: crate::DescriptorSetHandle,
}
impl DescriptorSetAccelerationStructureHandles {
    /// AccelerationStructureのDescriptorSetをまとめた構造体を作成する
    pub fn create(
        device: &crate::DeviceHandle,
        tlas: &crate::AccelerationStructureHandle,
    ) -> DescriptorSetAccelerationStructureHandles {
        // descriptor set layoutを作成
        let layout = {
            let layout_acceleration_structure = vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1)
                .stage_flags(
                    vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                );

            let bindings = [layout_acceleration_structure.build()];

            let descriptor_set_layout_create_info =
                vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

            let descriptor_set_layout =
                device.create_descriptor_set_layout(&descriptor_set_layout_create_info);

            descriptor_set_layout
        };

        // descriptor poolの作成
        let pool = {
            let descriptor_pool_size_acceleration_structure = vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(1);
            let descriptor_pool_sizes = [descriptor_pool_size_acceleration_structure.build()];

            let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1)
                .pool_sizes(&descriptor_pool_sizes)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);

            device.create_descriptor_pool(&descriptor_pool_create_info)
        };

        // descriptor setの作成
        let descriptor_set = {
            // descriptor setのアロケート
            let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*pool)
                .set_layouts(std::slice::from_ref(&layout));
            let descriptor_set = device
                .allocate_descriptor_sets(&pool, &descriptor_set_allocate_info)
                .into_iter()
                .next()
                .unwrap();

            // acceleration structureの書き込み
            let mut descriptor_acceleration_structure_info =
                vk::WriteDescriptorSetAccelerationStructureKHR::builder()
                    .acceleration_structures(std::slice::from_ref(&tlas));
            let mut acceleration_structure_write = vk::WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .push_next(&mut descriptor_acceleration_structure_info);
            acceleration_structure_write.descriptor_count = 1;

            // descriptor setの更新
            let descriptor_writes = [acceleration_structure_write.build()];
            device.update_descriptor_sets(&descriptor_writes);

            descriptor_set
        };

        DescriptorSetAccelerationStructureHandles {
            pool,
            layout,
            set: descriptor_set,
        }
    }
}
