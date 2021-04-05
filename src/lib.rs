extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate wvr_data;

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

use reqwest::blocking::get;

use wvr_data::config::project_config::{self, SampledInput};
use wvr_data::config::server_config::ServerConfig;

pub mod config;

use config::{InputConfig, ShadertoyConfig};

pub fn create_project_from_shadertoy_url(
    wvr_data_directory: &Path,
    shadertoy_url: &str,
    api_key: &str,
) -> Option<PathBuf> {
    let shadertoy_id = shadertoy_url.split('/').last().unwrap();

    let request_url = format!(
        "https://www.shadertoy.com/api/v1/shaders/{:}?key={:}",
        shadertoy_id, api_key
    );
    let mut response = get(&request_url).unwrap();

    let shadertoy_config: ShadertoyConfig = {
        let mut shadertoy_config = String::new();
        response.read_to_string(&mut shadertoy_config).unwrap();

        serde_json::from_str(&shadertoy_config).unwrap()
    };

    let project_name = shadertoy_config.shader.info.name;

    let template_path = wvr_data_directory.join("projects").join("wvr_template");
    let project_path = wvr_data_directory.join("projects").join(project_name);
    let project_filters_path = project_path.join("filters");
    let project_config_path = project_path.join("config.toml");
    let render_chain_path = project_path.join("render_chain");

    let mut filter_list = HashMap::new();
    let mut inputs = HashMap::new();
    let mut render_chain = Vec::new();
    let mut final_stage = None;

    let mut fragment_shader_files = Vec::new();
    let mut vertex_shader_files = Vec::new();

    let stage_count = shadertoy_config.shader.renderpass.len();
    for (stage_index, render_pass) in shadertoy_config.shader.renderpass.iter().enumerate() {
        let render_stage_name = render_pass.name.clone();
        let filter_name = render_stage_name.clone();

        let vertex_shader_file_path = Path::new("render_chain")
            .join(&filter_name)
            .join("vertex")
            .join("main.glsl")
            .to_str()
            .unwrap()
            .to_owned();
        let fragment_shader_file_path = Path::new("render_chain")
            .join(&filter_name)
            .join("fragment")
            .join("main.glsl")
            .to_str()
            .unwrap()
            .to_owned();

        let mut render_pass_inputs = HashMap::new();

        for (index, input) in render_pass.inputs.iter().enumerate() {
            let uniform_name = format!("iChannel{:}", index);
            let input_name = match input {
                InputConfig::Buffer { channel } => match channel {
                    0 => "Buffer A".to_owned(),
                    1 => "Buffer B".to_owned(),
                    2 => "Buffer C".to_owned(),
                    3 => "Buffer D".to_owned(),
                    _ => unimplemented!(),
                },
                InputConfig::Webcam { .. } => {
                    inputs.insert(
                        "webcam".to_owned(),
                        project_config::InputConfig::Cam {
                            path: "/dev/video0".to_owned(),
                            width: 640,
                            height: 480,
                        },
                    );
                    "webcam".to_owned()
                }
            };

            render_pass_inputs.insert(uniform_name.clone(), SampledInput::Linear(input_name));
        }

        let filter = project_config::FilterConfig {
            path: None,
            inputs: render_pass_inputs.keys().map(String::clone).collect(),
            variables: HashMap::new(),
            vertex_shader: vec![vertex_shader_file_path.clone()],
            fragment_shader: vec![
                Path::new("render_chain")
                    .join("utils")
                    .join("header.glsl")
                    .to_str()
                    .unwrap()
                    .to_owned(),
                fragment_shader_file_path.clone(),
            ],
        };

        let render_stage = project_config::RenderStageConfig {
            name: render_stage_name.clone(),
            filter: render_stage_name.clone(),
            inputs: render_pass_inputs,
            variables: HashMap::new(),
            precision: project_config::BufferPrecision::F32,
        };

        filter_list.insert(filter_name.clone(), filter);
        vertex_shader_files.insert(0, render_stage_name.clone());
        fragment_shader_files.insert(0, (render_stage_name.clone(), render_pass.code.clone()));

        if stage_index == stage_count - 1 {
            final_stage = Some(render_stage);
        } else {
            render_chain.insert(0, render_stage);
        }
    }

    // Remove previous shadertoy project if existing
    if project_path.exists() {
        std::fs::remove_dir_all(&project_path).unwrap();
    }

    // Creating the base structure for the project
    std::fs::create_dir_all(&project_path).unwrap();
    std::fs::create_dir(&project_filters_path).unwrap();
    std::fs::create_dir(&render_chain_path).unwrap();
    std::fs::create_dir(&render_chain_path.join("utils")).unwrap();

    std::fs::copy(
        template_path
            .join("render_chain")
            .join("utils")
            .join("header.glsl"),
        render_chain_path.join("utils").join("header.glsl"),
    )
    .unwrap();

    for render_stage_name in vertex_shader_files {
        let render_stage_path = render_chain_path.join(&render_stage_name);
        if !render_stage_path.exists() {
            std::fs::create_dir(&render_stage_path).unwrap();
        }
        let vertex_shader_directory = render_stage_path.join("vertex");
        if !vertex_shader_directory.exists() {
            std::fs::create_dir(&vertex_shader_directory).unwrap();
        }

        std::fs::copy(
            template_path
                .join("render_chain")
                .join("Image")
                .join("vertex")
                .join("main.glsl"),
            vertex_shader_directory.join("main.glsl"),
        )
        .unwrap();
    }

    for (render_stage_name, shader_code) in fragment_shader_files {
        let render_stage_path = render_chain_path.join(&render_stage_name);
        if !render_stage_path.exists() {
            fs::create_dir(&render_stage_path).unwrap();
        }
        let fragment_shader_directory = render_stage_path.join("fragment");
        if !fragment_shader_directory.exists() {
            fs::create_dir(&fragment_shader_directory).unwrap();
        }

        if let Ok(mut file) = fs::File::create(fragment_shader_directory.join("main.glsl")) {
            file.write_all(&shader_code.into_bytes()).unwrap();
        }
    }

    let project_config = project_config::ProjectConfig {
        bpm: 89.0,
        view: project_config::ViewConfig {
            width: 640,
            height: 480,
            fullscreen: false,
            dynamic: true,
            vsync: true,
            screenshot: false,
            screenshot_path: PathBuf::from("output/"),
            target_fps: 60.0,
            locked_speed: false,
        },
        server: ServerConfig {
            ip: "localhost".to_owned(),
            port: 3000,
            enable: false,
        },
        inputs,
        render_chain,
        final_stage: final_stage.unwrap(),
    };

    if let Ok(mut project_config_file) = std::fs::File::create(&project_config_path) {
        project_config_file
            .write_all(
                &serde_json::ser::to_string_pretty(&project_config)
                    .unwrap()
                    .into_bytes(),
            )
            .unwrap();
    }

    for (filter_name, filter_config) in filter_list {
        let filter_config_path = project_filters_path.join(format!("{:}.ron", filter_name));
        if let Ok(mut filter_config_file) = std::fs::File::create(&filter_config_path) {
            let filter_config_string = serde_json::ser::to_string_pretty(&project_config).unwrap();
            filter_config_file
                .write_all(&filter_config_string.into_bytes())
                .unwrap();
        }
    }

    Some(project_config_path)
}
