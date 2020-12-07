#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "ctype")]
pub enum InputConfig {
    #[serde(rename = "buffer")]
    Buffer { channel: i64 },
    #[serde(rename = "webcam")]
    Webcam { channel: i64 },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderPassConfig {
    pub name: String,
    pub code: String,
    pub inputs: Vec<InputConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShaderInfo {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShaderConfig {
    pub info: ShaderInfo,
    pub renderpass: Vec<RenderPassConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShadertoyConfig {
    #[serde(rename = "Shader")]
    pub shader: ShaderConfig,
}
