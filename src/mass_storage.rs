use std::path::PathBuf;

use axum::{extract, Json};
use serde::{Deserialize, Serialize};

use crate::api_error;

#[derive(Serialize)]
pub struct ImageBlock {
    offset: usize,
    size: usize,
    checksum: [u8; 0x10],
}

#[derive(Serialize)]
pub struct Image {
    name: String,
    blocks: Vec<ImageBlock>,
}

const IP_KVM_IMAGES_PATH: &str = "ip-kvm-images";

fn get_image_path(file_name: &String) -> api_error::Result<PathBuf> {
    let file_path = if let Ok(file_path) = std::fs::canonicalize(file_name) {
        file_path
    } else {
        return Err(anyhow::anyhow!("Canonicalize {file_name:?} failed!"))?;
    };
    let images_path = if let Ok(images_path) = std::fs::canonicalize(IP_KVM_IMAGES_PATH) {
        images_path
    } else {
        return Err(anyhow::anyhow!("Canonicalize {IP_KVM_IMAGES_PATH:?} failed!"))?;
    };
    if !file_path.starts_with(images_path) {
        return Err(anyhow::anyhow!("Invalid file_name:{file_name:?}."))?;
    }
    Ok(file_path)
}

#[derive(Deserialize)]
pub struct CurrentImageInput {
    image_name: String,
}

pub async fn put_current_image(Json(payload): Json<CurrentImageInput>) -> api_error::Result<String> {
    let file_path = get_image_path(&payload.image_name)?;
    Ok("null".into())
}

pub async fn get_images() -> api_error::Result<Json<Vec<String>>> {
    todo!()
}

pub async fn get_image(extract::Path(file_name): extract::Path<String>) -> api_error::Result<Json<Image>> {
    let file_path = get_image_path(&file_name)?;
    todo!()
}

pub async fn delete_image(extract::Path(file_name): extract::Path<String>) -> api_error::Result<String> {
    let file_path = get_image_path(&file_name)?;
    Ok("null".into())
}

pub async fn put_image_block(extract::Path((file_name, offset)): extract::Path<(String, usize)>) -> api_error::Result<Json<ImageBlock>> {
    let file_path = get_image_path(&file_name)?;
    todo!()
}