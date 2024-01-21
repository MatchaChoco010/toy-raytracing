use ash::vk;

/// bytesを与えてShaderModuleを作成するヘルパー関数
pub fn create_shader_module(
    device: &crate::DeviceHandle,
    bytes: &[u8],
) -> crate::ShaderModuleHandle {
    let words = bytes
        .chunks_exact(4)
        .map(|x| x.try_into().unwrap())
        .map(match bytes[0] {
            0x03 => u32::from_le_bytes,
            0x07 => u32::from_be_bytes,
            _ => panic!("Unknown endianness"),
        })
        .collect::<Vec<u32>>();
    let create_info = vk::ShaderModuleCreateInfo::builder().code(&words);
    device.create_shader_module(&create_info)
}
