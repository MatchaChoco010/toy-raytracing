//! 参照カウンタで管理して、参照がすべて破棄された際に
//! Deviceの破棄の処理まで行うDeviceHandleを定義する。

use anyhow::Result;
#[cfg(target_os = "linux")]
use ash::extensions::khr::ExternalMemoryFd;
#[cfg(target_os = "windows")]
use ash::extensions::khr::ExternalMemoryWin32;
use ash::{
    extensions::khr::{AccelerationStructure, RayTracingPipeline, Swapchain},
    vk,
};
use std::{
    fmt::Debug,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{fence, AtomicUsize, Ordering},
};

struct DeviceHandleData {
    instance: crate::InstanceHandle,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    swapchain_loader: Swapchain,
    acceleration_structure_loader: AccelerationStructure,
    ray_tracing_pipeline_loader: RayTracingPipeline,
    #[cfg(target_os = "windows")]
    external_memory_win32: ExternalMemoryWin32,
    #[cfg(target_os = "linux")]
    external_memory_fd: ExternalMemoryFd,
    ref_count: AtomicUsize,
}
impl DeviceHandleData {
    fn new(
        instance: crate::InstanceHandle,
        physical_device: vk::PhysicalDevice,
        device_create_info: &vk::DeviceCreateInfo,
    ) -> Result<Self> {
        // create device
        let device = unsafe {
            ash::Instance::create_device(&instance, physical_device, device_create_info, None)?
        };

        // swapchain loader
        let swapchain_loader = Swapchain::new(&instance, &device);

        // acceleration structure loader
        let acceleration_structure_loader = AccelerationStructure::new(&instance, &device);

        // ray_tracing pipeline loader
        let ray_tracing_pipeline_loader = RayTracingPipeline::new(&instance, &device);

        // external memory win32
        #[cfg(target_os = "windows")]
        let external_memory_win32 = ExternalMemoryWin32::new(&instance, &device);

        // external memory fd
        #[cfg(target_os = "linux")]
        let external_memory_fd = ExternalMemoryFd::new(&instance, &device);

        Ok(Self {
            instance,
            physical_device,
            device,
            swapchain_loader,
            acceleration_structure_loader,
            ray_tracing_pipeline_loader,
            #[cfg(target_os = "windows")]
            external_memory_win32,
            #[cfg(target_os = "linux")]
            external_memory_fd,
            ref_count: AtomicUsize::new(1),
        })
    }
}

/// ash::Deviceを参照カウントで管理するためのハンドル
pub struct DeviceHandle {
    ptr: NonNull<DeviceHandleData>,
}
impl DeviceHandle {
    pub(crate) fn new(
        instance_handle: crate::InstanceHandle,
        physical_device: vk::PhysicalDevice,
        device_create_info: &vk::DeviceCreateInfo,
    ) -> Result<Self> {
        let data = Box::new(DeviceHandleData::new(
            instance_handle,
            physical_device,
            device_create_info,
        )?);
        let ptr = unsafe { NonNull::new_unchecked(Box::into_raw(data)) };
        Ok(Self { ptr })
    }

    // Deviceの関数

    // create系

    /// SwapchainHandleを作成する
    pub fn create_swapchain(
        &self,
        swapchain_create_info: &vk::SwapchainCreateInfoKHR,
    ) -> crate::SwapchainHandle {
        crate::SwapchainHandle::new(self.clone(), swapchain_create_info)
    }

    /// CommandPoolHandleを作成する
    pub fn create_command_pool(
        &self,
        command_pool_create_info: &vk::CommandPoolCreateInfo,
    ) -> crate::CommandPoolHandle {
        crate::CommandPoolHandle::new(self.clone(), command_pool_create_info)
    }

    /// CommandBufferHandleを割り当てる
    pub fn allocate_command_buffers(
        &self,
        command_pool_handle: &crate::CommandPoolHandle,
        allocate_info: &vk::CommandBufferAllocateInfo,
    ) -> Vec<crate::CommandBufferHandle> {
        crate::CommandBufferHandle::allocate(
            self.clone(),
            command_pool_handle.clone(),
            allocate_info,
        )
    }

