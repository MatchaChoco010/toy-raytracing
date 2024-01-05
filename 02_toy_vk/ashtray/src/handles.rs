//! Vulkanの各Objectを参照カウンタで管理して、参照がすべて破棄された際に
//! 自動で各種destroy処理を行うようにしたラッパーの構造体の各種Handleを用意している。
//!
//! 参照カウントの実装には「詳解 Rustアトミック操作とロック ―並行処理実装のための低レイヤプログラミング」の
//! Arcの実装を参考にしている。
//! メモリのOrderingなどは、それに準拠している。

mod instance;
pub use instance::InstanceHandle;
mod surface;
pub use surface::SurfaceHandle;
mod device;
pub use device::DeviceHandle;
mod command_pool;
pub use command_pool::CommandPoolHandle;
mod command_buffer;
pub use command_buffer::CommandBufferHandle;
mod swapchain;
pub use swapchain::SwapchainHandle;
mod image;
pub use image::ImageHandle;
mod image_view;
pub use image_view::ImageViewHandle;
mod sampler;
pub use sampler::SamplerHandle;
mod buffer;
pub use buffer::BufferHandle;
mod shader_module;
pub use shader_module::ShaderModuleHandle;
mod descriptor_pool;
pub use descriptor_pool::DescriptorPoolHandle;
mod descriptor_set_layout;
pub use descriptor_set_layout::DescriptorSetLayoutHandle;
mod descriptor_set;
pub use descriptor_set::DescriptorSetHandle;
mod pipeline_layout;
pub use pipeline_layout::PipelineLayoutHandle;
mod compute_pipeline;
pub use compute_pipeline::ComputePipelineHandle;
mod semaphore;
pub use semaphore::SemaphoreHandle;
mod fence;
pub use fence::FenceHandle;
mod acceleration_structure;
pub use acceleration_structure::AccelerationStructureHandle;
mod raytracing_pipeline;
pub use raytracing_pipeline::RayTracingPipelineHandle;

mod allocator;
pub use allocator::AllocatorHandle;
mod allocation;
pub use allocation::AllocationHandle;