    /// AllocatorHandleを作成する
    pub fn create_allocator(
        &self,
        allocator_create_desc: &gpu_allocator::vulkan::AllocatorCreateDesc,
    ) -> crate::AllocatorHandle {
        crate::AllocatorHandle::new(self.clone(), allocator_create_desc)
    }

    /// ImageHandleを作成する
    pub fn create_image(&self, image_create_info: &vk::ImageCreateInfo) -> crate::ImageHandle {
        crate::ImageHandle::new(self.clone(), image_create_info)
    }

    /// ImageViewHandleを作成する
    pub fn create_image_view(
        &self,
        image_view_create_info: &vk::ImageViewCreateInfo,
    ) -> crate::ImageViewHandle {
        crate::ImageViewHandle::new(self.clone(), image_view_create_info)
    }

    /// SamplerHandleを作成する
    pub fn create_sampler(
        &self,
        sampler_create_info: &vk::SamplerCreateInfo,
    ) -> crate::SamplerHandle {
        crate::SamplerHandle::new(self.clone(), sampler_create_info)
    }

    /// BufferHandleを作成する
    pub fn create_buffer(&self, buffer_create_info: &vk::BufferCreateInfo) -> crate::BufferHandle {
        crate::BufferHandle::new(self.clone(), buffer_create_info)
    }

    /// ShaderModuleHandleを作成する
    pub fn create_shader_module(
        &self,
        shader_module_create_info: &vk::ShaderModuleCreateInfo,
    ) -> crate::ShaderModuleHandle {
        crate::ShaderModuleHandle::new(self.clone(), shader_module_create_info)
    }

    /// DescriptorPoolHandleを作成する
    pub fn create_descriptor_pool(
        &self,
        descriptor_pool_create_info: &vk::DescriptorPoolCreateInfo,
    ) -> crate::DescriptorPoolHandle {
        crate::DescriptorPoolHandle::new(self.clone(), descriptor_pool_create_info)
    }

    /// DescriptorSetLayoutHandleを作成する
    pub fn create_descriptor_set_layout(
        &self,
        descriptor_set_layout_create_info: &vk::DescriptorSetLayoutCreateInfo,
    ) -> crate::DescriptorSetLayoutHandle {
        crate::DescriptorSetLayoutHandle::new(self.clone(), descriptor_set_layout_create_info)
    }

    /// DescriptorSetHandleを割り当てる
    pub fn allocate_descriptor_sets(
        &self,
        descriptor_pool_handle: &crate::DescriptorPoolHandle,
        descriptor_set_allocate_info: &vk::DescriptorSetAllocateInfo,
    ) -> Vec<crate::DescriptorSetHandle> {
        crate::DescriptorSetHandle::new(
            self.clone(),
            descriptor_pool_handle,
            descriptor_set_allocate_info,
        )
    }

    /// PipelineLayoutHandleを作成する
    pub fn create_pipeline_layout(
        &self,
        pipeline_layout_create_info: &vk::PipelineLayoutCreateInfo,
    ) -> crate::PipelineLayoutHandle {
        crate::PipelineLayoutHandle::new(self.clone(), pipeline_layout_create_info)
    }

    /// ComputePipelineHandleを作成する
    pub fn create_compute_pipelines(
        &self,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::ComputePipelineCreateInfo],
    ) -> Vec<crate::ComputePipelineHandle> {
        crate::ComputePipelineHandle::new(self.clone(), pipeline_cache, create_infos)
    }

    /// RayTracingPipelineHandleを作成する
    pub fn create_ray_tracing_pipelines(
        &self,
        deferred_operation: vk::DeferredOperationKHR,
        pipeline_cache: vk::PipelineCache,
        create_infos: &[vk::RayTracingPipelineCreateInfoKHR],
    ) -> Vec<crate::RayTracingPipelineHandle> {
        crate::RayTracingPipelineHandle::new(
            self.clone(),
            deferred_operation,
            pipeline_cache,
            create_infos,
        )
    }

    /// SemaphoreHandleを作成する
    pub fn create_semaphore(
        &self,
        semaphore_create_info: &vk::SemaphoreCreateInfo,
    ) -> crate::SemaphoreHandle {
        crate::SemaphoreHandle::new(self.clone(), semaphore_create_info)
    }

    /// FenceHandleを作成する
    pub fn create_fence(&self, fence_create_info: &vk::FenceCreateInfo) -> crate::FenceHandle {
        crate::FenceHandle::new(self.clone(), fence_create_info)
    }

    /// AccelerationStructureHandleを作成する
    pub fn create_acceleration_structure(
        &self,
        acceleration_structure_create_info: &vk::AccelerationStructureCreateInfoKHR,
    ) -> crate::AccelerationStructureHandle {
        crate::AccelerationStructureHandle::new(self.clone(), acceleration_structure_create_info)
    }

    // 他のDeviceの関数

    /// vk::Queueを取得する
    pub fn get_device_queue(&self, queue_family_index: u32, queue_index: u32) -> vk::Queue {
        unsafe {
            self.data()
                .device
                .get_device_queue(queue_family_index, queue_index)
        }
    }

    /// swapchainのイメージを取得する
    pub fn get_swapchain_images(&self, swapchain: &crate::SwapchainHandle) -> Vec<vk::Image> {
        unsafe {
            self.data()
                .swapchain_loader
                .get_swapchain_images(swapchain.swapchain_raw())
                .expect("Failed to get swapchain images.")
        }
    }

    /// Bufferのデバイスアドレスを取得する
    pub fn get_buffer_device_address(
        &self,
        buffer_device_address_info: &vk::BufferDeviceAddressInfo,
    ) -> vk::DeviceAddress {
        unsafe {
            self.data()
                .device
                .get_buffer_device_address(buffer_device_address_info)
        }
    }

    /// external memory win32のハンドルを取得する
    #[cfg(target_os = "windows")]
    pub fn get_memory_win32_handle(
        &self,
        memory_get_win32_handle_info: &vk::MemoryGetWin32HandleInfoKHR,
    ) -> *mut std::ffi::c_void {
        unsafe {
            self.data()
                .external_memory_win32
                .get_memory_win32_handle(memory_get_win32_handle_info)
                .expect("Failed to get memory win32 handle.")
        }
    }

    /// external memory fdのハンドルを取得する
    #[cfg(target_os = "linux")]
    pub fn get_memory_fd(
        &self,
        memory_get_fd_info: &vk::MemoryGetFdInfoKHR,
    ) -> std::os::raw::c_int {
        unsafe {
            self.data()
                .external_memory_fd
                .get_memory_fd(memory_get_fd_info)
                .expect("Failed to get memory fd.")
        }
    }

    /// DescriptorSetの更新をする
    pub fn update_descriptor_sets(&self, write_descriptor_sets: &[vk::WriteDescriptorSet]) {
        unsafe {
            self.data()
                .device
                .update_descriptor_sets(write_descriptor_sets, &[])
        }
    }

    /// 次の表示イメージを取得する
    pub fn acquire_next_image(
        &self,
        swapchain: &crate::SwapchainHandle,
        timeout: u64,
        semaphore: Option<crate::SemaphoreHandle>,
        fence: Option<crate::FenceHandle>,
    ) -> Result<(u32, bool), vk::Result> {
        unsafe {
            let semaphore = semaphore
                .map(|s| s.semaphore_raw())
                .unwrap_or(vk::Semaphore::null());
            let fence = fence.map(|f| f.fence_raw()).unwrap_or(vk::Fence::null());
            self.data().swapchain_loader.acquire_next_image(
                swapchain.swapchain_raw(),
                timeout,
                semaphore,
                fence,
            )
        }
    }

    /// QueueにコマンドをSubmitする
    pub fn queue_submit(
        &self,
        queue: vk::Queue,
        submit_infos: &[vk::SubmitInfo],
        fence: Option<crate::FenceHandle>,
    ) {
        unsafe {
            self.data()
                .device
                .queue_submit(
                    queue,
                    submit_infos,
                    fence.map(|f| f.fence_raw()).unwrap_or(vk::Fence::null()),
                )
                .expect("Failed to submit queue.");
        }
    }

    /// Queueにpresentを行う
    pub fn queue_present(
        &self,
        queue: vk::Queue,
        present_info: &vk::PresentInfoKHR,
    ) -> Result<bool, vk::Result> {
        unsafe {
            self.data()
                .swapchain_loader
                .queue_present(queue, present_info)
        }
    }

    /// DeviceのIdleを待機する
    pub fn wait_idle(&self) {
        unsafe {
            self.data()
                .device
                .device_wait_idle()
                .expect("Failed to wait device idle.")
        }
    }

    /// Fenceをリセットする
    pub fn reset_fences(&self, fences: &[crate::FenceHandle]) {
        unsafe {
            let fences = fences
                .iter()
                .map(|fence| fence.fence_raw())
                .collect::<Vec<_>>();
            self.data()
                .device
                .reset_fences(&fences)
                .expect("Failed to reset fences.")
        }
    }

    /// Fenceを待機する
    pub fn wait_fences(&self, fences: &[crate::FenceHandle], timeout: u64) {
        unsafe {
            let fences = fences
                .iter()
                .map(|fence| fence.fence_raw())
                .collect::<Vec<_>>();
            self.data()
                .device
                .wait_for_fences(&fences, true, timeout)
                .expect("Failed to wait fence.")
        }
    }

    /// AccelerationStructureのビルドサイズを取得する
    pub fn get_acceleration_structure_build_sizes(
        &self,
        build_type: vk::AccelerationStructureBuildTypeKHR,
        build_geometry_info: &vk::AccelerationStructureBuildGeometryInfoKHR,
        max_primitive_counts: &[u32],
    ) -> vk::AccelerationStructureBuildSizesInfoKHR {
        unsafe {
            self.data()
                .acceleration_structure_loader
                .get_acceleration_structure_build_sizes(
                    build_type,
                    build_geometry_info,
                    max_primitive_counts,
                )
        }
    }

    /// physical device memory propertiesを取得する
    pub fn get_physical_device_memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        self.data()
            .instance
            .get_physical_device_memory_properties(self.data().physical_device)
    }

    // raw

    /// InstanceHandleを取得する
    pub fn instance(&self) -> crate::InstanceHandle {
        self.data().instance.clone()
    }

    /// Deviceを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したDeviceは無効になる。
    pub unsafe fn device_raw(&self) -> ash::Device {
        self.data().device.clone()
    }

    /// Swapchainを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したSwapchainは無効になる。
    pub unsafe fn swapchain_loader_raw(&self) -> Swapchain {
        self.data().swapchain_loader.clone()
    }

    /// AccelerationStructureLoaderを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したAccelerationStructureLoaderは無効になる。
    pub unsafe fn acceleration_structure_loader_raw(&self) -> AccelerationStructure {
        self.data().acceleration_structure_loader.clone()
    }

    /// RayTracingPipelineLoaderを取得する
    /// ## Safety
    /// 参照カウントの管理から中身を取り出すので注意。
    /// Handleが破棄されると、この関数で取り出したRayTracingPipelineLoaderは無効になる。
    pub unsafe fn ray_tracing_pipeline_loader_raw(&self) -> RayTracingPipeline {
        self.data().ray_tracing_pipeline_loader.clone()
    }

    fn data(&self) -> &DeviceHandleData {
        unsafe { self.ptr.as_ref() }
    }
}

// Debugトレイトの実装
impl Debug for DeviceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceHandle").finish()
    }
}

// DeviceHandleDataの中身はSendかつSyncなのでDeviceHandleはSend
unsafe impl Send for DeviceHandle {}
// DeviceHandleDataの中身はSendかつSyncなのでDeviceHandleはSync
unsafe impl Sync for DeviceHandle {}

// DeviceHandleはash::DeviceにDerefする
impl Deref for DeviceHandle {
    type Target = ash::Device;
    fn deref(&self) -> &Self::Target {
        &self.data().device
    }
}

// Cloneで参照カウントを増やす
impl Clone for DeviceHandle {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            panic!("Too many references to DeviceHandle");
        }
        Self { ptr: self.ptr }
    }
}

// Drop時に参照カウントを減らし、0になったら破棄する
impl Drop for DeviceHandle {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            self.wait_idle();
            unsafe {
                let data = Box::from_raw(self.ptr.as_ptr());

                // Deviceの破棄
                data.device.destroy_device(None);
            }
        }
    }
}
